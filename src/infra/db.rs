use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(config: &AppConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .acquire_timeout(Duration::from_secs(config.db_connect_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.db_idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.db_max_lifetime_seconds))
            .test_before_acquire(true)
            .connect(&config.database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

