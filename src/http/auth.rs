use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::HeaderName;
use axum::http::request::Parts;
use subtle::ConstantTimeEq;

use crate::app::auth::AuthService;
use crate::http::AppError;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct AdminToken;

const ADMIN_TOKEN_HEADER: HeaderName = HeaderName::from_static("x-admin-token");

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // The extractor runs in several middleware layers plus the handler;
        // cache the validated identity in request extensions so the token is
        // only decrypted and checked against the database once per request.
        if let Some(cached) = parts.extensions.get::<AuthUser>() {
            return Ok(cached.clone());
        }

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("invalid Authorization header"))?;

        let app_state = state;

        let service = AuthService::new(
            app_state.db.clone(),
            app_state.paseto_access_key,
            app_state.paseto_refresh_key,
            app_state.access_ttl_minutes,
            app_state.refresh_ttl_days,
        );
        let session = service
            .authenticate_access_token(token)
            .await
            .map_err(|_| AppError::unauthorized("invalid or expired token"))?;

        let session = session.ok_or_else(|| AppError::unauthorized("invalid token"))?;
        let user = AuthUser {
            user_id: session.user_id,
        };
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminToken {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let expected = state
            .admin_token
            .as_ref()
            .ok_or_else(|| AppError::forbidden("admin token not configured"))?;

        let provided = parts
            .headers
            .get(ADMIN_TOKEN_HEADER)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::forbidden("missing admin token"))?;

        let is_valid: bool = provided.as_bytes().ct_eq(expected.as_bytes()).into();
        if !is_valid {
            return Err(AppError::forbidden("invalid admin token"));
        }

        Ok(AdminToken)
    }
}
