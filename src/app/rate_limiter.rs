use anyhow::Result;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::config::rate_limits::{current_window, RateLimits, RateWindow, TrustLevel};
use crate::infra::cache::RedisCache;

pub struct RateLimitInfo {
    pub limited: bool,
    pub limit: u32,
    pub remaining: u32,
}

/// Atomically increments the counter and sets the TTL on first increment.
/// Doing this in one Lua script closes the check-then-increment race that
/// previously let bursts of concurrent requests exceed the limit.
const INCR_WITH_TTL_SCRIPT: &str = r#"
local count = redis.call('INCR', KEYS[1])
if count == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return count
"#;

#[derive(Clone)]
pub struct RateLimiter {
    cache: RedisCache,
}

impl RateLimiter {
    pub fn new(cache: RedisCache) -> Self {
        Self { cache }
    }

    async fn incr_with_ttl(&self, key: &str, ttl_seconds: u64) -> Result<u32> {
        let mut conn = self.cache.conn();
        let count: u32 = redis::Script::new(INCR_WITH_TTL_SCRIPT)
            .key(key)
            .arg(ttl_seconds)
            .invoke_async(&mut conn)
            .await?;
        Ok(count)
    }

    /// Atomically check-and-consume one unit of quota for an action.
    /// Errors propagate to the caller (fail closed) rather than silently
    /// allowing unlimited traffic when Redis is unavailable.
    pub async fn check_and_increment(
        &self,
        user_id: Uuid,
        action: &str,
        trust_level: TrustLevel,
    ) -> Result<RateLimitInfo> {
        let limits = RateLimits::for_trust_level(trust_level);

        // Check both hourly and daily limits where applicable
        let checks = match action {
            "post" => vec![
                (limits.posts_per_hour, RateWindow::Hour),
                (limits.posts_per_day, RateWindow::Day),
            ],
            "follow" => vec![
                (limits.follows_per_hour, RateWindow::Hour),
                (limits.follows_per_day, RateWindow::Day),
            ],
            "unfollow" => vec![(limits.unfollows_per_day, RateWindow::Day)],
            "like" => vec![(limits.likes_per_hour, RateWindow::Hour)],
            "comment" => vec![(limits.comments_per_hour, RateWindow::Hour)],
            "login" => vec![(limits.login_attempts_per_hour, RateWindow::Hour)],
            "feed" => vec![(limits.feed_requests_per_hour, RateWindow::Hour)],
            "notifications" => vec![(limits.notifications_per_hour, RateWindow::Hour)],
            "search" => vec![(limits.search_requests_per_hour, RateWindow::Hour)],
            "media_read" => vec![(limits.media_read_per_hour, RateWindow::Hour)],
            "media_upload" => vec![(limits.media_upload_per_hour, RateWindow::Hour)],
            "moderation" => vec![(limits.moderation_actions_per_hour, RateWindow::Hour)],
            _ => return Ok(RateLimitInfo { limited: false, limit: 0, remaining: 0 }),
        };

        // Track the tightest (most constrained) window for response headers
        let mut min_remaining = u32::MAX;
        let mut effective_limit: u32 = 0;
        let mut limited = false;

        for (limit, window) in checks {
            let window_seconds = window.seconds();
            let key = format!(
                "ratelimit:{}:{}:{}",
                user_id,
                action,
                current_window(window_seconds)
            );

            let count = self.incr_with_ttl(&key, window_seconds).await?;
            let remaining = limit.saturating_sub(count);

            if remaining < min_remaining {
                min_remaining = remaining;
                effective_limit = limit;
            }

            if count > limit {
                tracing::debug!(
                    user_id = %user_id,
                    action = action,
                    window = ?window,
                    count = count,
                    limit = limit,
                    "Rate limit exceeded"
                );
                limited = true;
            }
        }

        Ok(RateLimitInfo {
            limited,
            limit: effective_limit,
            remaining: min_remaining,
        })
    }

    /// Get remaining quota for an action (informational; does not consume quota).
    pub async fn get_remaining(
        &self,
        user_id: Uuid,
        action: &str,
        trust_level: TrustLevel,
    ) -> Result<u32> {
        let limits = RateLimits::for_trust_level(trust_level);

        let (limit, window) = match action {
            "post" => (limits.posts_per_hour, RateWindow::Hour),
            "follow" => (limits.follows_per_hour, RateWindow::Hour),
            "like" => (limits.likes_per_hour, RateWindow::Hour),
            "comment" => (limits.comments_per_hour, RateWindow::Hour),
            "media_read" => (limits.media_read_per_hour, RateWindow::Hour),
            "media_upload" => (limits.media_upload_per_hour, RateWindow::Hour),
            _ => return Ok(0),
        };

        let window_seconds = window.seconds();
        let key = format!(
            "ratelimit:{}:{}:{}",
            user_id,
            action,
            current_window(window_seconds)
        );

        let mut conn = self.cache.conn();
        let count: Option<u32> = conn.get(&key).await?;

        Ok(limit.saturating_sub(count.unwrap_or(0)))
    }

    /// Atomically check-and-consume IP-based quota (for unauthenticated requests).
    /// Returns true when the request should be rejected.
    pub async fn check_and_increment_ip(
        &self,
        ip: &str,
        action: &str,
        limit: u32,
        window: RateWindow,
    ) -> Result<bool> {
        let window_seconds = window.seconds();
        let key = format!("ratelimit:ip:{}:{}:{}", ip, action, current_window(window_seconds));

        let count = self.incr_with_ttl(&key, window_seconds).await?;

        if count > limit {
            tracing::debug!(
                ip = ip,
                action = action,
                count = count,
                limit = limit,
                "IP rate limit exceeded"
            );
            return Ok(true);
        }

        Ok(false)
    }
}
