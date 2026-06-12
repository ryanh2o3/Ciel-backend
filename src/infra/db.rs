use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use std::time::Duration;

use crate::config::AppConfig;

const CONNECT_MAX_ATTEMPTS: u32 = 10;
const CONNECT_RETRY_DELAY_SECS: u64 = 2;

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(config: &AppConfig) -> Result<Self> {
        // Per-connection statement timeout so a stuck query can't hold a pool
        // connection forever and starve the whole API.
        let statement_timeout_ms = config.db_statement_timeout_seconds * 1000;
        let options = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .acquire_timeout(Duration::from_secs(config.db_connect_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.db_idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.db_max_lifetime_seconds))
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    conn.execute(
                        format!("SET statement_timeout = {}", statement_timeout_ms).as_str(),
                    )
                    .await?;
                    Ok(())
                })
            });

        // Retry at startup: the database is often still coming up when the app
        // starts (container orchestration races), and crash-looping is noisier
        // than a short bounded wait.
        let mut attempt: u32 = 1;
        let pool = loop {
            match options.clone().connect(&config.database_url).await {
                Ok(pool) => break pool,
                Err(err) if attempt < CONNECT_MAX_ATTEMPTS => {
                    tracing::warn!(
                        error = %err,
                        attempt,
                        max_attempts = CONNECT_MAX_ATTEMPTS,
                        "database connection failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(CONNECT_RETRY_DELAY_SECS)).await;
                    attempt += 1;
                }
                Err(err) => return Err(err.into()),
            }
        };

        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
