//! Authentication & Security Tests
//!
//! Covers login security, token lifecycle, and protected route authorization.

mod common;

use axum::http::StatusCode;
use common::{app, DEFAULT_PASSWORD};
use serde_json::json;
use uuid::Uuid;

// ===========================================================================
// Login Security
// ===========================================================================

#[tokio::test]
async fn login_valid_credentials() {
    let app = app().await;
    let user = app.create_user("login_valid").await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": user.email, "password": DEFAULT_PASSWORD }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert!(body["access_expires_at"].is_string());
    assert!(body["refresh_expires_at"].is_string());
}

#[tokio::test]
async fn login_invalid_password() {
    let app = app().await;
    let user = app.create_user("login_badpw").await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": user.email, "password": "wrong_password" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_message(), "invalid credentials");
}

#[tokio::test]
async fn login_nonexistent_user() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "nobody@example.com", "password": "whatever123" }),
            None,
        )
        .await;

    // Must return 401 with the SAME message as wrong password (no user enumeration)
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_message(), "invalid credentials");
}

#[tokio::test]
async fn login_empty_email() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "", "password": "somepassword" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "email and password are required");
}

#[tokio::test]
async fn login_empty_password() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "someone@example.com", "password": "" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "email and password are required");
}

#[tokio::test]
async fn login_password_too_long() {
    let app = app().await;
    let long_pw: String = "a".repeat(150);

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "someone@example.com", "password": long_pw }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "password must be at most 128 characters"
    );
}

#[tokio::test]
async fn login_sql_injection_email() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "'; DROP TABLE users;--", "password": "whatever123" }),
            None,
        )
        .await;

    // Must not crash, must not leak SQL errors
    assert!(
        resp.status == StatusCode::UNAUTHORIZED || resp.status == StatusCode::BAD_REQUEST,
        "expected 401 or 400, got {}",
        resp.status
    );
}

#[tokio::test]
async fn login_sql_injection_password() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/login",
            json!({ "email": "someone@example.com", "password": "'; DROP TABLE users;--" }),
            None,
        )
        .await;

    assert!(
        resp.status == StatusCode::UNAUTHORIZED || resp.status == StatusCode::BAD_REQUEST,
        "expected 401 or 400, got {}",
        resp.status
    );
}

// ===========================================================================
// Token Lifecycle
// ===========================================================================

#[tokio::test]
async fn refresh_valid_token() {
    let app = app().await;
    let user = app.create_user("refresh_valid").await;

    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());

    // New tokens should differ from originals
    assert_ne!(body["access_token"].as_str().unwrap(), user.access_token);
    assert_ne!(body["refresh_token"].as_str().unwrap(), user.refresh_token);
}

#[tokio::test]
async fn refresh_malformed_token() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": "this-is-not-a-valid-token" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_message(), "invalid refresh token");
}

#[tokio::test]
async fn refresh_empty_token() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": "" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "refresh_token is required");
}

#[tokio::test]
async fn refresh_revoked_token() {
    let app = app().await;
    let user = app.create_user("refresh_revoked").await;

    // Revoke the token first
    let resp = app
        .post_json(
            "/auth/revoke",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Now try to refresh with the revoked token
    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_message(), "invalid refresh token");
}

#[tokio::test]
async fn refresh_already_used_token() {
    let app = app().await;
    let user = app.create_user("refresh_used").await;

    // Refresh once (old token gets rotated/revoked)
    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // Try to refresh again with the same (now-revoked) token
    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_message(), "invalid refresh token");
}

#[tokio::test]
async fn revoke_own_token() {
    let app = app().await;
    let user = app.create_user("revoke_own").await;

    let resp = app
        .post_json(
            "/auth/revoke",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Verify the token is now unusable for refresh
    let resp = app
        .post_json(
            "/auth/refresh",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoke_already_revoked() {
    let app = app().await;
    let user = app.create_user("revoke_twice").await;

    // Revoke once
    let resp = app
        .post_json(
            "/auth/revoke",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Revoke again — should be idempotent (204)
    let resp = app
        .post_json(
            "/auth/revoke",
            json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn revoke_empty_token() {
    let app = app().await;

    let resp = app
        .post_json(
            "/auth/revoke",
            json!({ "refresh_token": "" }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "refresh_token is required");
}

// ===========================================================================
// Protected Route Authorization
// ===========================================================================

#[tokio::test]
async fn get_current_user_no_token() {
    let app = app().await;

    let resp = app.get("/auth/me", None).await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_current_user_invalid_token() {
    let app = app().await;

    let resp = app.get("/auth/me", Some("garbage-token-value")).await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_current_user_valid_token() {
    let app = app().await;
    let user = app.create_user("me_valid").await;

    let resp = app.get("/auth/me", Some(&user.access_token)).await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), user.id.to_string());
    assert_eq!(body["handle"].as_str().unwrap(), user.handle);
    assert_eq!(body["email"].as_str().unwrap(), user.email);
}

#[tokio::test]
async fn create_post_no_auth() {
    let app = app().await;

    let resp = app
        .post_json(
            "/posts",
            json!({
                "media_id": Uuid::new_v4().to_string(),
                "caption": "test"
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_endpoint_no_admin_token() {
    let app = app().await;
    let fake_post_id = Uuid::new_v4();

    let resp = app
        .post_admin(
            &format!("/moderation/posts/{}/takedown", fake_post_id),
            json!({ "reason": "test" }),
            None,
        )
        .await;

    assert!(
        resp.status == StatusCode::FORBIDDEN || resp.status == StatusCode::UNAUTHORIZED,
        "expected 403 or 401, got {}",
        resp.status
    );
}

#[tokio::test]
async fn admin_endpoint_wrong_admin_token() {
    let app = app().await;
    let fake_post_id = Uuid::new_v4();

    let resp = app
        .post_admin(
            &format!("/moderation/posts/{}/takedown", fake_post_id),
            json!({ "reason": "test" }),
            Some("wrong-admin-token"),
        )
        .await;

    assert!(
        resp.status == StatusCode::FORBIDDEN || resp.status == StatusCode::UNAUTHORIZED,
        "expected 403 or 401, got {}",
        resp.status
    );
}

#[tokio::test]
async fn admin_endpoint_valid_admin_token() {
    let app = app().await;
    let fake_post_id = Uuid::new_v4();

    // Valid admin token but non-existent post — should NOT get 403
    let resp = app
        .post_admin(
            &format!("/moderation/posts/{}/takedown", fake_post_id),
            json!({ "reason": "test" }),
            Some(app.admin_token()),
        )
        .await;

    // Should be 404 or 500 (post not found), NOT 403/401
    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "valid admin token should not be rejected"
    );
    assert_ne!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "valid admin token should not be rejected"
    );
}

#[tokio::test]
async fn admin_audit_no_admin_token() {
    let app = app().await;

    let resp = app.get_admin("/moderation/audit", None).await;

    assert!(
        resp.status == StatusCode::FORBIDDEN || resp.status == StatusCode::UNAUTHORIZED,
        "expected 403 or 401, got {}",
        resp.status
    );
}
