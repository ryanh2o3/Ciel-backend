use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app::auth::AuthService;
use crate::app::engagement::EngagementService;
use crate::app::feed::FeedService;
use crate::app::media::{MediaService, UploadIntent, UploadStatus};
use crate::app::moderation::ModerationService;
use crate::app::notifications::NotificationService;
use crate::app::posts::PostService;
use crate::app::search::SearchService;
use crate::app::social::SocialService;
use crate::app::users::UserService;
use crate::http::{AdminToken, AppError, AuthUser};
use crate::AppState;

// Input validation constants
const MAX_HANDLE_LEN: usize = 30;
const MIN_HANDLE_LEN: usize = 3;
const MAX_DISPLAY_NAME_LEN: usize = 50;
const MAX_BIO_LEN: usize = 500;
const MAX_CAPTION_LEN: usize = 2200;
const MAX_PASSWORD_LEN: usize = 128;

/// Validate handle format: alphanumeric and underscores only, 3-30 characters
fn validate_handle(handle: &str) -> Result<(), AppError> {
    let handle = handle.trim();
    if handle.len() < MIN_HANDLE_LEN {
        return Err(AppError::bad_request("handle must be at least 3 characters"));
    }
    if handle.len() > MAX_HANDLE_LEN {
        return Err(AppError::bad_request("handle must be at most 30 characters"));
    }
    if !handle.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::bad_request("handle can only contain letters, numbers, and underscores"));
    }
    Ok(())
}

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    status: &'static str,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Serialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

fn parse_cursor(cursor: Option<String>) -> Result<Option<(OffsetDateTime, Uuid)>, AppError> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };

    let mut parts = cursor.splitn(2, '/');
    let timestamp = parts
        .next()
        .ok_or_else(|| AppError::bad_request("invalid cursor"))?;
    let id = parts
        .next()
        .ok_or_else(|| AppError::bad_request("invalid cursor"))?;

    let timestamp = OffsetDateTime::parse(timestamp, &Rfc3339)
        .map_err(|_| AppError::bad_request("invalid cursor"))?;
    let id = Uuid::parse_str(id).map_err(|_| AppError::bad_request("invalid cursor"))?;

    Ok(Some((timestamp, id)))
}

fn encode_cursor(cursor: Option<(OffsetDateTime, Uuid)>) -> Option<String> {
    let (timestamp, id) = cursor?;
    let timestamp = timestamp.format(&Rfc3339).ok()?;
    Some(format!("{}/{}", timestamp, id))
}

pub(crate) async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db = state.db.ping().await.is_ok();
    let redis = state.cache.ping().await.is_ok();
    let status = if db && redis { "ok" } else { "degraded" };

    Json(HealthResponse { status })
}

pub async fn metrics() -> Result<StatusCode, AppError> {
    Err(AppError::not_implemented("metrics not yet available"))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "time::serde::rfc3339")]
    pub access_expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub refresh_expires_at: OffsetDateTime,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, AppError> {
    const MAX_PASSWORD_LEN: usize = 128;

    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::bad_request("email and password are required"));
    }
    if payload.password.len() > MAX_PASSWORD_LEN {
        return Err(AppError::bad_request("password must be at most 128 characters"));
    }

    let service = AuthService::new(
        state.db.clone(),
        state.paseto_access_key,
        state.paseto_refresh_key,
        state.access_ttl_minutes,
        state.refresh_ttl_days,
    );
    let tokens = service
        .login(&payload.email, &payload.password)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to login");
            AppError::internal("failed to login")
        })?;

    match tokens {
        Some(tokens) => Ok(Json(AuthTokenResponse {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            access_expires_at: tokens.access_expires_at,
            refresh_expires_at: tokens.refresh_expires_at,
        })),
        None => Err(AppError::unauthorized("invalid credentials")),
    }
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthTokenResponse>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::bad_request("refresh_token is required"));
    }

    let service = AuthService::new(
        state.db.clone(),
        state.paseto_access_key,
        state.paseto_refresh_key,
        state.access_ttl_minutes,
        state.refresh_ttl_days,
    );
    let tokens = service
        .refresh(&payload.refresh_token)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to refresh token");
            AppError::internal("failed to refresh token")
        })?;

    match tokens {
        Some(tokens) => Ok(Json(AuthTokenResponse {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            access_expires_at: tokens.access_expires_at,
            refresh_expires_at: tokens.refresh_expires_at,
        })),
        None => Err(AppError::unauthorized("invalid refresh token")),
    }
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub refresh_token: String,
}

pub async fn revoke_token(
    State(state): State<AppState>,
    Json(payload): Json<RevokeRequest>,
) -> Result<StatusCode, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::bad_request("refresh_token is required"));
    }

    let service = AuthService::new(
        state.db.clone(),
        state.paseto_access_key,
        state.paseto_refresh_key,
        state.access_ttl_minutes,
        state.refresh_ttl_days,
    );
    let revoked = service
        .revoke_refresh_token(&payload.refresh_token)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to revoke token");
            AppError::internal("failed to revoke token")
        })?;

    let _ = revoked;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_current_user(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::user::User>, AppError> {
    let service = AuthService::new(
        state.db.clone(),
        state.paseto_access_key,
        state.paseto_refresh_key,
        state.access_ttl_minutes,
        state.refresh_ttl_days,
    );
    let user = service
        .get_current_user(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to fetch current user");
            AppError::internal("failed to fetch current user")
        })?;

    match user {
        Some(mut user) => {
            let media_svc = MediaService::new(
                state.db.clone(),
                state.storage.clone(),
                state.queue.clone(),
                state.s3_public_endpoint.clone(),
            );
            media_svc.populate_user_avatar_url(&mut user).await;
            Ok(Json(user))
        }
        None => Err(AppError::not_found("user not found")),
    }
}

