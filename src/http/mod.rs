use axum::{
    http::{self, Method},
    middleware as axum_middleware,
    Router,
};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

use crate::AppState;

mod auth;
mod error;
mod handlers;
mod logging;
pub mod middleware;
pub mod validation;
mod routes;

pub use auth::{AdminToken, AuthUser};
pub use error::AppError;
pub use logging::internal_err;
pub use logging::internal_err_user;

pub fn router(state: AppState) -> Router {
    let request_ctx_state = state.clone();

    // M8: Versioned API routes under /v1
    let v1_routes = Router::new()
        // Auth routes with IP rate limiting
        .merge(
            routes::auth()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::ip_rate_limit_middleware,
                ))
        )
        // User routes with rate limiting + IP rate limiting for signup
        .merge(
            routes::users()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::ip_rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        // Posts, social, engagement routes with rate limiting
        .merge(
            routes::posts()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        .merge(
            routes::feed()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        .merge(
            routes::media()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        .merge(
            routes::notifications()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        .merge(
            routes::moderation()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        .merge(
            routes::search()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        // Stories with rate limiting
        .merge(
            routes::stories()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        )
        // Add safety routes
        .merge(
            routes::safety()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::ban::ban_check_middleware,
                ))
        );

    Router::new()
        // Health and metrics at root (no version prefix), with IP rate limiting
        .merge(
            routes::health()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::ip_rate_limit_middleware,
                ))
        )
        // All API routes under /v1
        .nest("/v1", v1_routes)
        .with_state(state)
        // Global middleware layers (applied to all routes)
        // Metrics (Prometheus)
        .layer(axum_middleware::from_fn(middleware::metrics::metrics_middleware))
        // CORS — no web origins allowed (mobile-only API)
        .layer(
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    http::header::AUTHORIZATION,
                    http::header::CONTENT_TYPE,
                ])
        )
        // M3: Request ID
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        // Trusted-proxy-aware client IP + scheme (before security / IP rate limits)
        .layer(axum_middleware::from_fn_with_state(
            request_ctx_state,
            middleware::request_context::request_context_middleware,
        ))
        // Security headers and HTTPS enforcement
        .layer(axum_middleware::from_fn(
            middleware::security::security_headers_middleware,
        ))
        // Response compression (gzip, brotli)
        .layer(CompressionLayer::new())
        // Request body size limit (10MB default, matches UPLOAD_MAX_BYTES)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
}
