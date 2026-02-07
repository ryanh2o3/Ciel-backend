//! Post CRUD Tests
//!
//! Covers post creation, reading, updating, deleting, and listing.

mod common;

use axum::http::StatusCode;
use common::app;
use serde_json::json;
use uuid::Uuid;

// ===========================================================================
// Post Creation
// ===========================================================================

#[tokio::test]
async fn create_post_valid() {
    let app = app().await;
    let user = app.create_user("post_create").await;
    let media_id = app.create_media(user.id).await;

    let resp = app
        .post_json(
            "/posts",
            json!({ "media_id": media_id.to_string(), "caption": "My first post!" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["id"].is_string());
    assert_eq!(body["owner_id"].as_str().unwrap(), user.id.to_string());
    assert_eq!(body["media_id"].as_str().unwrap(), media_id.to_string());
    assert_eq!(body["caption"].as_str().unwrap(), "My first post!");
}

#[tokio::test]
async fn create_post_caption_too_long() {
    let app = app().await;
    let user = app.create_user("post_longcaption").await;
    let media_id = app.create_media(user.id).await;

    let resp = app
        .post_json(
            "/posts",
            json!({
                "media_id": media_id.to_string(),
                "caption": "a".repeat(2201)
            }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "caption must be at most 2200 characters"
    );
}

#[tokio::test]
async fn create_post_invalid_media() {
    let app = app().await;
    let user = app.create_user("post_badmedia").await;
    let fake_media_id = Uuid::new_v4();

    let resp = app
        .post_json(
            "/posts",
            json!({ "media_id": fake_media_id.to_string(), "caption": "test" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "invalid media_id");
}

#[tokio::test]
async fn get_post() {
    let app = app().await;
    let user = app.create_user("post_get").await;
    let (post_id, _) = app.create_post_for_user(user.id).await;

    // Public posts visible without auth
    let resp = app.get(&format!("/posts/{}", post_id), None).await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), post_id.to_string());
    assert_eq!(body["owner_id"].as_str().unwrap(), user.id.to_string());
}

#[tokio::test]
async fn get_nonexistent_post() {
    let app = app().await;

    let resp = app
        .get(&format!("/posts/{}", Uuid::new_v4()), None)
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    assert_eq!(resp.error_message(), "post not found");
}

#[tokio::test]
async fn update_post_caption() {
    let app = app().await;
    let user = app.create_user("post_update").await;
    let (post_id, _) = app.create_post_for_user(user.id).await;

    let resp = app
        .patch_json(
            &format!("/posts/{}", post_id),
            json!({ "caption": "Updated caption" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["caption"].as_str().unwrap(), "Updated caption");
}

#[tokio::test]
async fn update_post_wrong_user() {
    let app = app().await;
    let user_a = app.create_user("post_update_a").await;
    let user_b = app.create_user("post_update_b").await;
    let (post_id, _) = app.create_post_for_user(user_a.id).await;

    let resp = app
        .patch_json(
            &format!("/posts/{}", post_id),
            json!({ "caption": "Hacked caption" }),
            Some(&user_b.access_token),
        )
        .await;

    // Ownership enforced — returns 404 (not 403) to avoid leaking existence
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_post() {
    let app = app().await;
    let user = app.create_user("post_delete").await;
    let (post_id, _) = app.create_post_for_user(user.id).await;

    let resp = app
        .delete(&format!("/posts/{}", post_id), Some(&user.access_token))
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Verify post is gone
    let resp = app.get(&format!("/posts/{}", post_id), None).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_post_wrong_user() {
    let app = app().await;
    let user_a = app.create_user("post_delete_a").await;
    let user_b = app.create_user("post_delete_b").await;
    let (post_id, _) = app.create_post_for_user(user_a.id).await;

    let resp = app
        .delete(&format!("/posts/{}", post_id), Some(&user_b.access_token))
        .await;

    // Ownership enforced — returns 404
    assert_eq!(resp.status, StatusCode::NOT_FOUND);

    // Verify post still exists
    let resp = app.get(&format!("/posts/{}", post_id), None).await;
    assert_eq!(resp.status, StatusCode::OK);
}

#[tokio::test]
async fn list_user_posts() {
    let app = app().await;
    let user = app.create_user("post_list").await;

    // Create two posts
    app.create_post_for_user(user.id).await;
    app.create_post_for_user(user.id).await;

    let resp = app
        .get(&format!("/users/{}/posts?limit=10", user.id), None)
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
}

#[tokio::test]
async fn create_post_no_caption() {
    let app = app().await;
    let user = app.create_user("post_nocap").await;
    let media_id = app.create_media(user.id).await;

    let resp = app
        .post_json(
            "/posts",
            json!({ "media_id": media_id.to_string() }),
            Some(&user.access_token),
        )
        .await;

    // Caption is optional, so this should succeed
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["caption"].is_null());
}