pub async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::user::PublicUser>, AppError> {
    let service = UserService::new(state.db.clone());
    let user = service.get_user(id).await.map_err(|err| {
        tracing::error!(error = ?err, user_id = %id, "failed to fetch user");
        AppError::internal("failed to fetch user")
    })?;

    match user {
        Some(mut user) => {
            let media_svc = MediaService::new(
                state.db.clone(),
                state.storage.clone(),
                state.queue.clone(),
                state.s3_public_endpoint.clone(),
            );
            media_svc.populate_user_avatar_url(&mut user).await;
            Ok(Json(user.into()))
        }
        None => Err(AppError::not_found("user not found")),
    }
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub handle: String,
    pub email: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_key: Option<String>,
    pub password: String,
    pub invite_code: String,
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<crate::domain::user::User>, AppError> {
    // Validate handle format
    validate_handle(&payload.handle)?;
    
    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("email cannot be empty"));
    }
    if payload.display_name.trim().is_empty() {
        return Err(AppError::bad_request("display_name cannot be empty"));
    }
    if payload.display_name.len() > MAX_DISPLAY_NAME_LEN {
        return Err(AppError::bad_request("display_name must be at most 50 characters"));
    }
    if let Some(ref bio) = payload.bio {
        if bio.len() > MAX_BIO_LEN {
            return Err(AppError::bad_request("bio must be at most 500 characters"));
        }
    }
    if payload.password.trim().len() < 8 {
        return Err(AppError::bad_request("password must be at least 8 characters"));
    }
    if payload.password.len() > MAX_PASSWORD_LEN {
        return Err(AppError::bad_request("password must be at most 128 characters"));
    }
    if payload.invite_code.trim().is_empty() {
        return Err(AppError::bad_request("invite_code is required"));
    }

    let service = AuthService::new(
        state.db.clone(),
        state.paseto_access_key,
        state.paseto_refresh_key,
        state.access_ttl_minutes,
        state.refresh_ttl_days,
    );
    let user = service
        .signup(
            payload.handle,
            payload.email,
            payload.display_name,
            payload.bio,
            payload.avatar_key,
            payload.password,
            payload.invite_code,
        )
        .await
        .map_err(|err| {
            if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
                if let Some(db_err) = sqlx_err.as_database_error() {
                    if let Some(code) = db_err.code() {
                        if code == "23505" {
                            let constraint = db_err.constraint().unwrap_or_default();
                            if constraint.contains("users_handle_key") {
                                return AppError::conflict("Handle already taken");
                            }
                            if constraint.contains("users_email_key") {
                                return AppError::conflict("Email already taken");
                            }
                        }
                    }
                }
            }
            let message = err.to_string();
            if message.contains("invite code") || message.contains("Invite code") {
                return AppError::bad_request(message);
            }
            tracing::error!(error = ?err, "failed to create user");
            AppError::internal("failed to create user")
        })?;

    // Populate avatar URL if avatar_key exists
    let mut user = user;
    let media_svc = MediaService::new(
        state.db.clone(),
        state.storage.clone(),
        state.queue.clone(),
        state.s3_public_endpoint.clone(),
    );
    media_svc.populate_user_avatar_url(&mut user).await;

    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_key: Option<String>,
}

pub async fn update_profile(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<crate::domain::user::User>, AppError> {
    if auth.user_id != id {
        return Err(AppError::forbidden("cannot update other users"));
    }

    if let Some(display_name) = &payload.display_name {
        if display_name.trim().is_empty() {
            return Err(AppError::bad_request("display_name cannot be empty"));
        }
        if display_name.len() > MAX_DISPLAY_NAME_LEN {
            return Err(AppError::bad_request("display_name must be at most 50 characters"));
        }
    }
    if let Some(ref bio) = payload.bio {
        if bio.len() > MAX_BIO_LEN {
            return Err(AppError::bad_request("bio must be at most 500 characters"));
        }
    }

    let service = UserService::new(state.db.clone());
    let user = service
        .update_profile(id, payload.display_name, payload.bio, payload.avatar_key)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to update profile");
            AppError::internal("failed to update profile")
        })?;

    match user {
        Some(mut user) => {
            let media_svc = MediaService::new(
                state.db.clone(),
                state.storage.clone(),
                state.queue.clone(),
                state.s3_public_endpoint.clone(),
            );
            media_svc.populate_user_avatar_url(&mut user).await;
            Ok(Json(user))
        }
        None => Err(AppError::not_found("user not found")),
    }
}

/// Delete user account and all associated data (GDPR/CCPA compliance)
pub async fn delete_account(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = UserService::new(state.db.clone());
    let deleted = service
        .delete_account(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to delete account");
            AppError::internal("failed to delete account")
        })?;

    if deleted {
        tracing::info!(user_id = %auth.user_id, "account deleted");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("user not found"))
    }
}

