use anyhow::Result;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::post::{Post, PostVisibility};
use crate::domain::user::User;
use crate::infra::db::Db;

#[derive(Clone)]
pub struct SearchService {
    db: Db,
}

impl SearchService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn search_users(
        &self,
        query: &str,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<User>> {
        let pattern = format!("%{}%", query);
        let rows = match cursor {
            Some((created_at, user_id)) => {
                sqlx::query(
                    "SELECT id, handle, email, display_name, bio, avatar_key, created_at \
                     FROM users \
                     WHERE (handle ILIKE $1 OR display_name ILIKE $1) \
                       AND (created_at < $2 OR (created_at = $2 AND id < $3)) \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $4",
                )
                .bind(&pattern)
                .bind(created_at)
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, handle, email, display_name, bio, avatar_key, created_at \
                     FROM users \
                     WHERE handle ILIKE $1 OR display_name ILIKE $1 \
                     ORDER BY created_at DESC, id DESC \
                     LIMIT $2",
                )
                .bind(&pattern)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            users.push(User {
                id: row.get("id"),
                handle: row.get("handle"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                bio: row.get("bio"),
                avatar_key: row.get("avatar_key"),
                created_at: row.get("created_at"),
            });
        }

        Ok(users)
    }

    pub async fn search_posts(
        &self,
        query: &str,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let pattern = format!("%{}%", query);
        let rows = match cursor {
            Some((created_at, post_id)) => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            p.media_id, p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id \
                     WHERE p.visibility = 'public' \
                       AND p.caption ILIKE $1 \
                       AND (p.created_at < $2 OR (p.created_at = $2 AND p.id < $3)) \
                     ORDER BY p.created_at DESC, p.id DESC \
                     LIMIT $4",
                )
                .bind(&pattern)
                .bind(created_at)
                .bind(post_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            p.media_id, p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id \
                     WHERE p.visibility = 'public' AND p.caption ILIKE $1 \
                     ORDER BY p.created_at DESC, p.id DESC \
                     LIMIT $2",
                )
                .bind(&pattern)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let visibility: String = row.get("visibility");
            let visibility = PostVisibility::from_db(&visibility)
                .ok_or_else(|| anyhow::anyhow!("unknown post visibility: {}", visibility))?;
            posts.push(Post {
                id: row.get("id"),
                owner_id: row.get("owner_id"),
                owner_handle: Some(row.get("owner_handle")),
                owner_display_name: Some(row.get("owner_display_name")),
                media_id: row.get("media_id"),
                caption: row.get("caption"),
                visibility,
                created_at: row.get("created_at"),
            });
        }

        Ok(posts)
    }
}
