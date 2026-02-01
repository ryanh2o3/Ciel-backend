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
use crate::http::{AppError, AuthUser};
use crate::AppState;

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    status: &'static str,
    db: bool,
    redis: bool,
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

    Json(HealthResponse { status, db, redis })
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
    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::bad_request("email and password are required"));
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
            tracing::error!(error = ?err, email = %payload.email, "failed to login");
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

    if revoked {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
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
        Some(user) => Ok(Json(user)),
        None => Err(AppError::not_found("user not found")),
    }
}

pub async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::user::User>, AppError> {
    let service = UserService::new(state.db.clone());
    let user = service.get_user(id).await.map_err(|err| {
        tracing::error!(error = ?err, user_id = %id, "failed to fetch user");
        AppError::internal("failed to fetch user")
    })?;

    match user {
        Some(user) => Ok(Json(user)),
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
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<crate::domain::user::User>, AppError> {
    if payload.handle.trim().is_empty() {
        return Err(AppError::bad_request("handle cannot be empty"));
    }
    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("email cannot be empty"));
    }
    if payload.display_name.trim().is_empty() {
        return Err(AppError::bad_request("display_name cannot be empty"));
    }
    if payload.password.trim().len() < 8 {
        return Err(AppError::bad_request("password must be at least 8 characters"));
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
            tracing::error!(error = ?err, "failed to create user");
            AppError::bad_request("failed to create user")
        })?;

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
        return Err(AppError::unauthorized("cannot update other users"));
    }

    if let Some(display_name) = &payload.display_name {
        if display_name.trim().is_empty() {
            return Err(AppError::bad_request("display_name cannot be empty"));
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
        Some(user) => Ok(Json(user)),
        None => Err(AppError::not_found("user not found")),
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
    pub user: crate::domain::user::User,
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
            user: edge.user,
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
            user: edge.user,
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
    if payload.body.trim().is_empty() {
        return Err(AppError::bad_request("comment body cannot be empty"));
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
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<crate::domain::media::Media>, AppError> {
    let service = MediaService::new(state.db.clone(), state.storage.clone(), state.queue.clone(), state.s3_public_endpoint.clone());
    let media = service.get_media(id).await.map_err(|err| {
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
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ListResponse<crate::domain::user::User>>, AppError> {
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

    Ok(Json(ListResponse {
        items: users,
        next_cursor: encode_cursor(next_cursor),
    }))
}

pub async fn search_posts(
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