pub async fn list_user_posts(
    Path(id): Path<Uuid>,
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::post::Post>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;
    let viewer_id = auth.map(|user| user.user_id);

    let service = PostService::new(state.db.clone());
    let mut posts = service
        .list_by_user(id, viewer_id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to list user posts");
            AppError::internal("failed to list user posts")
        })?;

    let next_cursor = if posts.len() > limit as usize {
        let last = posts.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: posts,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn follow_user(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<FollowResponse>, AppError> {
    if auth.user_id == id {
        return Err(AppError::bad_request("cannot follow yourself"));
    }

    let service = SocialService::new(state.db.clone());
    let followed = service.follow(auth.user_id, id).await.map_err(|err| {
        if err.to_string().contains("follower limit") {
            return AppError::forbidden("user has reached the follower limit");
        }
        tracing::error!(error = ?err, follower_id = %auth.user_id, followee_id = %id, "failed to follow user");
        AppError::internal("failed to follow user")
    })?;

    Ok(Json(FollowResponse { followed }))
}

#[derive(Serialize)]
pub struct FollowResponse {
    pub followed: bool,
}

pub async fn unfollow_user(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UnfollowResponse>, AppError> {
    if auth.user_id == id {
        return Err(AppError::bad_request("cannot unfollow yourself"));
    }

    let service = SocialService::new(state.db.clone());
    let unfollowed = service.unfollow(auth.user_id, id).await.map_err(|err| {
        tracing::error!(error = ?err, follower_id = %auth.user_id, followee_id = %id, "failed to unfollow user");
        AppError::internal("failed to unfollow user")
    })?;

    Ok(Json(UnfollowResponse { unfollowed }))
}

#[derive(Serialize)]
pub struct UnfollowResponse {
    pub unfollowed: bool,
}

pub async fn block_user(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<BlockResponse>, AppError> {
    if auth.user_id == id {
        return Err(AppError::bad_request("cannot block yourself"));
    }

    let service = SocialService::new(state.db.clone());
    let blocked = service.block(auth.user_id, id).await.map_err(|err| {
        tracing::error!(error = ?err, blocker_id = %auth.user_id, blocked_id = %id, "failed to block user");
        AppError::internal("failed to block user")
    })?;

    Ok(Json(BlockResponse { blocked }))
}

#[derive(Serialize)]
pub struct BlockResponse {
    pub blocked: bool,
}

pub async fn unblock_user(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UnblockResponse>, AppError> {
    if auth.user_id == id {
        return Err(AppError::bad_request("cannot unblock yourself"));
    }

    let service = SocialService::new(state.db.clone());
    let unblocked = service.unblock(auth.user_id, id).await.map_err(|err| {
        tracing::error!(error = ?err, blocker_id = %auth.user_id, blocked_id = %id, "failed to unblock user");
        AppError::internal("failed to unblock user")
    })?;

    Ok(Json(UnblockResponse { unblocked }))
}

#[derive(Serialize)]
pub struct UnblockResponse {
    pub unblocked: bool,
}

#[derive(Serialize)]
pub struct SocialUserItem {
    pub user: crate::domain::user::PublicUser,
    #[serde(with = "time::serde::rfc3339")]
    pub followed_at: OffsetDateTime,
}

pub async fn list_followers(
    Path(id): Path<Uuid>,
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<SocialUserItem>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = SocialService::new(state.db.clone());
    let mut followers = service
        .list_followers(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to list followers");
            AppError::internal("failed to list followers")
        })?;

    let next_cursor = if followers.len() > limit as usize {
        let last = followers.pop().expect("checked len");
        Some((last.followed_at, last.user.id))
    } else {
        None
    };

    let items = followers
        .into_iter()
        .map(|edge| SocialUserItem {
            user: edge.user.into(),
            followed_at: edge.followed_at,
        })
        .collect();

    Ok(Json(ListResponse {
        items,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn list_following(
    Path(id): Path<Uuid>,
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<SocialUserItem>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = SocialService::new(state.db.clone());
    let mut following = service
        .list_following(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to list following");
            AppError::internal("failed to list following")
        })?;

    let next_cursor = if following.len() > limit as usize {
        let last = following.pop().expect("checked len");
        Some((last.followed_at, last.user.id))
    } else {
        None
    };

    let items = following
        .into_iter()
        .map(|edge| SocialUserItem {
            user: edge.user.into(),
            followed_at: edge.followed_at,
        })
        .collect();

    Ok(Json(ListResponse {
        items,
        next_cursor: encode_cursor(next_cursor),
    }))
}

#[derive(Serialize)]
pub struct RelationshipResponse {
    pub is_following: bool,
    pub is_followed_by: bool,
    pub is_blocking: bool,
    pub is_blocked_by: bool,
}

pub async fn relationship_status(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<RelationshipResponse>, AppError> {
    if auth.user_id == id {
        return Ok(Json(RelationshipResponse {
            is_following: false,
            is_followed_by: false,
            is_blocking: false,
            is_blocked_by: false,
        }));
    }

    let service = SocialService::new(state.db.clone());
    let status = service
        .relationship_status(auth.user_id, id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, viewer_id = %auth.user_id, other_id = %id, "failed to fetch relationship status");
            AppError::internal("failed to fetch relationship status")
        })?;

    Ok(Json(RelationshipResponse {
        is_following: status.is_following,
        is_followed_by: status.is_followed_by,
        is_blocking: status.is_blocking,
        is_blocked_by: status.is_blocked_by,
    }))
}

#[derive(Deserialize)]
pub struct CreatePostRequest {
    pub media_id: Uuid,
    pub caption: Option<String>,
}

pub async fn create_post(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<Json<crate::domain::post::Post>, AppError> {
    if let Some(ref caption) = payload.caption {
        if caption.len() > MAX_CAPTION_LEN {
            return Err(AppError::bad_request("caption must be at most 2200 characters"));
        }
    }
    
    let service = PostService::new(state.db.clone());
    let post = service
        .create_post(auth.user_id, payload.media_id, payload.caption)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, owner_id = %auth.user_id, "failed to create post");
            AppError::internal("failed to create post")
        })?;

    Ok(Json(post))
}

pub async fn get_post(
    Path(id): Path<Uuid>,
    auth: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::post::Post>, AppError> {
    let viewer_id = auth.map(|user| user.user_id);
    let service = PostService::new(state.db.clone());
    let post = service.get_post(id, viewer_id).await.map_err(|err| {
        tracing::error!(error = ?err, post_id = %id, "failed to fetch post");
        AppError::internal("failed to fetch post")
    })?;

    match post {
        Some(post) => Ok(Json(post)),
        None => Err(AppError::not_found("post not found")),
    }
}

#[derive(Deserialize)]
pub struct UpdateCaptionRequest {
    pub caption: Option<String>,
}

pub async fn update_post_caption(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateCaptionRequest>,
) -> Result<Json<crate::domain::post::Post>, AppError> {
    if let Some(ref caption) = payload.caption {
        if caption.len() > MAX_CAPTION_LEN {
            return Err(AppError::bad_request("caption must be at most 2200 characters"));
        }
    }
    
    let service = PostService::new(state.db.clone());
    let post = service
        .update_caption(id, auth.user_id, payload.caption)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, post_id = %id, "failed to update post");
            AppError::internal("failed to update post")
        })?;

    match post {
        Some(post) => Ok(Json(post)),
        None => Err(AppError::not_found("post not found")),
    }
}

pub async fn delete_post(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = PostService::new(state.db.clone());
    let deleted = service.delete_post(id, auth.user_id).await.map_err(|err| {
        tracing::error!(error = ?err, post_id = %id, "failed to delete post");
        AppError::internal("failed to delete post")
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("post not found"))
    }
}

pub async fn like_post(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<LikeResponse>, AppError> {
    let service = EngagementService::new(state.db.clone());
    let like = service
        .like_post(auth.user_id, id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, post_id = %id, "failed to like post");
            AppError::internal("failed to like post")
        })?;

    Ok(Json(LikeResponse {
        created: like.is_some(),
    }))
}

pub async fn unlike_post(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = EngagementService::new(state.db.clone());
    let deleted = service
        .unlike_post(auth.user_id, id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, post_id = %id, "failed to unlike post");
            AppError::internal("failed to unlike post")
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("like not found"))
    }
}

