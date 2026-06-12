use anyhow::Result;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
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

/// Cached first page of a home feed, including the pagination cursor so a
/// cache hit doesn't break "load more". The timestamp is stored as unix
/// nanoseconds for a lossless round trip.
#[derive(Serialize, Deserialize)]
struct CachedHomeFeed {
    posts: Vec<Post>,
    next_cursor_nanos: Option<i128>,
    next_cursor_id: Option<Uuid>,
}

impl CachedHomeFeed {
    fn from_page(posts: &[Post], next_cursor: Option<(OffsetDateTime, Uuid)>) -> Self {
        Self {
            posts: posts.to_vec(),
            next_cursor_nanos: next_cursor.map(|(ts, _)| ts.unix_timestamp_nanos()),
            next_cursor_id: next_cursor.map(|(_, id)| id),
        }
    }

    #[allow(clippy::type_complexity)]
    fn into_page(self) -> Option<(Vec<Post>, Option<(OffsetDateTime, Uuid)>)> {
        let next_cursor = match (self.next_cursor_nanos, self.next_cursor_id) {
            (Some(nanos), Some(id)) => {
                Some((OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()?, id))
            }
            (None, None) => None,
            _ => return None,
        };
        Some((self.posts, next_cursor))
    }
}

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
        // Only the first page is cached — it absorbs nearly all the load, and a
        // single key per user keeps invalidation complete and cheap.
        let should_cache = cursor.is_none();
        let cache_key = format!("feed:home:{}", user_id);
        let ttl = FEED_CACHE_TTL_SECONDS;

        if should_cache {
            let mut conn = self.cache.conn();
            match conn.get::<_, Option<String>>(&cache_key).await {
                Ok(Some(payload)) => {
                    if let Ok(cached) = serde_json::from_str::<CachedHomeFeed>(&payload) {
                        if let Some(page) = cached.into_page() {
                            return Ok(page);
                        }
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    warn!(error = ?err, "failed to read feed cache");
                }
            }
        }

        let limit_plus = limit + 1;
        let rows = match cursor {
            Some((created_at, post_id)) => {
                sqlx::query(
                    "SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name, \
                            u.avatar_key AS owner_avatar_key, \
                            COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                            p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
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
                            u.avatar_key AS owner_avatar_key, \
                            COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids, \
                            p.caption, p.visibility::text AS visibility, p.created_at \
                     FROM posts p \
                     JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL \
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
                media_ids: row.get("media_ids"),
                primary_media: None,
                caption: row.get("caption"),
                visibility,
                created_at: row.get("created_at"),
                owner_avatar_key: row.get("owner_avatar_key"),
                owner_avatar_url: None,
            });
        }

        let next_cursor = if posts.len() > limit as usize {
            posts.pop().map(|extra| (extra.created_at, extra.id))
        } else {
            None
        };

        if should_cache {
            let mut conn = self.cache.conn();
            if let Ok(payload) =
                serde_json::to_string(&CachedHomeFeed::from_page(&posts, next_cursor))
            {
                if let Err(err) = conn.set_ex::<_, _, ()>(&cache_key, payload, ttl).await {
                    warn!(error = ?err, "failed to write feed cache");
                }
            }
        }

        Ok((posts, next_cursor))
    }

    pub async fn refresh_home_feed(&self, user_id: Uuid) -> Result<()> {
        let cache_key = format!("feed:home:{}", user_id);
        let mut conn = self.cache.conn();
        if let Err(err) = conn.del::<_, ()>(&cache_key).await {
            warn!(error = ?err, user_id = %user_id, "failed to invalidate feed cache");
        }
        Ok(())
    }
}

