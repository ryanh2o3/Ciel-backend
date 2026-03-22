use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;

use crate::app::rate_limiter::RateLimiter;
use crate::app::trust::TrustService;
use crate::config::rate_limits::{RateWindow, TrustLevel};
use crate::http::{AppError, AuthUser};
use crate::http::middleware::request_context::RequestPeer;
use crate::AppState;

/// Strip `/v1` prefix so matchers work for nested API routes.
fn logical_path(path: &str) -> &str {
    path.strip_prefix("/v1").unwrap_or(path)
}

fn rate_limit_action(path: &str, method: &str) -> Option<&'static str> {
    let p = logical_path(path);
    match (p, method) {
        ("/posts", "POST") => Some("post"),
        (p, "POST") if p.starts_with("/posts/") && p.ends_with("/like") => Some("like"),
        (p, "POST") if p.starts_with("/posts/") && p.ends_with("/comment") => Some("comment"),
        (p, "POST") if p.contains("/follow") && !p.contains("/unfollow") => Some("follow"),
        (p, "POST") if p.contains("/unfollow") => Some("unfollow"),
        ("/feed", "GET") | ("/feed/stories", "GET") => Some("feed"),
        ("/feed/refresh", "POST") => Some("feed"),
        (p, _) if p.starts_with("/notifications") => Some("notifications"),
        (p, _) if p.starts_with("/search/") => Some("search"),
        (p, "GET") if p.starts_with("/media") => Some("media_read"),
        (p, "POST") if p.starts_with("/media") => Some("media_upload"),
        (p, "DELETE") if p.starts_with("/media") => Some("media_upload"),
        (p, _) if p.starts_with("/moderation/") => Some("moderation"),
        _ => None,
    }
}

fn ip_rate_limit_config(
    path: &str,
    method: &str,
    ip_signup_limit: u32,
) -> Option<(&'static str, u32, RateWindow)> {
    let p = logical_path(path);
    match (p, method) {
        ("/auth/login", "POST") => Some(("login", 10, RateWindow::Hour)),
        ("/users", "POST") => Some(("signup", ip_signup_limit, RateWindow::Day)),
        ("/health", "GET") => Some(("health", 60, RateWindow::Minute)),
        _ => None,
    }
}

/// Rate limiting middleware for authenticated endpoints
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = request.uri().path();
    let method = request.method().as_str();

    let action = rate_limit_action(path, method);

    if let Some(action) = action {
        if let Some(auth_user) = auth {
            let trust_service = TrustService::new(state.db.clone());
            let trust_score = trust_service
                .get_trust_score(auth_user.user_id)
                .await
                .map_err(|err| {
                    tracing::error!(error = ?err, "failed to check trust score");
                    AppError::internal("failed to check trust score")
                })?;

            let trust_level = trust_score
                .map(|s| s.trust_level)
                .unwrap_or(TrustLevel::New);

            let rate_limiter = RateLimiter::new(state.cache.clone());
            let info = rate_limiter
                .check_rate_limit(auth_user.user_id, action, trust_level)
                .await
                .map_err(|err| {
                    tracing::error!(error = ?err, "failed to check rate limit");
                    AppError::internal("failed to check rate limit")
                })?;

            if info.limited {
                return Err(AppError::rate_limited_with_headers(
                    &format!("Rate limit exceeded for action: {}. Please try again later.", action),
                    info.limit,
                    0,
                ));
            }

            if let Err(err) = rate_limiter.increment(auth_user.user_id, action).await {
                tracing::warn!(error = ?err, "failed to increment rate limit counter");
            }

            let mut response = next.run(request).await;
            let headers = response.headers_mut();
            if let Ok(v) = HeaderValue::from_str(&info.limit.to_string()) {
                headers.insert("X-RateLimit-Limit", v);
            }
            if let Ok(v) = HeaderValue::from_str(&info.remaining.saturating_sub(1).to_string()) {
                headers.insert("X-RateLimit-Remaining", v);
            }
            return Ok(response);
        }
    }

    Ok(next.run(request).await)
}

/// IP-based rate limiting for unauthenticated endpoints (like signup, login)
pub async fn ip_rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = request.uri().path();
    let method = request.method().as_str();

    let rate_limit_config =
        ip_rate_limit_config(path, method, state.ip_signup_rate_limit);

    let (action, limit, window) = match rate_limit_config {
        Some(config) => config,
        None => return Ok(next.run(request).await),
    };

    let ip = request
        .extensions()
        .get::<RequestPeer>()
        .map(|p| p.client_ip.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let rate_limiter = RateLimiter::new(state.cache.clone());

    let is_limited = rate_limiter
        .check_ip_rate_limit(&ip, action, limit, window)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to check IP rate limit");
            AppError::internal("failed to check rate limit")
        })?;

    if is_limited {
        tracing::warn!(ip = ip, action = action, "IP rate limit exceeded");
        return Err(AppError::rate_limited_with_headers(
            "Too many attempts from your IP address. Please try again later.",
            limit,
            0,
        ));
    }

    if let Err(err) = rate_limiter.increment_ip(&ip, action, window).await {
        tracing::warn!(error = ?err, "failed to increment IP rate limit counter");
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::{ip_rate_limit_config, rate_limit_action};

    #[test]
    fn rate_limit_post_create_respects_v1_prefix() {
        assert_eq!(rate_limit_action("/v1/posts", "POST"), Some("post"));
    }

    #[test]
    fn rate_limit_like_under_v1() {
        assert_eq!(
            rate_limit_action("/v1/posts/550e8400-e29b-41d4-a716-446655440000/like", "POST"),
            Some("like")
        );
    }

    #[test]
    fn ip_limit_login_under_v1() {
        assert!(ip_rate_limit_config("/v1/auth/login", "POST", 3).is_some());
    }

    #[test]
    fn ip_limit_health_no_v1() {
        assert!(ip_rate_limit_config("/health", "GET", 3).is_some());
    }
}
