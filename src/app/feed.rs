use anyhow::Result;
use redis::AsyncCommands;
use serde_json;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;
use tracing::warn;

use crate::domain::post::Post;
use crate::domain::post::PostVisibility;
use crate::infra::{cache::RedisCache, db::Db};

#[derive(Clone)]
pub struct FeedService {
    db: Db,
    cache: RedisCache,
}

const FEED_CACHE_TTL_SECONDS: u64 = 30;

impl FeedService {
    pub fn new(db: Db, cache: RedisCache) -> Self {
        Self { db, cache }
    }

    pub async fn get_home_feed(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<(Vec<Post>, Option<(OffsetDateTime, Uuid)>)> {
        // Fan-out on read: query recent posts from followed accounts and cache by user.
        // Cache is short-lived to keep freshness while absorbing spikes.
        let cache_key = match cursor {
            Some((created_at, id)) => format!("feed:home:{}:{}:{}", user_id, created_at, id),
            None => format!("feed:home:{}", user_id),
        };
        let ttl = FEED_CACHE_TTL_SECONDS;

        if let Ok(mut conn) = self.cache.client().get_multiplexed_async_connection().await {
            if let Ok(Some(payload)) = conn.get::<_, Option<String>>(&cache_key).await {
                if let Ok(posts) = serde_json::from_str::<Vec<Post>>(&payload) {
                    return Ok((posts, None));
                }
            }
        }

        let limit_plus = limit + 1;
        let rows = match cursor {
            Some((created_at, post_id)) => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            p.media_id, p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id \
                     WHERE (p.owner_id = $1 \
                        OR (p.owner_id IN ( \
                            SELECT followee_id FROM follows WHERE follower_id = $1 \
                        ) AND NOT EXISTS ( \
                            SELECT 1 FROM blocks \
                            WHERE (blocker_id = p.owner_id AND blocked_id = $1) \
                               OR (blocker_id = $1 AND blocked_id = p.owner_id) \
                        ))) \
                       AND (p.created_at < $2 OR (p.created_at = $2 AND p.id < $3)) \
                     ORDER BY p.created_at DESC, p.id DESC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(created_at)
                .bind(post_id)
                .bind(limit_plus)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            p.media_id, p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id \
                     WHERE p.owner_id = $1 \
                        OR (p.owner_id IN ( \
                            SELECT followee_id FROM follows WHERE follower_id = $1 \
                        ) AND NOT EXISTS ( \
                            SELECT 1 FROM blocks \
                            WHERE (blocker_id = p.owner_id AND blocked_id = $1) \
                               OR (blocker_id = $1 AND blocked_id = p.owner_id) \
                        )) \
                     ORDER BY p.created_at DESC, p.id DESC \
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit_plus)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let visibility: String = row.get("visibility");
            let visibility = PostVisibility::from_db(&visibility).ok_or_else(|| {
                anyhow::anyhow!("unknown post visibility: {}", visibility)
            })?;

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

        let next_cursor = if posts.len() > limit as usize {
            let extra = posts.pop().expect("checked len");
            Some((extra.created_at, extra.id))
        } else {
            None
        };

        if let Ok(mut conn) = self.cache.client().get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(&posts) {
                if let Err(err) = conn.set_ex::<_, _, ()>(&cache_key, payload, ttl).await {
                    warn!(error = ?err, "failed to write feed cache");
                }
            }
        }

        Ok((posts, next_cursor))
    }

    pub async fn refresh_home_feed(&self, user_id: Uuid) -> Result<()> {
        let cache_key = format!("feed:home:{}", user_id);
        if let Ok(mut conn) = self.cache.client().get_multiplexed_async_connection().await {
            let _ = conn.del::<_, ()>(&cache_key).await;
        }
        Ok(())
    }
}

