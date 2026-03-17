use axum::{
    body::Body,
    http::{header, header::HeaderName, Request, Response, StatusCode},
    middleware::Next,
};

const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";
const CSP_VALUE: &str = "default-src 'none'; frame-ancestors 'none'";
const CACHE_CONTROL_VALUE: &str = "no-store, no-cache, must-revalidate";

/// Security headers middleware that adds essential security headers to all responses
/// and enforces HTTPS in non-local environments
pub async fn security_headers_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Check for HTTPS enforcement in non-local environments
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let is_local = host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("0.0.0.0")
        || host.starts_with("[::1]");
    
    // Check X-Forwarded-Proto header for HTTPS (common with load balancers)
    let forwarded_proto = request
        .headers()
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("https"); // Default to https if not present (local dev)
    
    // Enforce HTTPS in non-local environments
    if !is_local && forwarded_proto != "https" {
        tracing::warn!(
            host = host,
            proto = forwarded_proto,
            "rejected non-HTTPS request in production"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Strict-Transport-Security: enforce HTTPS for 1 year, include subdomains
    // Only set on non-local to avoid issues with local development
    if !is_local {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            header::HeaderValue::from_static(HSTS_VALUE),
        );
    }

    // X-Content-Type-Options: prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options: prevent clickjacking
    headers.insert(
        header::X_FRAME_OPTIONS,
        header::HeaderValue::from_static("DENY"),
    );

    // X-XSS-Protection: legacy XSS protection for older browsers
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        header::HeaderValue::from_static("1; mode=block"),
    );

    // Content-Security-Policy: restrictive default for API responses
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        header::HeaderValue::from_static(CSP_VALUE),
    );

    // Referrer-Policy: don't leak referrer information
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        header::HeaderValue::from_static("no-referrer"),
    );

    // Cache-Control: prevent caching of API responses by default
    // Individual endpoints can override this if needed
    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(CACHE_CONTROL_VALUE),
        );
    }

    Ok(response)
}
