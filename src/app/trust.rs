use anyhow::Result;
use sqlx::{Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::config::rate_limits::TrustLevel;
use crate::infra::db::Db;

#[derive(Clone)]
pub struct TrustService {
    db: Db,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TrustScore {
    pub user_id: Uuid,
    pub trust_level: TrustLevel,
    pub trust_points: i32,
    pub account_age_days: i32,
    pub posts_count: i32,
    pub comments_count: i32,
    pub likes_received_count: i32,
    pub followers_count: i32,
    pub flags_received: i32,
    pub strikes: i32,
    pub banned_until: Option<OffsetDateTime>,
    pub invites_sent: i32,
    pub successful_invites: i32,
}

impl TrustService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Initialize trust score for new user
    #[allow(dead_code)]
    pub async fn initialize_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_trust_scores \
             (user_id, trust_level, trust_points, account_age_days) \
             VALUES ($1, 0, 0, 0) \
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn initialize_user_with_tx(
        &self,
        user_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_trust_scores \
             (user_id, trust_level, trust_points, account_age_days) \
             VALUES ($1, 0, 0, 0) \
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Get user's trust score
    pub async fn get_trust_score(&self, user_id: Uuid) -> Result<Option<TrustScore>> {
        let row = sqlx::query(
            "SELECT user_id, trust_level, trust_points, account_age_days, \
                    posts_count, comments_count, likes_received_count, followers_count, \
                    flags_received, strikes, banned_until, \
                    invites_sent, successful_invites \
             FROM user_trust_scores \
             WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await?;

        let score = row.map(|row| {
            let level: i32 = row.get("trust_level");
            TrustScore {
                user_id: row.get("user_id"),
                trust_level: TrustLevel::from_i32(level),
                trust_points: row.get("trust_points"),
                account_age_days: row.get("account_age_days"),
                posts_count: row.get("posts_count"),
                comments_count: row.get("comments_count"),
                likes_received_count: row.get("likes_received_count"),
                followers_count: row.get("followers_count"),
                flags_received: row.get("flags_received"),
                strikes: row.get("strikes"),
                banned_until: row.get("banned_until"),
                invites_sent: row.get("invites_sent"),
                successful_invites: row.get("successful_invites"),
            }
        });

        Ok(score)
    }

    /// Update trust score based on activity
    #[allow(dead_code)]
    pub async fn record_activity(&self, user_id: Uuid, activity_type: &str) -> Result<()> {
        let (points_delta, field_to_increment): (i32, Option<&str>) = match activity_type {
            "post_created" => (5, Some("posts_count")),
            "comment_created" => (2, Some("comments_count")),
            "like_received" => (1, Some("likes_received_count")),
            "follower_gained" => (3, Some("followers_count")),
            "follower_lost" => (-1, None),
            "flag_received" => (-10, None),
            "content_removed" => (-25, None),
            _ => (0, None),
        };

        // Update trust points and activity count
        let mut query = String::from(
            "UPDATE user_trust_scores \
             SET trust_points = GREATEST(0, trust_points + $1), \
                 updated_at = NOW(), \
                 last_activity_at = NOW()",
        );

        if let Some(field) = field_to_increment {
            query.push_str(&format!(", {} = {} + 1", field, field));
        }

        query.push_str(" WHERE user_id = $2");

        sqlx::query(&query)
            .bind(points_delta)
            .bind(user_id)
            .execute(self.db.pool())
            .await?;

        // Recalculate trust level if points changed significantly
        if points_delta.abs() >= 5 {
            self.recalculate_trust_level(user_id).await?;
        }

        Ok(())
    }

    /// Recalculate trust level based on metrics
    #[allow(dead_code)]
    pub async fn recalculate_trust_level(&self, user_id: Uuid) -> Result<()> {
        let mut tx = self.db.pool().begin().await?;
        self.recalculate_trust_level_with_tx(user_id, &mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Check if user is currently banned
    pub async fn is_banned(&self, user_id: Uuid) -> Result<bool> {
        let row = sqlx::query(
            "SELECT banned_until FROM user_trust_scores WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(row) = row {
            if let Some(banned_until) = row.get::<Option<OffsetDateTime>, _>("banned_until") {
                return Ok(banned_until > OffsetDateTime::now_utc());
            }
        }

        Ok(false)
    }

    /// Add strike to user (3 strikes = temporary ban)
    #[allow(dead_code)]
    pub async fn add_strike(&self, user_id: Uuid, reason: &str) -> Result<i32> {
        let mut tx = self.db.pool().begin().await?;
        let strikes = self
            .add_strike_with_tx(user_id, reason, &mut tx)
            .await?;
        tx.commit().await?;
        Ok(strikes)
    }

    async fn add_strike_with_tx(
        &self,
        user_id: Uuid,
        reason: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<i32> {
        let result = sqlx::query(
            "UPDATE user_trust_scores \
             SET strikes = strikes + 1, \
                 flags_received = flags_received + 1, \
                 trust_points = GREATEST(0, trust_points - 50), \
                 updated_at = NOW() \
             WHERE user_id = $1 \
             RETURNING strikes",
        )
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;

        let strikes: i32 = result.get("strikes");

        // Auto-ban after 3 strikes with escalating duration
        if strikes >= 3 {
            let ban_duration_days = match strikes {
                3 => 7,    // 1 week
                4 => 30,   // 1 month
                _ => 365,  // 1 year
            };

            let ban_until = OffsetDateTime::now_utc() + time::Duration::days(ban_duration_days);

            sqlx::query(
                "UPDATE user_trust_scores \
                 SET banned_until = $1, \
                     trust_level = 0, \
                     updated_at = NOW() \
                 WHERE user_id = $2",
            )
            .bind(ban_until)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;

            tracing::warn!(
                user_id = %user_id,
                strikes = strikes,
                ban_until = %ban_until,
                reason = reason,
                "User automatically banned due to strikes"
            );
        }

        self.recalculate_trust_level_with_tx(user_id, tx).await?;
        Ok(strikes)
    }

    async fn recalculate_trust_level_with_tx(
        &self,
        user_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let row = sqlx::query(
            "SELECT account_age_days, posts_count, trust_points, flags_received, strikes \
             FROM user_trust_scores \
             WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;

        let age_days: i32 = row.get("account_age_days");
        let posts: i32 = row.get("posts_count");
        let points: i32 = row.get("trust_points");
        let flags: i32 = row.get("flags_received");
        let strikes: i32 = row.get("strikes");

        let new_level = if strikes >= 3 {
            TrustLevel::New // Demoted due to violations
        } else if age_days >= 90 && posts >= 50 && points >= 200 && flags < 3 {
            TrustLevel::Trusted
        } else if age_days >= 7 && posts >= 5 && points >= 20 && flags < 5 {
            TrustLevel::Basic
        } else {
            TrustLevel::New
        };

        sqlx::query(
            "UPDATE user_trust_scores \
             SET trust_level = $1, updated_at = NOW() \
             WHERE user_id = $2",
        )
        .bind(new_level.as_i32())
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Record a flag/report against a user. Strike thresholds are evaluated
    /// atomically in the same transaction so concurrent flags can't skip a
    /// boundary (e.g. 9→11 without triggering at 10).
    pub async fn record_flag(&self, user_id: Uuid) -> Result<()> {
        let mut tx = self.db.pool().begin().await?;

        let flags: i32 = sqlx::query_scalar(
            "UPDATE user_trust_scores \
             SET flags_received = flags_received + 1, \
                 trust_points = GREATEST(0, trust_points - 10), \
                 updated_at = NOW() \
             WHERE user_id = $1 \
             RETURNING flags_received",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        if flags >= 10 && flags % 10 == 0 {
            self.add_strike_with_tx(user_id, "Excessive flags received", &mut tx)
                .await?;
        } else {
            self.recalculate_trust_level_with_tx(user_id, &mut tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Manually set trust level (admin action)
    #[allow(dead_code)]
    pub async fn set_trust_level(&self, user_id: Uuid, level: TrustLevel) -> Result<()> {
        sqlx::query(
            "UPDATE user_trust_scores \
             SET trust_level = $1, updated_at = NOW() \
             WHERE user_id = $2",
        )
        .bind(level.as_i32())
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get trust level statistics (for admin dashboard)
    #[allow(dead_code)]
    pub async fn get_trust_level_stats(&self) -> Result<Vec<(TrustLevel, i64)>> {
        let rows = sqlx::query(
            "SELECT trust_level, COUNT(*) as count \
             FROM user_trust_scores \
             GROUP BY trust_level \
             ORDER BY trust_level",
        )
        .fetch_all(self.db.pool())
        .await?;

        let stats = rows
            .into_iter()
            .map(|row| {
                let level: i32 = row.get("trust_level");
                let count: i64 = row.get("count");
                (TrustLevel::from_i32(level), count)
            })
            .collect();

        Ok(stats)
    }
}
