use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;

use crate::app::rate_limiter::RateLimiter;
use crate::app::trust::TrustService;
use crate::config::rate_limits::{RateWindow, TrustLevel};
use crate::http::{AppError, AuthUser};
use crate::AppState;

/// Rate limiting middleware for authenticated endpoints
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = request.uri().path();
    let method = request.method();

    // Determine action type from path and method
    let action = match (path, method.as_str()) {
        (p, "POST") if p.starts_with("/posts") && !p.contains("/comment") && !p.contains("/like") => {
            Some("post")
        }
        (p, "POST") if p.contains("/follow") && !p.contains("/unfollow") => Some("follow"),
        (p, "POST") if p.contains("/unfollow") => Some("unfollow"),
        (p, "POST") if p.contains("/like") => Some("like"),
        (p, "POST") if p.contains("/comment") => Some("comment"),
        ("/feed", "GET") | ("/feed/stories", "GET") => Some("feed"),
        ("/feed/refresh", "POST") => Some("feed"),
        (p, _) if p.starts_with("/notifications") => Some("notifications"),
        (p, _) if p.starts_with("/search/") => Some("search"),
        // Media: separate read vs write operations
        (p, "GET") if p.starts_with("/media") => Some("media_read"),
        (p, "POST") if p.starts_with("/media") => Some("media_upload"),
        (p, "DELETE") if p.starts_with("/media") => Some("media_upload"),
        (p, _) if p.starts_with("/moderation/") => Some("moderation"),
        _ => None,
    };

    if let Some(action) = action {
        if let Some(auth_user) = auth {
            // Get trust level
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

            // Check rate limit
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

            // Increment counter after successful check
            if let Err(err) = rate_limiter.increment(auth_user.user_id, action).await {
                tracing::warn!(error = ?err, "failed to increment rate limit counter");
            }

            // M5: Add rate limit headers to the response
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
    let method = request.method();

    // Determine if this is a sensitive unauthenticated endpoint
    let rate_limit_config = match (path, method.as_str()) {
        ("/auth/login", "POST") => Some(("login", 10, RateWindow::Hour)),
        ("/users", "POST") => Some(("signup", state.ip_signup_rate_limit, RateWindow::Day)),
        ("/health", "GET") => Some(("health", 60, RateWindow::Minute)),
        _ => None,
    };

    // Skip rate limiting if not a sensitive endpoint
    let (action, limit, window) = match rate_limit_config {
        Some(config) => config,
        None => return Ok(next.run(request).await),
    };

    let ip = addr.ip().to_string();
    let rate_limiter = RateLimiter::new(state.cache.clone());

    // Check IP-based rate limit
    let is_limited = rate_limiter
        .check_ip_rate_limit(&ip, action, limit, window)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to check IP rate limit");
            AppError::internal("failed to check rate limit")
        })?;

    if is_limited {
        tracing::warn!(
            ip = ip,
            action = action,
            "IP rate limit exceeded"
        );
        return Err(AppError::rate_limited_with_headers(
            "Too many attempts from your IP address. Please try again later.",
            limit,
            0,
        ));
    }

    // Increment counter
    if let Err(err) = rate_limiter.increment_ip(&ip, action, window).await {
        tracing::warn!(error = ?err, "failed to increment IP rate limit counter");
    }

    Ok(next.run(request).await)
}