pub async fn list_post_likes(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::engagement::Like>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = EngagementService::new(state.db.clone());
    let mut likes = service
        .list_likes(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, post_id = %id, "failed to list likes");
            AppError::internal("failed to list likes")
        })?;

    let next_cursor = if likes.len() > limit as usize {
        let last = likes.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: likes,
        next_cursor: encode_cursor(next_cursor),
    }))
}

#[derive(Serialize)]
pub struct LikeResponse {
    pub created: bool,
}

#[derive(Deserialize)]
pub struct CommentRequest {
    pub body: String,
}

pub async fn comment_post(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CommentRequest>,
) -> Result<Json<crate::domain::engagement::Comment>, AppError> {
    const MAX_COMMENT_LEN: usize = 1000;

    if payload.body.trim().is_empty() {
        return Err(AppError::bad_request("comment body cannot be empty"));
    }
    if payload.body.chars().count() > MAX_COMMENT_LEN {
        return Err(AppError::bad_request("comment body exceeds 1000 characters"));
    }

    let service = EngagementService::new(state.db.clone());
    let comment = service
        .comment_post(auth.user_id, id, payload.body)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, post_id = %id, "failed to comment");
            AppError::internal("failed to comment")
        })?;

    Ok(Json(comment))
}

pub async fn list_post_comments(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::engagement::Comment>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = EngagementService::new(state.db.clone());
    let mut comments = service
        .list_comments(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, post_id = %id, "failed to list comments");
            AppError::internal("failed to list comments")
        })?;

    let next_cursor = if comments.len() > limit as usize {
        let last = comments.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: comments,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn delete_comment(
    Path((post_id, comment_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = EngagementService::new(state.db.clone());
    let deleted = service
        .delete_comment(comment_id, post_id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, comment_id = %comment_id, user_id = %auth.user_id, "failed to delete comment");
            AppError::internal("failed to delete comment")
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("comment not found"))
    }
}

pub async fn home_feed(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::post::Post>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = FeedService::new(state.db.clone(), state.cache.clone());
    let (posts, next_cursor) = service
        .get_home_feed(auth.user_id, cursor, limit)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to fetch home feed");
            AppError::internal("failed to fetch home feed")
        })?;

    Ok(Json(ListResponse {
        items: posts,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn refresh_feed(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = FeedService::new(state.db.clone(), state.cache.clone());
    service
        .refresh_home_feed(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to refresh feed");
            AppError::internal("failed to refresh feed")
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct UploadRequest {
    pub content_type: String,
    pub bytes: i64,
}

pub async fn create_upload(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest>,
) -> Result<Json<UploadIntent>, AppError> {
    if payload.bytes <= 0 {
        return Err(AppError::bad_request("bytes must be greater than 0"));
    }
    if payload.bytes > state.upload_max_bytes {
        return Err(AppError::bad_request("upload exceeds max size"));
    }

    let service = crate::app::media::MediaService::new(
        state.db.clone(),
        state.storage.clone(),
        state.queue.clone(),
        state.s3_public_endpoint.clone(),
    );

    let intent = service
        .create_upload(
            auth.user_id,
            payload.content_type,
            payload.bytes,
            state.upload_url_ttl_seconds,
        )
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to create upload");
            AppError::bad_request("invalid upload request")
        })?;

    Ok(Json(intent))
}

pub async fn complete_upload(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = crate::app::media::MediaService::new(
        state.db.clone(),
        state.storage.clone(),
        state.queue.clone(),
        state.s3_public_endpoint.clone(),
    );

    let queued = service
        .complete_upload(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, upload_id = %id, user_id = %auth.user_id, "failed to complete upload");
            AppError::internal("failed to complete upload")
        })?;

    if queued {
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(AppError::not_found("upload not found"))
    }
}

pub async fn get_media(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::media::Media>, AppError> {
    let service = MediaService::new(state.db.clone(), state.storage.clone(), state.queue.clone(), state.s3_public_endpoint.clone());
    let media = service.get_media_for_user(id, auth.user_id).await.map_err(|err| {
        tracing::error!(error = ?err, media_id = %id, "failed to fetch media");
        AppError::internal("failed to fetch media")
    })?;

    match media {
        Some(media) => Ok(Json(media)),
        None => Err(AppError::not_found("media not found")),
    }
}

pub async fn get_upload_status(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<UploadStatus>, AppError> {
    let service = MediaService::new(state.db.clone(), state.storage.clone(), state.queue.clone(), state.s3_public_endpoint.clone());
    let status = service
        .get_upload_status(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, upload_id = %id, user_id = %auth.user_id, "failed to fetch upload status");
            AppError::internal("failed to fetch upload status")
        })?;

    match status {
        Some(status) => Ok(Json(status)),
        None => Err(AppError::not_found("upload not found")),
    }
}

pub async fn delete_media(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = MediaService::new(state.db.clone(), state.storage.clone(), state.queue.clone(), state.s3_public_endpoint.clone());
    let deleted = service
        .delete_media(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, media_id = %id, user_id = %auth.user_id, "failed to delete media");
            AppError::internal("failed to delete media")
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("media not found"))
    }
}

