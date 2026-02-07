pub mod app;
pub mod config;
pub mod domain;
pub mod http;
pub mod infra;
pub mod jobs;

use crate::infra::{cache::RedisCache, db::Db, queue::QueueClient, storage::ObjectStorage};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub cache: RedisCache,
    pub storage: ObjectStorage,
    pub queue: QueueClient,
    pub auth_token_ttl_hours: u64,
    pub upload_url_ttl_seconds: u64,
    pub upload_max_bytes: i64,
    pub admin_token: Option<String>,
    pub paseto_access_key: [u8; 32],
    pub paseto_refresh_key: [u8; 32],
    pub access_ttl_minutes: u64,
    pub refresh_ttl_days: u64,
    pub s3_public_endpoint: Option<String>,
}
