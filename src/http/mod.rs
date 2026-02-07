use axum::{
    middleware as axum_middleware,
    Router,
};
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

mod auth;
mod error;
mod handlers;
pub mod middleware;
mod routes;

pub use auth::{AdminToken, AuthUser};
pub use error::AppError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(routes::health())
        // Auth routes with IP rate limiting
        .merge(
            routes::auth()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::ip_rate_limit_middleware,
                ))
        )
        // User routes with rate limiting
        .merge(
            routes::users()
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
        )
        .with_state(state)
        // Global middleware layers (applied to all routes)
        // Security headers and HTTPS enforcement
        .layer(axum_middleware::from_fn(
            middleware::security::security_headers_middleware,
        ))
        // Response compression (gzip, brotli)
        .layer(CompressionLayer::new())
        // Request body size limit (10MB default, matches UPLOAD_MAX_BYTES)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
}
