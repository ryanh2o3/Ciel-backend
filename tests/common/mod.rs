#![allow(dead_code)]

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::extract::connect_info::ConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::sync::OnceCell;
use tower::ServiceExt;
use uuid::Uuid;

use ciel::app::auth::AuthService;
use ciel::config::AppConfig;
use ciel::infra::{cache::RedisCache, db::Db, queue::QueueClient, storage::ObjectStorage};
use ciel::AppState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// 32 bytes base64-encoded (test-only keys — NOT used in production)
// "0123456789abcdef0123456789abcdef" (32 bytes)
const TEST_PASETO_ACCESS_KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
// "fedcba9876543210fedcba9876543210" (32 bytes)
const TEST_PASETO_REFRESH_KEY: &str = "ZmVkY2JhOTg3NjU0MzIxMGZlZGNiYTk4NzY1NDMyMTA=";
const TEST_ADMIN_TOKEN: &str = "test-admin-token-12345";
pub const DEFAULT_PASSWORD: &str = "testpassword123";

// ---------------------------------------------------------------------------
// TestApp — shared, lazily initialized once per test binary
// ---------------------------------------------------------------------------

pub struct TestApp {
    router: Router,
    pub state: AppState,
}

pub struct TestResponse {
    pub status: StatusCode,
    body_bytes: bytes::Bytes,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body_bytes).unwrap_or(Value::Null)
    }

    pub fn error_message(&self) -> String {
        self.json()["error"].as_str().unwrap_or("").to_string()
    }
}

pub struct TestUser {
    pub id: Uuid,
    pub handle: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
}

static TEST_APP: OnceCell<TestApp> = OnceCell::const_new();

/// Get (or lazily create) the shared TestApp instance.
pub async fn app() -> &'static TestApp {
    TEST_APP
        .get_or_init(|| async { TestApp::setup().await })
        .await
}