pub async fn list_notifications(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::notification::Notification>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = NotificationService::new(state.db.clone());
    let mut notifications = service
        .list(auth.user_id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to list notifications");
            AppError::internal("failed to list notifications")
        })?;

    let next_cursor = if notifications.len() > limit as usize {
        let last = notifications.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: notifications,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn mark_notification_read(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = NotificationService::new(state.db.clone());
    let updated = service
        .mark_read(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, notification_id = %id, user_id = %auth.user_id, "failed to mark notification read");
            AppError::internal("failed to mark notification read")
        })?;

    if updated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("notification not found"))
    }
}

#[derive(Deserialize)]
pub struct ModerationRequest {
    pub reason: Option<String>,
}

pub async fn flag_user(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<ModerationRequest>,
) -> Result<Json<crate::domain::moderation::UserFlag>, AppError> {
    let service = ModerationService::new(state.db.clone());
    let flag = service
        .flag_user(auth.user_id, id, payload.reason)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, reporter_id = %auth.user_id, target_id = %id, "failed to flag user");
            AppError::internal("failed to flag user")
        })?;

    Ok(Json(flag))
}

pub async fn takedown_post(
    auth: AuthUser,
    _admin: AdminToken,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<ModerationRequest>,
) -> Result<StatusCode, AppError> {
    let service = ModerationService::new(state.db.clone());
    let removed = service
        .takedown_post(auth.user_id, id, payload.reason)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, actor_id = %auth.user_id, post_id = %id, "failed to takedown post");
            AppError::internal("failed to takedown post")
        })?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("post not found"))
    }
}

pub async fn takedown_comment(
    auth: AuthUser,
    _admin: AdminToken,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<ModerationRequest>,
) -> Result<StatusCode, AppError> {
    let service = ModerationService::new(state.db.clone());
    let removed = service
        .takedown_comment(auth.user_id, id, payload.reason)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, actor_id = %auth.user_id, comment_id = %id, "failed to takedown comment");
            AppError::internal("failed to takedown comment")
        })?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("comment not found"))
    }
}

