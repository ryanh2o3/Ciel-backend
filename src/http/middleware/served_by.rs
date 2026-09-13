use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

static SERVED_BY: HeaderName = HeaderName::from_static("x-ciel-served-by");

/// Adds `X-Ciel-Served-By: rust` (or `CIEL_SERVED_BY`) so opaque multi-impl
/// routing is observable without requiring clients to opt in.
pub async fn served_by_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let value = std::env::var("CIEL_SERVED_BY").unwrap_or_else(|_| "rust".to_string());
    if let Ok(hv) = HeaderValue::from_str(&value) {
        response.headers_mut().insert(SERVED_BY.clone(), hv);
    }
    response
}
