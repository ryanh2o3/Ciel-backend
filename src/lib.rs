pub mod app;
pub mod config;
pub mod domain;
pub mod http;
pub mod infra;
pub mod jobs;

use crate::infra::{cache::RedisCache, db::Db, queue::QueueClient, storage::ObjectStorage};
use crate::jobs::notifications::NotificationJob;
use ipnet::IpNet;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub cache: RedisCache,
    pub storage: ObjectStorage,
    pub queue: QueueClient,
    pub metrics: PrometheusHandle,
    pub upload_url_ttl_seconds: u64,
    pub upload_max_bytes: i64,
    pub admin_token: Option<String>,
    pub paseto_access_key: [u8; 32],
    pub paseto_refresh_key: [u8; 32],
    pub access_ttl_minutes: u64,
    pub refresh_ttl_days: u64,
    pub s3_public_endpoint: Option<String>,
    pub ip_signup_rate_limit: u32,
    pub trusted_proxy_cidrs: Arc<Vec<IpNet>>,
    pub notification_tx: mpsc::Sender<NotificationJob>,
}