pub async fn list_moderation_audit(
    _auth: AuthUser,
    _admin: AdminToken,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::moderation::ModerationAction>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = ModerationService::new(state.db.clone());
    let mut actions = service
        .list_audit(cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to list moderation audit");
            AppError::internal("failed to list moderation audit")
        })?;

    let next_cursor = if actions.len() > limit as usize {
        let last = actions.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: actions,
        next_cursor: encode_cursor(next_cursor),
    }))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn search_users(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ListResponse<crate::domain::user::PublicUser>>, AppError> {
    let term = query.q.trim();
    if term.len() < 2 {
        return Err(AppError::bad_request("q must be at least 2 characters"));
    }
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = SearchService::new(state.db.clone());
    let mut users = service
        .search_users(term, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to search users");
            AppError::internal("failed to search users")
        })?;

    let next_cursor = if users.len() > limit as usize {
        let last = users.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    let items = users.into_iter().map(crate::domain::user::PublicUser::from).collect();

    Ok(Json(ListResponse {
        items,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn search_posts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ListResponse<crate::domain::post::Post>>, AppError> {
    let term = query.q.trim();
    if term.len() < 2 {
        return Err(AppError::bad_request("q must be at least 2 characters"));
    }
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service = SearchService::new(state.db.clone());
    let mut posts = service
        .search_posts(term, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to search posts");
            AppError::internal("failed to search posts")
        })?;

    let next_cursor = if posts.len() > limit as usize {
        let last = posts.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: posts,
        next_cursor: encode_cursor(next_cursor),
    }))
}


// ============================================================================
// Safety & Anti-Abuse Handlers
// ============================================================================

// Trust Score Handlers

#[derive(Serialize)]
pub struct TrustScoreResponse {
    pub user_id: String,
    pub trust_level: i32,
    pub trust_level_name: String,
    pub trust_points: i32,
    pub account_age_days: i32,
    pub posts_count: i32,
    pub followers_count: i32,
    pub strikes: i32,
    pub is_banned: bool,
}

pub async fn get_trust_score(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<TrustScoreResponse>, AppError> {
    let trust_service = crate::app::trust::TrustService::new(state.db.clone());
    let score = trust_service
        .get_trust_score(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to fetch trust score");
            AppError::internal("failed to fetch trust score")
        })?
        .ok_or_else(|| AppError::not_found("trust score not found"))?;

    let trust_level_name = match score.trust_level {
        crate::config::rate_limits::TrustLevel::New => "New",
        crate::config::rate_limits::TrustLevel::Basic => "Basic",
        crate::config::rate_limits::TrustLevel::Trusted => "Trusted",
        crate::config::rate_limits::TrustLevel::Verified => "Verified",
    };

    Ok(Json(TrustScoreResponse {
        user_id: score.user_id.to_string(),
        trust_level: score.trust_level as i32,
        trust_level_name: trust_level_name.to_string(),
        trust_points: score.trust_points,
        account_age_days: score.account_age_days,
        posts_count: score.posts_count,
        followers_count: score.followers_count,
        strikes: score.strikes,
        is_banned: score.banned_until.map(|until| until > time::OffsetDateTime::now_utc()).unwrap_or(false),
    }))
}

// Rate Limit Handlers

#[derive(Serialize)]
pub struct RateLimitsResponse {
    pub trust_level: String,
    pub posts_per_hour: u32,
    pub posts_per_day: u32,
    pub follows_per_hour: u32,
    pub follows_per_day: u32,
    pub likes_per_hour: u32,
    pub comments_per_hour: u32,
    pub remaining: RemainingQuotas,
}

#[derive(Serialize)]
pub struct RemainingQuotas {
    pub posts: u32,
    pub follows: u32,
    pub likes: u32,
    pub comments: u32,
}

pub async fn get_rate_limits(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<RateLimitsResponse>, AppError> {
    let trust_service = crate::app::trust::TrustService::new(state.db.clone());
    let score = trust_service
        .get_trust_score(auth.user_id)
        .await
        .map_err(|_| AppError::internal("failed to fetch trust score"))?
        .ok_or_else(|| AppError::not_found("trust score not found"))?;

    let trust_level = score.trust_level;
    let limits = crate::config::rate_limits::RateLimits::for_trust_level(trust_level);

    let rate_limiter = crate::app::rate_limiter::RateLimiter::new(state.cache.clone());

    let remaining_posts = rate_limiter
        .get_remaining(auth.user_id, "post", trust_level)
        .await
        .unwrap_or(0);
    let remaining_follows = rate_limiter
        .get_remaining(auth.user_id, "follow", trust_level)
        .await
        .unwrap_or(0);
    let remaining_likes = rate_limiter
        .get_remaining(auth.user_id, "like", trust_level)
        .await
        .unwrap_or(0);
    let remaining_comments = rate_limiter
        .get_remaining(auth.user_id, "comment", trust_level)
        .await
        .unwrap_or(0);

    Ok(Json(RateLimitsResponse {
        trust_level: format!("{:?}", trust_level),
        posts_per_hour: limits.posts_per_hour,
        posts_per_day: limits.posts_per_day,
        follows_per_hour: limits.follows_per_hour,
        follows_per_day: limits.follows_per_day,
        likes_per_hour: limits.likes_per_hour,
        comments_per_hour: limits.comments_per_hour,
        remaining: RemainingQuotas {
            posts: remaining_posts,
            follows: remaining_follows,
            likes: remaining_likes,
            comments: remaining_comments,
        },
    }))
}

// Device Fingerprint Handlers

#[derive(Deserialize)]
pub struct RegisterFingerprintRequest {
    pub fingerprint: String,
}

pub async fn register_device_fingerprint(
    auth: Option<AuthUser>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RegisterFingerprintRequest>,
) -> Result<StatusCode, AppError> {
    let service = crate::app::fingerprint::FingerprintService::new(state.db.clone());

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let fingerprint_hash = crate::app::fingerprint::FingerprintService::hash_fingerprint(&payload.fingerprint);

    // Check if device is blocked
    let (risk_score, is_blocked) = service
        .check_device_risk(&fingerprint_hash)
        .await
        .map_err(|_| AppError::internal("failed to check device risk"))?;

    if is_blocked {
        return Err(AppError::forbidden("This device has been blocked"));
    }

    if risk_score > 80 {
        let user_id_log = match &auth {
            Some(auth_user) => format!("user_id={}", auth_user.user_id),
            None => "unauthenticated".to_string(),
        };
        tracing::warn!(
            user_id = user_id_log,
            fingerprint_hash = &fingerprint_hash[..8],
            risk_score = risk_score,
            "High-risk device detected"
        );
    }

    // Register fingerprint - user_id is optional for unauthenticated registration
    let user_id = auth.map(|a| a.user_id);
    service
        .register_fingerprint(fingerprint_hash, user_id, user_agent)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to register fingerprint");
            AppError::internal("failed to register device")
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct DeviceResponse {
    pub fingerprint_hash: String,
    pub account_count: i32,
    pub risk_score: i32,
    pub is_blocked: bool,
}

pub async fn list_user_devices(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DeviceResponse>>, AppError> {
    let service = crate::app::fingerprint::FingerprintService::new(state.db.clone());

    let devices = service
        .get_user_devices(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to fetch devices");
            AppError::internal("failed to fetch devices")
        })?;

    let response = devices
        .into_iter()
        .map(|device| DeviceResponse {
            fingerprint_hash: device.fingerprint_hash[..16].to_string(), // Truncate for privacy
            account_count: device.account_count,
            risk_score: device.risk_score,
            is_blocked: device.is_blocked,
        })
        .collect();

    Ok(Json(response))
}

// Invite Handlers

pub async fn list_invites(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::app::invites::InviteCode>>, AppError> {
    let service = crate::app::invites::InviteService::new(state.db.clone());

    let invites = service
        .list_user_invites(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to list invites");
            AppError::internal("failed to list invites")
        })?;

    Ok(Json(invites))
}

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    #[serde(default = "default_days_valid")]
    pub days_valid: i64,
}

fn default_days_valid() -> i64 {
    7
}

pub async fn create_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateInviteRequest>,
) -> Result<Json<crate::app::invites::InviteCode>, AppError> {
    if payload.days_valid < 1 || payload.days_valid > 30 {
        return Err(AppError::bad_request("days_valid must be between 1 and 30"));
    }

    let service = crate::app::invites::InviteService::new(state.db.clone());

    let invite = service
        .create_invite(auth.user_id, payload.days_valid)
        .await
        .map_err(|err| {
            if err.to_string().contains("Maximum invite limit") {
                AppError::forbidden(&err.to_string())
            } else {
                tracing::error!(error = ?err, "failed to create invite");
                AppError::internal("failed to create invite")
            }
        })?;

    Ok(Json(invite))
}

pub async fn get_invite_stats(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<crate::app::invites::InviteStats>, AppError> {
    let service = crate::app::invites::InviteService::new(state.db.clone());

    let stats = service
        .get_invite_stats(auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to fetch invite stats");
            AppError::internal("failed to fetch invite stats")
        })?;

    Ok(Json(stats))
}

pub async fn revoke_invite(
    auth: AuthUser,
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = crate::app::invites::InviteService::new(state.db.clone());

    let revoked = service
        .revoke_invite(&code, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "failed to revoke invite");
            AppError::internal("failed to revoke invite")
        })?;

    if revoked {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("invite code not found or already used"))
    }
}

