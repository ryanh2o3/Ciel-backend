use anyhow::Result;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::moderation::{ModerationAction, UserFlag};
use crate::infra::db::Db;

#[derive(Clone)]
pub struct ModerationService {
    db: Db,
}

impl ModerationService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn flag_user(
        &self,
        reporter_id: Uuid,
        target_id: Uuid,
        reason: Option<String>,
    ) -> Result<UserFlag> {
        let mut tx = self.db.pool().begin().await?;
        let flag_row = sqlx::query(
            "INSERT INTO user_flags (reporter_id, target_id, reason) \
             VALUES ($1, $2, $3) \
             RETURNING id, reporter_id, target_id, reason, created_at",
        )
        .bind(reporter_id)
        .bind(target_id)
        .bind(&reason)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO moderation_actions (actor_id, target_type, target_id, reason) \
             VALUES ($1, 'user_flag', $2, $3)",
        )
        .bind(reporter_id)
        .bind(target_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(UserFlag {
            id: flag_row.get("id"),
            reporter_id: flag_row.get("reporter_id"),
            target_id: flag_row.get("target_id"),
            reason: flag_row.get("reason"),
            created_at: flag_row.get("created_at"),
        })
    }

    /// Returns the owner id of the removed post, or None if it didn't exist.
    pub async fn takedown_post(
        &self,
        actor_id: Option<Uuid>,
        post_id: Uuid,
        reason: Option<String>,
    ) -> Result<Option<Uuid>> {
        let mut tx = self.db.pool().begin().await?;
        let owner: Option<Uuid> =
            sqlx::query_scalar("DELETE FROM posts WHERE id = $1 RETURNING owner_id")
                .bind(post_id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(owner_id) = owner else {
            tx.rollback().await?;
            return Ok(None);
        };

        sqlx::query(
            "INSERT INTO moderation_actions (actor_id, target_type, target_id, reason) \
             VALUES ($1, 'post', $2, $3)",
        )
        .bind(actor_id)
        .bind(post_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(owner_id))
    }

    /// Returns the author id of the removed comment, or None if it didn't exist.
    pub async fn takedown_comment(
        &self,
        actor_id: Option<Uuid>,
        comment_id: Uuid,
        reason: Option<String>,
    ) -> Result<Option<Uuid>> {
        let mut tx = self.db.pool().begin().await?;
        let owner: Option<Uuid> =
            sqlx::query_scalar("DELETE FROM comments WHERE id = $1 RETURNING user_id")
                .bind(comment_id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(owner_id) = owner else {
            tx.rollback().await?;
            return Ok(None);
        };

        sqlx::query(
            "INSERT INTO moderation_actions (actor_id, target_type, target_id, reason) \
             VALUES ($1, 'comment', $2, $3)",
        )
        .bind(actor_id)
        .bind(comment_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(owner_id))
    }

    pub async fn list_audit(
        &self,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>> {
        let rows = match cursor {
            Some((created_at, action_id)) => {
                sqlx::query(
                    "SELECT id, actor_id, target_type, target_id, reason, created_at \
                     FROM moderation_actions \
                     WHERE (created_at < $1 OR (created_at = $1 AND id < $2)) \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $3",
                )
                .bind(created_at)
                .bind(action_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, actor_id, target_type, target_id, reason, created_at \
                     FROM moderation_actions \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $1",
                )
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut actions = Vec::with_capacity(rows.len());
        for row in rows {
            actions.push(ModerationAction {
                id: row.get("id"),
                actor_id: row.get::<Option<Uuid>, _>("actor_id"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                reason: row.get("reason"),
                created_at: row.get("created_at"),
            });
        }

        Ok(actions)
    }
}
