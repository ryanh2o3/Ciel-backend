//! Safety & Anti-Abuse Tests
//!
//! Covers invites, trust scores, and device fingerprinting.

mod common;

use axum::http::StatusCode;
use common::app;
use serde_json::json;

// ===========================================================================
// Invite System
// ===========================================================================

#[tokio::test]
async fn create_invite() {
    let app = app().await;
    let user = app.create_user("safe_invite_create").await;

    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 7 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["code"].is_string());
    assert_eq!(body["created_by"].as_str().unwrap(), user.id.to_string());
    assert_eq!(body["is_valid"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn create_invite_invalid_days_zero() {
    let app = app().await;
    let user = app.create_user("safe_invite_zero").await;

    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 0 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "days_valid must be between 1 and 30"
    );
}

#[tokio::test]
async fn create_invite_invalid_days_over_max() {
    let app = app().await;
    let user = app.create_user("safe_invite_over").await;

    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 31 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "days_valid must be between 1 and 30"
    );
}

#[tokio::test]
async fn create_invite_default_days() {
    let app = app().await;
    let user = app.create_user("safe_invite_default").await;

    // Send empty JSON — days_valid defaults to 7
    let resp = app
        .post_json("/invites", json!({}), Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["code"].is_string());
}

#[tokio::test]
async fn list_invites() {
    let app = app().await;
    let user = app.create_user("safe_invite_list").await;

    // Create an invite via the API
    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 7 }),
            Some(&user.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // List invites
    let resp = app.get("/invites", Some(&user.access_token)).await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    let invites = body.as_array().unwrap();
    assert!(invites.len() >= 1);
}

#[tokio::test]
async fn get_invite_stats() {
    let app = app().await;
    let user = app.create_user("safe_invite_stats").await;

    let resp = app
        .get("/invites/stats", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["invites_sent"].is_number());
    assert!(body["successful_invites"].is_number());
    assert!(body["remaining_invites"].is_number());
    assert!(body["max_invites"].is_number());
}

#[tokio::test]
async fn revoke_invite() {
    let app = app().await;
    let user = app.create_user("safe_invite_revoke").await;

    // Create an invite via the API
    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 7 }),
            Some(&user.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let code = resp.json()["code"].as_str().unwrap().to_string();

    // Revoke it
    let resp = app
        .post_json(
            &format!("/invites/{}/revoke", code),
            json!({}),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn revoke_other_users_invite() {
    let app = app().await;
    let user_a = app.create_user("safe_revoke_other_a").await;
    let user_b = app.create_user("safe_revoke_other_b").await;

    // A creates an invite
    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 7 }),
            Some(&user_a.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let code = resp.json()["code"].as_str().unwrap().to_string();

    // B tries to revoke A's invite
    let resp = app
        .post_json(
            &format!("/invites/{}/revoke", code),
            json!({}),
            Some(&user_b.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_invite_max_limit() {
    let app = app().await;
    let user = app.create_user("safe_invite_max").await;

    // New users (trust_level=0) get 3 invites max
    // Create 3 invites via the API
    for _ in 0..3 {
        let resp = app
            .post_json(
                "/invites",
                json!({ "days_valid": 7 }),
                Some(&user.access_token),
            )
            .await;
        assert_eq!(resp.status, StatusCode::OK);
    }

    // 4th invite should fail
    let resp = app
        .post_json(
            "/invites",
            json!({ "days_valid": 7 }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN);
    assert!(resp.error_message().contains("Maximum invite limit"));
}

// ===========================================================================
// Trust System
// ===========================================================================

#[tokio::test]
async fn get_trust_score() {
    let app = app().await;
    let user = app.create_user("safe_trust_score").await;

    let resp = app
        .get("/account/trust-score", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["user_id"].as_str().unwrap(), user.id.to_string());
    assert!(body["trust_level"].is_number());
    assert!(body["trust_level_name"].is_string());
    assert!(body["trust_points"].is_number());
    assert_eq!(body["is_banned"].as_bool().unwrap(), false);
}

#[tokio::test]
async fn get_rate_limits() {
    let app = app().await;
    let user = app.create_user("safe_rate_limits").await;

    let resp = app
        .get("/account/rate-limits", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["trust_level"].is_string());
    assert!(body["posts_per_hour"].is_number());
    assert!(body["posts_per_day"].is_number());
    assert!(body["follows_per_hour"].is_number());
    assert!(body["likes_per_hour"].is_number());
    assert!(body["comments_per_hour"].is_number());
    assert!(body["remaining"].is_object());
}

#[tokio::test]
async fn trust_score_no_auth() {
    let app = app().await;

    let resp = app.get("/account/trust-score", None).await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// Device Fingerprinting
// ===========================================================================

#[tokio::test]
async fn register_device() {
    let app = app().await;
    let user = app.create_user("safe_device_reg").await;

    let resp = app
        .post_json(
            "/account/device/register",
            json!({ "fingerprint": "test-fingerprint-abc123" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn register_device_unauthenticated() {
    let app = app().await;

    // Device registration also works without auth (for pre-login fingerprinting)
    let resp = app
        .post_json(
            "/account/device/register",
            json!({ "fingerprint": "unauth-fingerprint-xyz" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn register_device_blocked() {
    let app = app().await;
    let user = app.create_user("safe_device_blocked").await;

    // Block a fingerprint hash directly in DB
    let fingerprint_hash =
        ciel::app::fingerprint::FingerprintService::hash_fingerprint("blocked-device-fp");
    app.block_device_fingerprint(&fingerprint_hash).await;

    let resp = app
        .post_json(
            "/account/device/register",
            json!({ "fingerprint": "blocked-device-fp" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN);
    assert_eq!(resp.error_message(), "This device has been blocked");
}

#[tokio::test]
async fn list_user_devices() {
    let app = app().await;
    let user = app.create_user("safe_device_list").await;

    // Register a device first
    app.post_json(
        "/account/device/register",
        json!({ "fingerprint": "list-test-fingerprint" }),
        Some(&user.access_token),
    )
    .await;

    let resp = app
        .get("/account/devices", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    let devices = body.as_array().unwrap();
    assert!(devices.len() >= 1);
    assert!(devices[0]["fingerprint_hash"].is_string());
    assert!(devices[0]["risk_score"].is_number());
    assert!(devices[0]["is_blocked"].is_boolean());
}

#[tokio::test]
async fn list_user_devices_empty() {
    let app = app().await;
    let user = app.create_user("safe_device_empty").await;

    // No devices registered
    let resp = app
        .get("/account/devices", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    let devices = body.as_array().unwrap();
    assert_eq!(devices.len(), 0);
}

#[tokio::test]
async fn register_device_twice_idempotent() {
    let app = app().await;
    let user = app.create_user("safe_device_idem").await;

    // Register same fingerprint twice
    let resp = app
        .post_json(
            "/account/device/register",
            json!({ "fingerprint": "idempotent-fp-test" }),
            Some(&user.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    let resp = app
        .post_json(
            "/account/device/register",
            json!({ "fingerprint": "idempotent-fp-test" }),
            Some(&user.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Should still show only one device
    let resp = app
        .get("/account/devices", Some(&user.access_token))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let devices = resp.json().as_array().unwrap().len();
    assert_eq!(devices, 1);
}
