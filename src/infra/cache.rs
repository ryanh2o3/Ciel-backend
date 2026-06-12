use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::Client;

/// Shared Redis handle backed by a single auto-reconnecting multiplexed
/// connection. Cloning is cheap; all callers share the same TCP connection
/// and the manager transparently reconnects after Redis restarts.
#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let mut manager = ConnectionManager::new(client).await?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut manager)
            .await?;
        Ok(Self { manager })
    }

    /// Cheap clone of the shared connection for issuing commands.
    pub fn conn(&self) -> ConnectionManager {
        self.manager.clone()
    }

    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.conn();
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await?;
        Ok(())
    }

    /// Delete all keys matching `pattern` using incremental SCAN
    /// (safe for production — never blocks Redis like KEYS would).
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut conn = self.conn();
        let mut cursor: u64 = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;
            if !keys.is_empty() {
                let _: () = redis::cmd("DEL").arg(&keys).query_async(&mut conn).await?;
            }
            cursor = next;
            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }
}
