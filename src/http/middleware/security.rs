use axum::{
    body::Body,
    http::{header, header::HeaderName, Request, Response, StatusCode},
    middleware::Next,
};

use super::request_context::{RequestPeer, ResolvedScheme};

const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";
const CSP_VALUE: &str = "default-src 'none'; frame-ancestors 'none'";
const CACHE_CONTROL_VALUE: &str = "no-store, no-cache, must-revalidate";

/// Security headers middleware that adds essential security headers to all responses
/// and enforces HTTPS when we can infer the original scheme from trusted forwarded headers.
pub async fn security_headers_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let is_local = host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("0.0.0.0")
        || host.starts_with("[::1]");

    let scheme = request
        .extensions()
        .get::<RequestPeer>()
        .map(|p| p.scheme)
        .unwrap_or(ResolvedScheme::Unknown);

    // Never trust a default "https" when headers are missing (see request_context).
    // Unknown = direct client or no trusted proxy: assume TLS may terminate at the app.
    if !is_local {
        match scheme {
            ResolvedScheme::Https => {}
            ResolvedScheme::Unknown => {}
            ResolvedScheme::Http => {
                tracing::warn!(
                    host = host,
                    "rejected non-HTTPS request (forwarded scheme was http)"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    if !is_local {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            header::HeaderValue::from_static(HSTS_VALUE),
        );
    }

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );

    headers.insert(
        header::X_FRAME_OPTIONS,
        header::HeaderValue::from_static("DENY"),
    );

    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        header::HeaderValue::from_static("1; mode=block"),
    );

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        header::HeaderValue::from_static(CSP_VALUE),
    );

    headers.insert(
        HeaderName::from_static("referrer-policy"),
        header::HeaderValue::from_static("no-referrer"),
    );

    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(CACHE_CONTROL_VALUE),
        );
    }

    Ok(response)
}
