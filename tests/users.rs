//! User Tests
//!
//! Covers registration, profile management, and account deletion.

mod common;

use axum::http::StatusCode;
use common::app;
use serde_json::json;
use uuid::Uuid;

// ===========================================================================
// User Registration
// ===========================================================================

#[tokio::test]
async fn signup_valid_data() {
    let app = app().await;
    let inviter = app.create_user("reg_inviter").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "newuser_reg",
                "email": "newuser_reg@example.com",
                "display_name": "New User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["id"].is_string());
    assert_eq!(body["handle"].as_str().unwrap(), "newuser_reg");
    assert_eq!(body["email"].as_str().unwrap(), "newuser_reg@example.com");
    assert_eq!(body["display_name"].as_str().unwrap(), "New User");
}

#[tokio::test]
async fn signup_duplicate_handle() {
    let app = app().await;
    let existing = app.create_user("reg_duphandle").await;
    let inviter = app.create_user("reg_duphandle_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": existing.handle,
                "email": "unique_duphandle@example.com",
                "display_name": "Another User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::CONFLICT);
    assert_eq!(resp.error_message(), "Handle already taken");
}

#[tokio::test]
async fn signup_duplicate_email() {
    let app = app().await;
    let existing = app.create_user("reg_dupemail").await;
    let inviter = app.create_user("reg_dupemail_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "unique_dupemail",
                "email": existing.email,
                "display_name": "Another User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::CONFLICT);
    assert_eq!(resp.error_message(), "Email already taken");
}

#[tokio::test]
async fn signup_invalid_invite_code() {
    let app = app().await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "inv_badcode",
                "email": "inv_badcode@example.com",
                "display_name": "Bad Code User",
                "password": "securepassword123",
                "invite_code": "NONEXISTENT_CODE"
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_expired_invite_code() {
    let app = app().await;
    let inviter = app.create_user("reg_expired_inv").await;
    let code = app.create_expired_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "inv_expired",
                "email": "inv_expired@example.com",
                "display_name": "Expired Invite User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_already_used_invite() {
    let app = app().await;
    let inviter = app.create_user("reg_used_inv").await;
    let code = app.create_used_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "inv_used",
                "email": "inv_used@example.com",
                "display_name": "Used Invite User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_revoked_invite() {
    let app = app().await;
    let inviter = app.create_user("reg_revoked_inv").await;
    let code = app.create_revoked_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "inv_revoked",
                "email": "inv_revoked@example.com",
                "display_name": "Revoked Invite User",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signup_handle_too_short() {
    let app = app().await;
    let inviter = app.create_user("reg_short_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "ab",
                "email": "short_handle@example.com",
                "display_name": "Short Handle",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "handle must be at least 3 characters"
    );
}

#[tokio::test]
async fn signup_handle_too_long() {
    let app = app().await;
    let inviter = app.create_user("reg_long_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "a".repeat(31),
                "email": "long_handle@example.com",
                "display_name": "Long Handle",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "handle must be at most 30 characters"
    );
}

#[tokio::test]
async fn signup_handle_special_chars() {
    let app = app().await;
    let inviter = app.create_user("reg_special_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "bad@handle#",
                "email": "special_handle@example.com",
                "display_name": "Special Handle",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "handle can only contain letters, numbers, and underscores"
    );
}

#[tokio::test]
async fn signup_password_too_short() {
    let app = app().await;
    let inviter = app.create_user("reg_shortpw_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "shortpw_user",
                "email": "shortpw@example.com",
                "display_name": "Short PW User",
                "password": "1234567",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "password must be at least 8 characters"
    );
}

#[tokio::test]
async fn signup_password_too_long() {
    let app = app().await;
    let inviter = app.create_user("reg_longpw_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "longpw_user",
                "email": "longpw@example.com",
                "display_name": "Long PW User",
                "password": "a".repeat(129),
                "invite_code": code
            }),
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
async fn signup_bio_too_long() {
    let app = app().await;
    let inviter = app.create_user("reg_longbio_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "longbio_user",
                "email": "longbio@example.com",
                "display_name": "Long Bio User",
                "bio": "a".repeat(501),
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "bio must be at most 500 characters"
    );
}

#[tokio::test]
async fn signup_display_name_empty() {
    let app = app().await;
    let inviter = app.create_user("reg_emptydn_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "emptydn_user",
                "email": "emptydn@example.com",
                "display_name": "",
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "display_name cannot be empty");
}

#[tokio::test]
async fn signup_display_name_too_long() {
    let app = app().await;
    let inviter = app.create_user("reg_longdn_inv").await;
    let code = app.create_invite_code(inviter.id).await;

    let resp = app
        .post_json(
            "/users",
            json!({
                "handle": "longdn_user",
                "email": "longdn@example.com",
                "display_name": "a".repeat(51),
                "password": "securepassword123",
                "invite_code": code
            }),
            None,
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "display_name must be at most 50 characters"
    );
}

// ===========================================================================
// Profile Management
// ===========================================================================

#[tokio::test]
async fn update_own_profile() {
    let app = app().await;
    let user = app.create_user("prof_update").await;

    let resp = app
        .patch_json(
            &format!("/users/{}", user.id),
            json!({ "display_name": "Updated Name", "bio": "Hello world" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["display_name"].as_str().unwrap(), "Updated Name");
    assert_eq!(body["bio"].as_str().unwrap(), "Hello world");
}

#[tokio::test]
async fn update_other_user_profile() {
    let app = app().await;
    let user_a = app.create_user("prof_other_a").await;
    let user_b = app.create_user("prof_other_b").await;

    let resp = app
        .patch_json(
            &format!("/users/{}", user_b.id),
            json!({ "display_name": "Hacked Name" }),
            Some(&user_a.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN);
    assert_eq!(resp.error_message(), "cannot update other users");
}

#[tokio::test]
async fn update_profile_empty_display_name() {
    let app = app().await;
    let user = app.create_user("prof_emptydn").await;

    let resp = app
        .patch_json(
            &format!("/users/{}", user.id),
            json!({ "display_name": "" }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(resp.error_message(), "display_name cannot be empty");
}

#[tokio::test]
async fn update_profile_bio_too_long() {
    let app = app().await;
    let user = app.create_user("prof_longbio").await;

    let resp = app
        .patch_json(
            &format!("/users/{}", user.id),
            json!({ "bio": "a".repeat(501) }),
            Some(&user.access_token),
        )
        .await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.error_message(),
        "bio must be at most 500 characters"
    );
}

#[tokio::test]
async fn get_user_by_id() {
    let app = app().await;
    let user = app.create_user("prof_getuser").await;

    // GET /users/:id is a public endpoint (no auth needed)
    let resp = app.get(&format!("/users/{}", user.id), None).await;

    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert_eq!(body["id"].as_str().unwrap(), user.id.to_string());
    assert_eq!(body["handle"].as_str().unwrap(), user.handle);
    assert_eq!(body["display_name"].as_str().unwrap(), format!("Test User prof_getuser"));
}

#[tokio::test]
async fn get_nonexistent_user() {
    let app = app().await;

    let resp = app
        .get(&format!("/users/{}", Uuid::new_v4()), None)
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    assert_eq!(resp.error_message(), "user not found");
}

// ===========================================================================
// Account Deletion
// ===========================================================================

#[tokio::test]
async fn delete_own_account() {
    let app = app().await;
    let user = app.create_user("del_own").await;

    let resp = app
        .delete("/account", Some(&user.access_token))
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Verify user no longer exists
    let resp = app.get(&format!("/users/{}", user.id), None).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_account_invalidates_tokens() {
    let app = app().await;
    let user = app.create_user("del_tokens").await;
    let token = user.access_token.clone();

    // Delete account
    let resp = app.delete("/account", Some(&token)).await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Old access token should no longer work for protected endpoints
    let resp = app.get("/auth/me", Some(&token)).await;
    assert!(
        resp.status == StatusCode::UNAUTHORIZED || resp.status == StatusCode::NOT_FOUND,
        "expected 401 or 404 after account deletion, got {}",
        resp.status
    );
}

#[tokio::test]
async fn delete_account_cascades_follows() {
    let app = app().await;
    let user_a = app.create_user("del_follow_a").await;
    let user_b = app.create_user("del_follow_b").await;

    // A follows B
    let resp = app
        .post_json(
            &format!("/users/{}/follow", user_b.id),
            json!({}),
            Some(&user_a.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // Verify A is in B's followers
    let resp = app
        .get(
            &format!("/users/{}/followers", user_b.id),
            Some(&user_b.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    assert!(body["items"].as_array().unwrap().len() >= 1);

    // Delete A's account
    let resp = app.delete("/account", Some(&user_a.access_token)).await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Verify A is no longer in B's followers
    let resp = app
        .get(
            &format!("/users/{}/followers", user_b.id),
            Some(&user_b.access_token),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json();
    let followers = body["items"].as_array().unwrap();
    let has_deleted_user = followers
        .iter()
        .any(|f| f["user"]["id"].as_str() == Some(&user_a.id.to_string()));
    assert!(!has_deleted_user, "deleted user should not appear in followers");
}
