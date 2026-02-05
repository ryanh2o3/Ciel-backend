use axum::{
    middleware as axum_middleware,
    Router,
};

use crate::AppState;

mod auth;
mod error;
mod handlers;
pub mod middleware;
mod routes;

pub use auth::AuthUser;
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
        )
        // Posts, social, engagement routes with rate limiting
        .merge(
            routes::posts()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
        )
        .merge(routes::feed())
        .merge(routes::media())
        .merge(routes::notifications())
        .merge(routes::moderation())
        .merge(routes::search())
        // Stories with rate limiting
        .merge(
            routes::stories()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
        )
        // Add safety routes
        .merge(routes::safety())
        .with_state(state)
}

