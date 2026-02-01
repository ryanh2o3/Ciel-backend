use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub http_addr: String,
    pub app_mode: String,
    pub database_url: String,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_public_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub queue_endpoint: String,
    pub queue_region: String,
    pub queue_name: String,
    pub db_max_connections: u32,
    pub db_connect_timeout_seconds: u64,
    pub db_idle_timeout_seconds: u64,
    pub db_max_lifetime_seconds: u64,
    pub admin_token: Option<String>,
    pub auth_token_ttl_hours: u64,
    pub upload_url_ttl_seconds: u64,
    pub upload_max_bytes: i64,
    pub paseto_access_key: [u8; 32],
    pub paseto_refresh_key: [u8; 32],
    pub access_ttl_minutes: u64,
    pub refresh_ttl_days: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let http_addr = env_or("HTTP_ADDR", "0.0.0.0:8080");
        let _parsed_http_addr = SocketAddr::from_str(&http_addr)
            .map_err(|err| anyhow!("invalid HTTP_ADDR: {}", err))?;
        let app_mode = env_or("APP_MODE", "api");

        let s3_region = env_or("S3_REGION", "fr-par");
        let queue_region = std::env::var("QUEUE_REGION").unwrap_or_else(|_| s3_region.clone());

        Ok(Self {
            http_addr,
            app_mode,
            database_url: env_or_err("DATABASE_URL")?,
            redis_url: env_or("REDIS_URL", "redis://127.0.0.1/"),
            s3_endpoint: env_or_err("S3_ENDPOINT")?,
            s3_public_endpoint: std::env::var("S3_PUBLIC_ENDPOINT").ok(),
            s3_region,
            s3_bucket: env_or_err("S3_BUCKET")?,
            queue_endpoint: env_or_err("QUEUE_ENDPOINT")?,
            queue_region,
            queue_name: env_or_err("QUEUE_NAME")?,
            db_max_connections: env_or_parse("DB_MAX_CONNECTIONS", "10")?,
            db_connect_timeout_seconds: env_or_parse("DB_CONNECT_TIMEOUT_SECONDS", "5")?,
            db_idle_timeout_seconds: env_or_parse("DB_IDLE_TIMEOUT_SECONDS", "300")?,
            db_max_lifetime_seconds: env_or_parse("DB_MAX_LIFETIME_SECONDS", "1800")?,
            admin_token: std::env::var("ADMIN_TOKEN").ok(),
            auth_token_ttl_hours: env_or_parse("AUTH_TOKEN_TTL_HOURS", "168")?,
            upload_url_ttl_seconds: env_or_parse("UPLOAD_URL_TTL_SECONDS", "900")?,
            upload_max_bytes: env_or_parse("UPLOAD_MAX_BYTES", "10485760")?,
            paseto_access_key: env_key_32("PASETO_ACCESS_KEY")?,
            paseto_refresh_key: env_key_32("PASETO_REFRESH_KEY")?,
            access_ttl_minutes: env_or_parse("ACCESS_TTL_MINUTES", "15")?,
            refresh_ttl_days: env_or_parse("REFRESH_TTL_DAYS", "30")?,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_err(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow!("missing required env var: {}", key))
}

fn env_or_parse<T>(key: &str, default: &str) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    let value = std::env::var(key).unwrap_or_else(|_| default.to_string());
    value
        .parse::<T>()
        .map_err(|err| anyhow!("invalid {}: {}", key, err))
}

fn env_key_32(key: &str) -> Result<[u8; 32]> {
    let value = env_or_err(key)?;
    let decoded = STANDARD
        .decode(value.as_bytes())
        .map_err(|err| anyhow!("invalid {}: {}", key, err))?;
    if decoded.len() != 32 {
        return Err(anyhow!("invalid {}: expected 32 bytes", key));
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&decoded);
    Ok(key_bytes)
}

