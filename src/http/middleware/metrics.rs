use axum::{
    body::Body,
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

pub async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let matched_path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let elapsed = start.elapsed();

    metrics::counter!(
        "http_requests_total",
        "method" => method.clone(),
        "path" => matched_path.clone(),
        "status" => status.clone()
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method,
        "path" => matched_path,
        "status" => status
    )
    .record(elapsed.as_secs_f64());

    response
}

