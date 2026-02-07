//! Media Upload Tests
//!
//! Covers upload creation, validation, completion, and status checking.

mod common;

use axum::http::StatusCode;
use common::app;
use serde_json::json;
use uuid::Uuid;

// ===========================================================================
// Media Upload Flow
// ===========================================================================

#[tokio::test]
async fn create_upload_valid() {
    let app = app().await;
    let user = app.create_user("media_valid").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 1024 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["upload_id"].is_string());
    assert!(body["upload_url"].is_string());
    assert!(body["object_key"].is_string());
    assert!(body["expires_in_seconds"].is_number());
}

#[tokio::test]
async fn create_upload_zero_bytes() {
    let app = app().await;
    let user = app.create_user("media_zero").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 0 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "bytes must be greater than 0");
}

#[tokio::test]
async fn create_upload_negative_bytes() {
    let app = app().await;
    let user = app.create_user("media_neg").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": -1 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "bytes must be greater than 0");
}

#[tokio::test]
async fn create_upload_exceeds_max() {
    let app = app().await;
    let user = app.create_user("media_big").await;

    // Use a value larger than any reasonable max (default is typically 50MB)
    let huge_bytes = 500_000_000_i64;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": huge_bytes }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "upload exceeds max size");
}

#[tokio::test]
async fn create_upload_invalid_content_type() {
    let app = app().await;
    let user = app.create_user("media_badtype").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "text/html", "bytes": 1024 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "invalid upload request");
}

#[tokio::test]
async fn create_upload_png() {
    let app = app().await;
    let user = app.create_user("media_png").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/png", "bytes": 2048 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["object_key"].as_str().unwrap().ends_with(".png"));
}

#[tokio::test]
async fn create_upload_webp() {
    let app = app().await;
    let user = app.create_user("media_webp").await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/webp", "bytes": 512 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["object_key"].as_str().unwrap().ends_with(".webp"));
}

#[tokio::test]
async fn complete_upload_wrong_user() {
    let app = app().await;
    let user_a = app.create_user("media_complete_a").await;
    let user_b = app.create_user("media_complete_b").await;

    // A creates an upload
    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 1024 }),
            Some(&user_a.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let upload_id = resp.json()["upload_id"].as_str().unwrap().to_string();

    // B tries to complete A's upload
    let resp = app
        .post_json(
            &format!("/media/upload/{}/complete", upload_id),
            json!({}),
            Some(&user_b.access_token),
        )
        .await;

    // Should not succeed — ownership check (returns 404 since row not found for user B)
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_upload_status() {
    let app = app().await;
    let user = app.create_user("media_status").await;

    // Create an upload
    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 1024 }),
            Some(&user.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let upload_id = resp.json()["upload_id"].as_str().unwrap().to_string();

    // Check status
    let resp = app
        .get(
            &format!("/media/upload/{}/status", upload_id),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[tokio::test]
async fn get_upload_status_wrong_user() {
    let app = app().await;
    let user_a = app.create_user("media_status_a").await;
    let user_b = app.create_user("media_status_b").await;

    // A creates an upload
    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 1024 }),
            Some(&user_a.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let upload_id = resp.json()["upload_id"].as_str().unwrap().to_string();

    // B tries to check A's upload status
    let resp = app
        .get(
            &format!("/media/upload/{}/status", upload_id),
            Some(&user_b.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_upload_no_auth() {
    let app = app().await;

    let resp = app
        .post_json(
            "/media/upload",
            json!({ "content_type": "image/jpeg", "bytes": 1024 }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_media_wrong_user() {
    let app = app().await;
    let user_a = app.create_user("media_del_a").await;
    let user_b = app.create_user("media_del_b").await;
    let media_id = app.create_media(user_a.id).await;

    let resp = app
        .delete(
            &format!("/media/{}", media_id),
            Some(&user_b.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_nonexistent_upload_status() {
    let app = app().await;
    let user = app.create_user("media_nostatus").await;

    let resp = app
        .get(
            &format!("/media/upload/{}/status", Uuid::new_v4()),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}
