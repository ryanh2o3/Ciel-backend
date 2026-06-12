use anyhow::Result;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::engagement::{Comment, Like};
use crate::infra::db::Db;

#[derive(Clone)]
pub struct EngagementService {
    db: Db,
}

impl EngagementService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn like_post(&self, user_id: Uuid, post_id: Uuid) -> Result<Option<Like>> {
        let row = sqlx::query(
            "INSERT INTO likes (user_id, post_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING \
             RETURNING id, user_id, post_id, created_at",
        )
        .bind(user_id)
        .bind(post_id)
        .fetch_optional(self.db.pool())
        .await?;

        let like = row.map(|row| Like {
            id: row.get("id"),
            user_id: row.get("user_id"),
            post_id: row.get("post_id"),
            created_at: row.get("created_at"),
        });

        Ok(like)
    }

    pub async fn comment_post(
        &self,
        user_id: Uuid,
        post_id: Uuid,
        body: String,
    ) -> Result<Comment> {
        let row = sqlx::query(
            "WITH inserted AS ( \
                INSERT INTO comments (user_id, post_id, body) VALUES ($1, $2, $3) \
                RETURNING id, user_id, post_id, body, created_at \
             ) \
             SELECT i.id, i.user_id, i.post_id, i.body, i.created_at, \
                    u.handle AS user_handle, u.display_name AS user_display_name \
             FROM inserted i \
             JOIN users u ON u.id = i.user_id",
        )
        .bind(user_id)
        .bind(post_id)
        .bind(body)
        .fetch_one(self.db.pool())
        .await?;

        Ok(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            post_id: row.get("post_id"),
            body: row.get("body"),
            created_at: row.get("created_at"),
            user_handle: Some(row.get("user_handle")),
            user_display_name: Some(row.get("user_display_name")),
        })
    }

    pub async fn unlike_post(&self, user_id: Uuid, post_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM likes WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_likes(
        &self,
        post_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Like>> {
        let rows = match cursor {
            Some((created_at, like_id)) => {
                sqlx::query(
                    "SELECT id, user_id, post_id, created_at \
                     FROM likes \
                     WHERE post_id = $1 \
                       AND (created_at < $2 OR (created_at = $2 AND id < $3)) \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $4",
                )
                .bind(post_id)
                .bind(created_at)
                .bind(like_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, user_id, post_id, created_at \
                     FROM likes \
                     WHERE post_id = $1 \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $2",
                )
                .bind(post_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut likes = Vec::with_capacity(rows.len());
        for row in rows {
            likes.push(Like {
                id: row.get("id"),
                user_id: row.get("user_id"),
                post_id: row.get("post_id"),
                created_at: row.get("created_at"),
            });
        }

        Ok(likes)
    }

    pub async fn list_comments(
        &self,
        post_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Comment>> {
        let rows = match cursor {
            Some((created_at, comment_id)) => {
                sqlx::query(
                    "SELECT c.id, c.user_id, c.post_id, c.body, c.created_at, \
                            u.handle AS user_handle, u.display_name AS user_display_name \
                     FROM comments c \
                     JOIN users u ON u.id = c.user_id \
                     WHERE c.post_id = $1 \
                       AND (c.created_at < $2 OR (c.created_at = $2 AND c.id < $3)) \
                     ORDER BY c.created_at DESC, c.id DESC \
                     LIMIT $4",
                )
                .bind(post_id)
                .bind(created_at)
                .bind(comment_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT c.id, c.user_id, c.post_id, c.body, c.created_at, \
                            u.handle AS user_handle, u.display_name AS user_display_name \
                     FROM comments c \
                     JOIN users u ON u.id = c.user_id \
                     WHERE c.post_id = $1 \
                     ORDER BY c.created_at DESC, c.id DESC \
                     LIMIT $2",
                )
                .bind(post_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut comments = Vec::with_capacity(rows.len());
        for row in rows {
            comments.push(Comment {
                id: row.get("id"),
                user_id: row.get("user_id"),
                post_id: row.get("post_id"),
                body: row.get("body"),
                created_at: row.get("created_at"),
                user_handle: Some(row.get("user_handle")),
                user_display_name: Some(row.get("user_display_name")),
            });
        }

        Ok(comments)
    }

    pub async fn delete_comment(
        &self,
        comment_id: Uuid,
        post_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM comments WHERE id = $1 AND post_id = $2 AND user_id = $3",
        )
        .bind(comment_id)
        .bind(post_id)
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
