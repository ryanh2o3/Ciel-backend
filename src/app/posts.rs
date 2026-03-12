use anyhow::Result;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::post::{Post, PostVisibility};
use crate::infra::db::Db;

#[derive(Clone)]
pub struct PostService {
    db: Db,
}

impl PostService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn create_post(
        &self,
        owner_id: Uuid,
        media_ids: Vec<Uuid>,
        caption: Option<String>,
    ) -> Result<Post> {
        if media_ids.is_empty() {
            return Err(anyhow::anyhow!("at least one media_id is required"));
        }
        if media_ids.len() > 10 {
            return Err(anyhow::anyhow!("maximum 10 images per post"));
        }

        // Verify all media IDs belong to this user
        let owned_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media WHERE id = ANY($1) AND owner_id = $2",
        )
        .bind(&media_ids)
        .bind(owner_id)
        .fetch_one(self.db.pool())
        .await?;

        if owned_count != media_ids.len() as i64 {
            return Err(anyhow::anyhow!("media not found or not owned by user"));
        }

        let mut tx = self.db.pool().begin().await?;

        let row = sqlx::query(
            "WITH inserted_post AS ( \
                INSERT INTO posts (owner_id, caption, visibility) \
                VALUES ($1, $2, $3::post_visibility) \
                RETURNING id, owner_id, caption, visibility::text AS visibility, created_at \
             ) \
             SELECT p.*, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                    u.avatar_key AS owner_avatar_key \
             FROM inserted_post p \
             JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL",
        )
        .bind(owner_id)
        .bind(&caption)
        .bind(PostVisibility::Public.as_db())
        .fetch_one(&mut *tx)
        .await?;

        let post_id: Uuid = row.get("id");

        for (position, media_id) in media_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO post_media (post_id, media_id, position) VALUES ($1, $2, $3)",
            )
            .bind(post_id)
            .bind(media_id)
            .bind(position as i16)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let visibility: String = row.get("visibility");
        let visibility = PostVisibility::from_db(&visibility).ok_or_else(|| {
            anyhow::anyhow!("unknown post visibility: {}", visibility)
        })?;

        Ok(Post {
            id: row.get("id"),
            owner_id: row.get("owner_id"),
            owner_handle: Some(row.get("owner_handle")),
            owner_display_name: Some(row.get("owner_display_name")),
            media_ids,
            caption: row.get("caption"),
            visibility,
            created_at: row.get("created_at"),
            owner_avatar_key: row.get("owner_avatar_key"),
            owner_avatar_url: None,
        })
    }

    pub async fn get_post(&self, post_id: Uuid, viewer_id: Option<Uuid>) -> Result<Option<Post>> {
        let row = match viewer_id {
            Some(viewer_id) => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            u.avatar_key AS owner_avatar_key, \
                            COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                            p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                     WHERE p.id = $1 \
                       AND (p.visibility = 'public' \
                            OR p.owner_id = $2 \
                            OR (p.visibility = 'followers_only' AND EXISTS ( \
                                SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = p.owner_id \
                            ))) \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = p.owner_id AND blocked_id = $2) \
                              OR (blocker_id = $2 AND blocked_id = p.owner_id) \
                       )",
                )
                .bind(post_id)
                .bind(viewer_id)
                .fetch_optional(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            u.avatar_key AS owner_avatar_key, \
                            COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                            p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                     WHERE p.id = $1 AND p.visibility = 'public'",
                )
                .bind(post_id)
                .fetch_optional(self.db.pool())
                .await?
            }
        };

        let post = match row {
            Some(row) => {
                let visibility: String = row.get("visibility");
                let visibility = PostVisibility::from_db(&visibility)
                    .ok_or_else(|| anyhow::anyhow!("unknown post visibility: {}", visibility))?;
                Some(Post {
                    id: row.get("id"),
                    owner_id: row.get("owner_id"),
                    owner_handle: Some(row.get("owner_handle")),
                    owner_display_name: Some(row.get("owner_display_name")),
                    media_ids: row.get("media_ids"),
                    caption: row.get("caption"),
                    visibility,
                    created_at: row.get("created_at"),
                    owner_avatar_key: row.get("owner_avatar_key"),
                    owner_avatar_url: None,
                })
            }
            None => None,
        };

        Ok(post)
    }

    pub async fn update_caption(
        &self,
        post_id: Uuid,
        owner_id: Uuid,
        caption: Option<String>,
    ) -> Result<Option<Post>> {
        let row = sqlx::query(
            "WITH updated_post AS ( \
                UPDATE posts \
                SET caption = $3 \
                WHERE id = $1 AND owner_id = $2 \
                RETURNING id, owner_id, caption, visibility::text AS visibility, created_at \
             ) \
             SELECT p.*, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                    u.avatar_key AS owner_avatar_key, \
                    COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids \
             FROM updated_post p \
             JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL",
        )
        .bind(post_id)
        .bind(owner_id)
        .bind(caption)
        .fetch_optional(self.db.pool())
        .await?;

        let post = match row {
            Some(row) => {
                let visibility: String = row.get("visibility");
                let visibility = PostVisibility::from_db(&visibility)
                    .ok_or_else(|| anyhow::anyhow!("unknown post visibility: {}", visibility))?;
                Some(Post {
                    id: row.get("id"),
                    owner_id: row.get("owner_id"),
                    owner_handle: Some(row.get("owner_handle")),
                    owner_display_name: Some(row.get("owner_display_name")),
                    media_ids: row.get("media_ids"),
                    caption: row.get("caption"),
                    visibility,
                    created_at: row.get("created_at"),
                    owner_avatar_key: row.get("owner_avatar_key"),
                    owner_avatar_url: None,
                })
            }
            None => None,
        };

        Ok(post)
    }

    pub async fn delete_post(&self, post_id: Uuid, owner_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM posts WHERE id = $1 AND owner_id = $2")
            .bind(post_id)
            .bind(owner_id)
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_by_user(
        &self,
        owner_id: Uuid,
        viewer_id: Option<Uuid>,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Post>> {
        let rows = match viewer_id {
            Some(viewer_id) => match cursor {
                Some((created_at, post_id)) => {
                    sqlx::query(
                        "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                                u.avatar_key AS owner_avatar_key, \
                                COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                                p.caption, p.visibility::text AS visibility, p.created_at \
                         FROM posts p \
                         JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                         WHERE p.owner_id = $1 \
                           AND (p.visibility = 'public' \
                                OR p.owner_id = $2 \
                                OR (p.visibility = 'followers_only' AND EXISTS ( \
                                    SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = p.owner_id \
                                ))) \
                           AND NOT EXISTS ( \
                               SELECT 1 FROM blocks \
                               WHERE (blocker_id = p.owner_id AND blocked_id = $2) \
                                  OR (blocker_id = $2 AND blocked_id = p.owner_id) \
                           ) \
                           AND (p.created_at < $3 OR (p.created_at = $3 AND p.id < $4)) \
                         ORDER BY p.created_at DESC, p.id DESC \
                         LIMIT $5",
                    )
                    .bind(owner_id)
                    .bind(viewer_id)
                    .bind(created_at)
                    .bind(post_id)
                    .bind(limit)
                    .fetch_all(self.db.pool())
                    .await?
                }
                None => {
                    sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            u.avatar_key AS owner_avatar_key, \
                            COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                            p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                     WHERE p.owner_id = $1 \
                       AND (p.visibility = 'public' \
                            OR p.owner_id = $2 \
                            OR (p.visibility = 'followers_only' AND EXISTS ( \
                                SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = p.owner_id \
                            ))) \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = p.owner_id AND blocked_id = $2) \
                              OR (blocker_id = $2 AND blocked_id = p.owner_id) \
                       ) \
                     ORDER BY p.created_at DESC, p.id DESC \
                     LIMIT $3",
                    )
                    .bind(owner_id)
                    .bind(viewer_id)
                    .bind(limit)
                    .fetch_all(self.db.pool())
                    .await?
                }
            },
            None => match cursor {
                Some((created_at, post_id)) => {
                    sqlx::query(
                        "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                                u.avatar_key AS owner_avatar_key, \
                                COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                                p.caption, p.visibility::text AS visibility, p.created_at \
                         FROM posts p \
                         JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                         WHERE p.owner_id = $1 \
                           AND p.visibility = 'public' \
                           AND (p.created_at < $2 OR (p.created_at = $2 AND p.id < $3)) \
                         ORDER BY p.created_at DESC, p.id DESC \
                         LIMIT $4",
                    )
                    .bind(owner_id)
                    .bind(created_at)
                    .bind(post_id)
                    .bind(limit)
                    .fetch_all(self.db.pool())
                    .await?
                }
                None => {
                    sqlx::query(
                        "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                                u.avatar_key AS owner_avatar_key, \
                                COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                                p.caption, p.visibility::text AS visibility, p.created_at \
                         FROM posts p \
                         JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
                         WHERE p.owner_id = $1 AND p.visibility = 'public' \
                         ORDER BY p.created_at DESC, p.id DESC \
                         LIMIT $2",
                    )
                    .bind(owner_id)
                    .bind(limit)
                    .fetch_all(self.db.pool())
                    .await?
                }
            },
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
                media_ids: row.get("media_ids"),
                caption: row.get("caption"),
                visibility,
                created_at: row.get("created_at"),
                owner_avatar_key: row.get("owner_avatar_key"),
                owner_avatar_url: None,
            });
        }

        Ok(posts)
    }
}