impl TestApp {
    // ------------------------------------------------------------------
    // Setup — runs once per test binary
    // ------------------------------------------------------------------
    async fn setup() -> Self {
        // Env vars that control test infra (override with env for CI)
        let base_url = std::env::var("TEST_DATABASE_BASE_URL")
            .unwrap_or_else(|_| "postgres://ciel:ciel@localhost:5432".into());
        let test_db = std::env::var("TEST_DATABASE_NAME")
            .unwrap_or_else(|_| "ciel_test".into());
        let redis_url = std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379/1".into());
        let s3_endpoint = std::env::var("TEST_S3_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4566".into());
        let queue_endpoint = std::env::var("TEST_QUEUE_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4566".into());

        // ---- Create test database if needed ----
        let admin_pool = PgPool::connect(&format!("{}/postgres", base_url))
            .await
            .expect("cannot connect to postgres admin database");

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
                .bind(&test_db)
                .fetch_one(&admin_pool)
                .await
                .expect("failed to check test db existence");

        if !exists {
            // CREATE DATABASE cannot run inside a transaction
            sqlx::query(&format!("CREATE DATABASE \"{}\"", test_db))
                .execute(&admin_pool)
                .await
                .expect("failed to create test database");
        }
        admin_pool.close().await;

        // ---- Connect to test database ----
        let database_url = format!("{}/{}", base_url, test_db);
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("cannot connect to test database");

        // ---- Run migrations ----
        let mut migration_files: Vec<_> = std::fs::read_dir("migrations")
            .expect("cannot read migrations/")
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "sql")
            })
            .collect();
        migration_files.sort_by_key(|e| e.file_name());

        for entry in &migration_files {
            let sql = std::fs::read_to_string(entry.path())
                .unwrap_or_else(|_| panic!("cannot read {:?}", entry.path()));
            sqlx::raw_sql(&sql).execute(&db_pool).await.unwrap_or_else(
                |e| panic!("migration {:?} failed: {}", entry.file_name(), e),
            );
        }

        // ---- Truncate all tables for clean test state ----
        sqlx::raw_sql(
            "DO $$ DECLARE r RECORD; BEGIN \
             FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') LOOP \
             EXECUTE 'TRUNCATE TABLE ' || quote_ident(r.tablename) || ' CASCADE'; \
             END LOOP; END $$;",
        )
        .execute(&db_pool)
        .await
        .expect("failed to truncate tables");

        db_pool.close().await;

        // ---- Flush test Redis (DB 1) to clear stale rate-limit counters ----
        {
            let redis_client = redis::Client::open(redis_url.as_str())
                .expect("cannot open Redis client for flush");
            let mut conn = redis_client
                .get_multiplexed_async_connection()
                .await
                .expect("cannot connect to Redis for flush");
            redis::cmd("FLUSHDB")
                .query_async::<_, ()>(&mut conn)
                .await
                .expect("FLUSHDB failed");
        }

        // ---- Build AppState via AppConfig (same code path as production) ----
        // Ensure the 32-byte keys decode correctly
        assert_eq!(STANDARD.decode(TEST_PASETO_ACCESS_KEY).unwrap().len(), 32);
        assert_eq!(STANDARD.decode(TEST_PASETO_REFRESH_KEY).unwrap().len(), 32);

        std::env::set_var("DATABASE_URL", &database_url);
        std::env::set_var("REDIS_URL", &redis_url);
        std::env::set_var("S3_ENDPOINT", &s3_endpoint);
        std::env::set_var("S3_BUCKET", "ciel-media-test");
        std::env::set_var("S3_REGION", "us-east-1");
        std::env::set_var("QUEUE_ENDPOINT", &queue_endpoint);
        std::env::set_var("QUEUE_NAME", "ciel-media-jobs-test");
        std::env::set_var("QUEUE_REGION", "us-east-1");
        std::env::set_var("PASETO_ACCESS_KEY", TEST_PASETO_ACCESS_KEY);
        std::env::set_var("PASETO_REFRESH_KEY", TEST_PASETO_REFRESH_KEY);
        std::env::set_var("ADMIN_TOKEN", TEST_ADMIN_TOKEN);
        std::env::set_var("APP_MODE", "api");
        std::env::set_var("DB_MAX_CONNECTIONS", "10");
        std::env::set_var("DB_CONNECT_TIMEOUT_SECONDS", "30");
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");

        let config = AppConfig::from_env().expect("failed to build AppConfig");

        let db = Db::connect(&config).await.expect("Db::connect failed");
        let cache = RedisCache::connect(&config.redis_url)
            .await
            .expect("Redis connect failed");
        let storage = ObjectStorage::new(&config)
            .await
            .expect("ObjectStorage::new failed");
        let queue = QueueClient::new(&config)
            .await
            .expect("QueueClient::new failed");

        let state = AppState {
            db,
            cache,
            storage,
            queue,
            upload_url_ttl_seconds: config.upload_url_ttl_seconds,
            upload_max_bytes: config.upload_max_bytes,
            admin_token: config.admin_token.clone(),
            paseto_access_key: config.paseto_access_key,
            paseto_refresh_key: config.paseto_refresh_key,
            access_ttl_minutes: config.access_ttl_minutes,
            refresh_ttl_days: config.refresh_ttl_days,
            s3_public_endpoint: config.s3_public_endpoint,
        };

        let router = ciel::http::router(state.clone());

        TestApp { router, state }
    }

    // ------------------------------------------------------------------
    // Low-level request helper
    // ------------------------------------------------------------------
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        headers: &[(&str, &str)],
    ) -> TestResponse {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "localhost");

        for &(key, value) in headers {
            builder = builder.header(key, value);
        }

        let request = if let Some(body) = body {
            builder
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        };

        // Inject ConnectInfo so the IP-rate-limit middleware can extract it.
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));

        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("oneshot failed");

        let status = response.status();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to collect body")
            .to_bytes();

        TestResponse { status, body_bytes }
    }

    // ------------------------------------------------------------------
    // Convenience HTTP helpers
    // ------------------------------------------------------------------
    pub async fn get(&self, path: &str, token: Option<&str>) -> TestResponse {
        let mut headers = vec![];
        let auth;
        if let Some(t) = token {
            auth = format!("Bearer {}", t);
            headers.push(("Authorization", auth.as_str()));
        }
        self.request(Method::GET, path, None, &headers).await
    }

    pub async fn post_json(&self, path: &str, body: Value, token: Option<&str>) -> TestResponse {
        let mut headers = vec![];
        let auth;
        if let Some(t) = token {
            auth = format!("Bearer {}", t);
            headers.push(("Authorization", auth.as_str()));
        }
        self.request(Method::POST, path, Some(body), &headers).await
    }

    pub async fn patch_json(&self, path: &str, body: Value, token: Option<&str>) -> TestResponse {
        let mut headers = vec![];
        let auth;
        if let Some(t) = token {
            auth = format!("Bearer {}", t);
            headers.push(("Authorization", auth.as_str()));
        }
        self.request(Method::PATCH, path, Some(body), &headers)
            .await
    }

    pub async fn delete(&self, path: &str, token: Option<&str>) -> TestResponse {
        let mut headers = vec![];
        let auth;
        if let Some(t) = token {
            auth = format!("Bearer {}", t);
            headers.push(("Authorization", auth.as_str()));
        }
        self.request(Method::DELETE, path, None, &headers).await
    }

    /// POST with an admin token in the x-admin-token header.
    pub async fn post_admin(
        &self,
        path: &str,
        body: Value,
        admin_token: Option<&str>,
    ) -> TestResponse {
        let mut headers = vec![];
        if let Some(t) = admin_token {
            headers.push(("x-admin-token", t));
        }
        self.request(Method::POST, path, Some(body), &headers).await
    }

    /// GET with an admin token in the x-admin-token header.
    pub async fn get_admin(&self, path: &str, admin_token: Option<&str>) -> TestResponse {
        let mut headers = vec![];
        if let Some(t) = admin_token {
            headers.push(("x-admin-token", t));
        }
        self.request(Method::GET, path, None, &headers).await
    }

    // ------------------------------------------------------------------
    // Test data helpers
    // ------------------------------------------------------------------

    /// Create a user directly in the DB and log in via the API to obtain tokens.
    pub async fn create_user(&self, suffix: &str) -> TestUser {
        let handle = format!("testuser_{}", suffix);
        let email = format!("test_{}@example.com", suffix);
        let display_name = format!("Test User {}", suffix);
        let password = DEFAULT_PASSWORD;

        // Hash password with Argon2 (same algorithm as production)
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("password hash failed")
            .to_string();

        let pool = self.state.db.pool();

        // Insert user
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (handle, email, display_name, password_hash) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(&handle)
        .bind(&email)
        .bind(&display_name)
        .bind(&hash)
        .fetch_one(pool)
        .await
        .expect("insert test user failed");

        // Initialize trust score (matches TrustService::initialize_user)
        sqlx::query(
            "INSERT INTO user_trust_scores (user_id, trust_level, trust_points) \
             VALUES ($1, 0, 0) ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("init trust score failed");

        // Issue tokens directly via AuthService (avoids IP rate-limiting)
        let auth_service = AuthService::new(
            self.state.db.clone(),
            self.state.paseto_access_key,
            self.state.paseto_refresh_key,
            self.state.access_ttl_minutes,
            self.state.refresh_ttl_days,
        );
        let tokens = auth_service
            .issue_token_pair(user_id)
            .await
            .expect("issue_token_pair failed");

        TestUser {
            id: user_id,
            handle,
            email,
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
        }
    }

    /// Create an invite code owned by the given user. Returns the code string.
    pub async fn create_invite_code(&self, user_id: Uuid) -> String {
        let code: String = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..12)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect()
        };

        let pool = self.state.db.pool();

        // Ensure invites_sent column is present (migration 007)
        sqlx::query(
            "INSERT INTO invite_codes (code, created_by, expires_at, is_valid, invite_type, max_uses, use_count) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days', true, 'standard', 1, 0)",
        )
        .bind(&code)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert invite code failed");

        // Increment invites_sent counter
        sqlx::query(
            "UPDATE user_trust_scores SET invites_sent = invites_sent + 1 WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("update invites_sent failed");

        code
    }

    /// Return the admin token used by the test infrastructure.
    pub fn admin_token(&self) -> &str {
        TEST_ADMIN_TOKEN
    }

    /// Return the pool for direct DB assertions.
    pub fn pool(&self) -> &PgPool {
        self.state.db.pool()
    }

    /// Insert a media record directly in DB (bypasses S3 upload flow). Returns media id.
    pub async fn create_media(&self, owner_id: Uuid) -> Uuid {
        let pool = self.state.db.pool();
        let unique = Uuid::new_v4();
        let media_id: Uuid = sqlx::query_scalar(
            "INSERT INTO media (owner_id, original_key, thumb_key, medium_key, width, height, bytes) \
             VALUES ($1, $2, $3, $4, 1920, 1080, 1024) RETURNING id",
        )
        .bind(owner_id)
        .bind(format!("test/{}/original.jpg", unique))
        .bind(format!("test/{}/thumb.jpg", unique))
        .bind(format!("test/{}/medium.jpg", unique))
        .fetch_one(pool)
        .await
        .expect("insert test media failed");
        media_id
    }

    /// Insert a media record + post directly in DB. Returns (post_id, media_id).
    pub async fn create_post_for_user(&self, owner_id: Uuid) -> (Uuid, Uuid) {
        let media_id = self.create_media(owner_id).await;
        let pool = self.state.db.pool();
        let post_id: Uuid = sqlx::query_scalar(
            "INSERT INTO posts (owner_id, media_id, caption, visibility) \
             VALUES ($1, $2, 'test caption', 'public'::post_visibility) RETURNING id",
        )
        .bind(owner_id)
        .bind(media_id)
        .fetch_one(pool)
        .await
        .expect("insert test post failed");
        (post_id, media_id)
    }

    /// Create an expired invite code directly in DB. Returns the code string.
    pub async fn create_expired_invite_code(&self, user_id: Uuid) -> String {
        let code: String = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..12)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect()
        };
        let pool = self.state.db.pool();
        sqlx::query(
            "INSERT INTO invite_codes (code, created_by, expires_at, is_valid, invite_type, max_uses, use_count) \
             VALUES ($1, $2, NOW() - INTERVAL '1 day', true, 'standard', 1, 0)",
        )
        .bind(&code)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert expired invite code failed");
        code
    }

    /// Create a revoked invite code directly in DB. Returns the code string.
    pub async fn create_revoked_invite_code(&self, user_id: Uuid) -> String {
        let code: String = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..12)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect()
        };
        let pool = self.state.db.pool();
        sqlx::query(
            "INSERT INTO invite_codes (code, created_by, expires_at, is_valid, invite_type, max_uses, use_count) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days', false, 'standard', 1, 0)",
        )
        .bind(&code)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert revoked invite code failed");
        code
    }

    /// Create a fully-used invite code directly in DB. Returns the code string.
    pub async fn create_used_invite_code(&self, user_id: Uuid) -> String {
        let code: String = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..12)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect()
        };
        let pool = self.state.db.pool();
        sqlx::query(
            "INSERT INTO invite_codes (code, created_by, expires_at, is_valid, invite_type, max_uses, use_count) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days', false, 'standard', 1, 1)",
        )
        .bind(&code)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert used invite code failed");
        code
    }

    /// Block a device fingerprint directly in DB for testing.
    pub async fn block_device_fingerprint(&self, fingerprint_hash: &str) {
        let pool = self.state.db.pool();
        sqlx::query(
            "INSERT INTO device_fingerprints (fingerprint_hash, user_ids, account_count, risk_score, is_blocked, block_reason) \
             VALUES ($1, '{}', 0, 100, true, 'test block') \
             ON CONFLICT (fingerprint_hash) DO UPDATE SET is_blocked = true, block_reason = 'test block'",
        )
        .bind(fingerprint_hash)
        .execute(pool)
        .await
        .expect("block device fingerprint failed");
    }
}