// ============================================================================
// Story Handlers
// ============================================================================

#[derive(Deserialize)]
pub struct CreateStoryRequest {
    pub media_id: Uuid,
    pub caption: Option<String>,
    pub visibility: crate::domain::story::StoryVisibility,
}

pub async fn create_story(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateStoryRequest>,
) -> Result<Json<crate::domain::story::Story>, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let story = service
        .create_story(auth.user_id, payload.media_id, payload.caption, payload.visibility)
        .await
        .map_err(|err| {
            if err.to_string().contains("media not found") {
                return AppError::not_found("media not found");
            }
            if err.to_string().contains("does not belong") {
                return AppError::forbidden("media does not belong to you");
            }
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to create story");
            AppError::internal("failed to create story")
        })?;

    Ok(Json(story))
}

pub async fn get_user_stories(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::story::Story>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let mut stories = service
        .get_user_stories(id, auth.user_id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to get user stories");
            AppError::internal("failed to get user stories")
        })?;

    let next_cursor = if stories.len() > limit as usize {
        let last = stories.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: stories,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn get_story(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::story::Story>, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let story = service
        .get_story(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, "failed to get story");
            AppError::internal("failed to get story")
        })?;

    match story {
        Some(story) => Ok(Json(story)),
        None => Err(AppError::not_found("story not found")),
    }
}

