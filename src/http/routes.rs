use axum::{routing::delete, routing::get, routing::patch, routing::post, Router};

use crate::AppState;
use crate::http::handlers;

pub fn health() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/metrics", get(handlers::metrics))
}

pub fn auth() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(handlers::login))
        .route("/auth/refresh", post(handlers::refresh_token))
        .route("/auth/revoke", post(handlers::revoke_token))
        .route("/auth/me", get(handlers::get_current_user))
}

pub fn users() -> Router<AppState> {
    Router::new()
        .route("/users", post(handlers::create_user))
        .route("/users/:id", get(handlers::get_user))
        .route("/users/:id", patch(handlers::update_profile))
        .route("/users/:id/posts", get(handlers::list_user_posts))
        .route("/users/:id/follow", post(handlers::follow_user))
        .route("/users/:id/unfollow", post(handlers::unfollow_user))
        .route("/users/:id/block", post(handlers::block_user))
        .route("/users/:id/unblock", post(handlers::unblock_user))
        .route("/users/:id/followers", get(handlers::list_followers))
        .route("/users/:id/following", get(handlers::list_following))
        .route("/users/:id/relationship", get(handlers::relationship_status))
}

pub fn posts() -> Router<AppState> {
    Router::new()
        .route("/posts", post(handlers::create_post))
        .route("/posts/:id", get(handlers::get_post))
        .route("/posts/:id", patch(handlers::update_post_caption))
        .route("/posts/:id", delete(handlers::delete_post))
        .route("/posts/:id/like", post(handlers::like_post))
        .route("/posts/:id/like", delete(handlers::unlike_post))
        .route("/posts/:id/likes", get(handlers::list_post_likes))
        .route("/posts/:id/comment", post(handlers::comment_post))
        .route("/posts/:id/comments", get(handlers::list_post_comments))
        .route(
            "/posts/:id/comments/:comment_id",
            delete(handlers::delete_comment),
        )
}

pub fn feed() -> Router<AppState> {
    Router::new()
        .route("/feed", get(handlers::home_feed))
        .route("/feed/refresh", post(handlers::refresh_feed))
}

pub fn media() -> Router<AppState> {
    Router::new()
        .route("/media/upload", post(handlers::create_upload))
        .route("/media/upload/:id/complete", post(handlers::complete_upload))
        .route("/media/:id", get(handlers::get_media))
        .route("/media/:id", delete(handlers::delete_media))
        .route(
            "/media/upload/:id/status",
            get(handlers::get_upload_status),
        )
}

pub fn notifications() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(handlers::list_notifications))
        .route(
            "/notifications/:id/read",
            post(handlers::mark_notification_read),
        )
}

pub fn moderation() -> Router<AppState> {
    Router::new()
        .route(
            "/moderation/users/:id/flag",
            post(handlers::flag_user),
        )
        .route(
            "/moderation/posts/:id/takedown",
            post(handlers::takedown_post),
        )
        .route(
            "/moderation/comments/:id/takedown",
            post(handlers::takedown_comment),
        )
        .route("/moderation/audit", get(handlers::list_moderation_audit))
}

pub fn search() -> Router<AppState> {
    Router::new()
        .route("/search/users", get(handlers::search_users))
        .route("/search/posts", get(handlers::search_posts))
}

pub fn stories() -> Router<AppState> {
    Router::new()
        .route("/stories", post(handlers::create_story))
        .route("/stories/:id", get(handlers::get_story))
        .route("/stories/:id", delete(handlers::delete_story))
        .route("/stories/:id/viewers", get(handlers::get_story_viewers))
        .route(
            "/stories/:id/reactions",
            post(handlers::add_story_reaction),
        )
        .route(
            "/stories/:id/reactions",
            get(handlers::list_story_reactions),
        )
        .route(
            "/stories/:id/reactions",
            delete(handlers::remove_story_reaction),
        )
        .route("/stories/:id/seen", post(handlers::mark_story_seen))
        .route("/stories/:id/metrics", get(handlers::get_story_metrics))
        .route(
            "/stories/:id/highlights",
            post(handlers::add_story_to_highlight),
        )
        .route("/users/:id/stories", get(handlers::get_user_stories))
        .route(
            "/users/:id/highlights",
            get(handlers::get_user_highlights),
        )
        .route("/feed/stories", get(handlers::get_stories_feed))
}

pub fn safety() -> Router<AppState> {
    Router::new()
        // Trust & Rate Limiting
        .route("/account/trust-score", get(handlers::get_trust_score))
        .route("/account/rate-limits", get(handlers::get_rate_limits))
        // Device Fingerprinting
        .route("/account/device/register", post(handlers::register_device_fingerprint))
        .route("/account/devices", get(handlers::list_user_devices))
        // Invite System
        .route("/invites", get(handlers::list_invites))
        .route("/invites", post(handlers::create_invite))
        .route("/invites/stats", get(handlers::get_invite_stats))
        .route("/invites/:code/revoke", post(handlers::revoke_invite))
}

