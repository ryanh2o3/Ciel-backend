use anyhow::Result;
use serde_json::Value;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::notification::Notification;
use crate::infra::db::Db;

#[derive(Clone)]
pub struct NotificationService {
    db: Db,
}

impl NotificationService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        notification_type: &str,
        payload: Value,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO notifications (user_id, notification_type, payload) \
             VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(notification_type)
        .bind(payload)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Creates a notification only if actor != recipient.
    /// Looks up actor handle/display_name and merges into payload.
    pub async fn create_if_not_self(
        &self,
        recipient_id: Uuid,
        actor_id: Uuid,
        notification_type: &str,
        mut payload: Value,
    ) -> Result<()> {
        if actor_id == recipient_id {
            return Ok(());
        }

        // Look up actor info
        let actor_row = sqlx::query(
            "SELECT handle, display_name FROM users WHERE id = $1",
        )
        .bind(actor_id)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(row) = actor_row {
            let handle: String = row.get("handle");
            let display_name: String = row.get("display_name");
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("actor_id".to_string(), Value::String(actor_id.to_string()));
                obj.insert("actor_handle".to_string(), Value::String(handle));
                obj.insert("actor_display_name".to_string(), Value::String(display_name));
            }
        }

        self.create(recipient_id, notification_type, payload).await
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        let rows = match cursor {
            Some((created_at, notification_id)) => {
                sqlx::query(
                    "SELECT id, user_id, notification_type, payload, read_at, created_at \
                     FROM notifications \
                     WHERE user_id = $1 \
                       AND (created_at < $2 OR (created_at = $2 AND id < $3)) \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(created_at)
                .bind(notification_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, user_id, notification_type, payload, read_at, created_at \
                     FROM notifications \
                     WHERE user_id = $1 \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut notifications = Vec::with_capacity(rows.len());
        for row in rows {
            notifications.push(Notification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                notification_type: row.get("notification_type"),
                payload: row.get("payload"),
                read_at: row.get("read_at"),
                created_at: row.get("created_at"),
            });
        }

        Ok(notifications)
    }

    pub async fn mark_read(&self, notification_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE notifications \
             SET read_at = now() \
             WHERE id = $1 AND user_id = $2 AND read_at IS NULL",
        )
        .bind(notification_id)
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