pub async fn delete_story(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let deleted = service
        .delete_story(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, "failed to delete story");
            AppError::internal("failed to delete story")
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("story not found"))
    }
}

pub async fn get_story_viewers(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::story::StoryView>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());

    let owner = service
        .get_story_owner(id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, "failed to check story ownership");
            AppError::internal("failed to get story viewers")
        })?;

    match owner {
        Some(owner_id) if owner_id == auth.user_id => {}
        Some(_) => return Err(AppError::forbidden("only the story owner can view viewers")),
        None => return Err(AppError::not_found("story not found")),
    }

    let mut viewers = service
        .list_viewers(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, "failed to list story viewers");
            AppError::internal("failed to list story viewers")
        })?;

    let next_cursor = if viewers.len() > limit as usize {
        let last = viewers.pop().expect("checked len");
        Some((last.viewed_at, last.viewer_id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: viewers,
        next_cursor: encode_cursor(next_cursor),
    }))
}

#[derive(Deserialize)]
pub struct AddReactionRequest {
    pub emoji: String,
}

pub async fn add_story_reaction(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<AddReactionRequest>,
) -> Result<Json<crate::domain::story::StoryReaction>, AppError> {
    let emoji = payload.emoji.trim();
    if emoji.is_empty() {
        return Err(AppError::bad_request("emoji cannot be empty"));
    }
    if emoji.chars().count() > 7 {
        return Err(AppError::bad_request("emoji must be at most 7 characters"));
    }

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let reaction = service
        .add_reaction(id, auth.user_id, emoji.to_string())
        .await
        .map_err(|err| {
            if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
                if let Some(db_err) = sqlx_err.as_database_error() {
                    if let Some(code) = db_err.code() {
                        if code == "23503" {
                            return AppError::not_found("story not found");
                        }
                    }
                }
            }
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to add reaction");
            AppError::internal("failed to add reaction")
        })?;

    Ok(Json(reaction))
}

pub async fn list_story_reactions(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::story::StoryReaction>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let story = service
        .get_story(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to check story visibility");
            AppError::internal("failed to list story reactions")
        })?;

    if story.is_none() {
        return Err(AppError::not_found("story not found"));
    }

    let mut reactions = service
        .list_reactions(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, "failed to list story reactions");
            AppError::internal("failed to list story reactions")
        })?;

    let next_cursor = if reactions.len() > limit as usize {
        let last = reactions.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: reactions,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn remove_story_reaction(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let removed = service
        .remove_reaction(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to remove reaction");
            AppError::internal("failed to remove reaction")
        })?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("reaction not found"))
    }
}

pub async fn mark_story_seen(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    service
        .mark_seen(id, auth.user_id)
        .await
        .map_err(|err| {
            if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
                if let Some(db_err) = sqlx_err.as_database_error() {
                    if let Some(code) = db_err.code() {
                        if code == "23503" {
                            return AppError::not_found("story not found");
                        }
                    }
                }
            }
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to mark story seen");
            AppError::internal("failed to mark story seen")
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_stories_feed(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::story::Story>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let mut stories = service
        .get_stories_feed(auth.user_id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %auth.user_id, "failed to get stories feed");
            AppError::internal("failed to get stories feed")
        })?;

    let next_cursor = if stories.len() > limit as usize {
        let last = stories.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: stories,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn get_story_metrics(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::story::StoryMetrics>, AppError> {
    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let metrics = service
        .get_metrics(id, auth.user_id)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to get story metrics");
            AppError::internal("failed to get story metrics")
        })?;

    match metrics {
        Some(metrics) => Ok(Json(metrics)),
        None => Err(AppError::not_found("story not found")),
    }
}

#[derive(Deserialize)]
pub struct AddToHighlightRequest {
    pub highlight_name: String,
}

pub async fn add_story_to_highlight(
    Path(id): Path<Uuid>,
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<AddToHighlightRequest>,
) -> Result<Json<crate::domain::story::StoryHighlight>, AppError> {
    if payload.highlight_name.trim().is_empty() {
        return Err(AppError::bad_request("highlight_name cannot be empty"));
    }

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let highlight = service
        .add_to_highlight(auth.user_id, id, payload.highlight_name)
        .await
        .map_err(|err| {
            if err.to_string().contains("story not found") {
                return AppError::not_found("story not found");
            }
            tracing::error!(error = ?err, story_id = %id, user_id = %auth.user_id, "failed to add to highlight");
            AppError::internal("failed to add to highlight")
        })?;

    Ok(Json(highlight))
}

pub async fn get_user_highlights(
    Path(id): Path<Uuid>,
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ListResponse<crate::domain::story::StoryHighlight>>, AppError> {
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(AppError::bad_request("limit must be between 1 and 200"));
    }
    let cursor = parse_cursor(query.cursor)?;

    let service =
        crate::app::stories::StoryService::new(state.db.clone(), state.cache.clone());
    let mut highlights = service
        .get_user_highlights(id, cursor, limit + 1)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, user_id = %id, "failed to get user highlights");
            AppError::internal("failed to get user highlights")
        })?;

    let next_cursor = if highlights.len() > limit as usize {
        let last = highlights.pop().expect("checked len");
        Some((last.created_at, last.id))
    } else {
        None
    };

    Ok(Json(ListResponse {
        items: highlights,
        next_cursor: encode_cursor(next_cursor),
    }))
}
