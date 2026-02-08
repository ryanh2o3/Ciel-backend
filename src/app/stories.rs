use anyhow::Result;
use redis::AsyncCommands;
use sqlx::postgres::PgRow;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::story::{
    EmojiCount, Story, StoryHighlight, StoryMetrics, StoryReaction, StoryView, StoryVisibility,
};
use crate::infra::{cache::RedisCache, db::Db};

const STORY_TTL_HOURS: u64 = 24;
const STORIES_FEED_CACHE_TTL: u64 = 60;

#[derive(Clone)]
pub struct StoryService {
    db: Db,
    cache: RedisCache,
}

impl StoryService {
    pub fn new(db: Db, cache: RedisCache) -> Self {
        Self { db, cache }
    }

    /// Verify that a story exists and is owned by the given user (no expiry filter —
    /// owners retain access to their own story metadata after expiry).
    pub async fn get_story_owner(&self, story_id: Uuid) -> Result<Option<Uuid>> {
        Ok(sqlx::query_scalar("SELECT user_id FROM stories WHERE id = $1")
            .bind(story_id)
            .fetch_optional(self.db.pool())
            .await?)
    }

    pub async fn create_story(
        &self,
        user_id: Uuid,
        media_id: Uuid,
        caption: Option<String>,
        visibility: StoryVisibility,
    ) -> Result<Story> {
        let media_owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT owner_id FROM media WHERE id = $1",
        )
        .bind(media_id)
        .fetch_optional(self.db.pool())
        .await?;

        match media_owner {
            Some(owner) if owner == user_id => {}
            Some(_) => return Err(anyhow::anyhow!("media does not belong to user")),
            None => return Err(anyhow::anyhow!("media not found")),
        }

        let expires_at =
            OffsetDateTime::now_utc() + std::time::Duration::from_secs(STORY_TTL_HOURS * 3600);

        let row = sqlx::query(
            "WITH inserted AS ( \
                INSERT INTO stories (user_id, media_id, caption, expires_at, visibility) \
                VALUES ($1, $2, $3, $4, $5::story_visibility) \
                RETURNING id, user_id, media_id, caption, created_at, expires_at, \
                         visibility::text AS visibility, view_count, reaction_count \
             ) \
             SELECT s.*, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key \
             FROM inserted s \
             JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(media_id)
        .bind(caption)
        .bind(expires_at)
        .bind(visibility.as_db())
        .fetch_one(self.db.pool())
        .await?;

        row_to_story(&row)
    }

    pub async fn get_user_stories(
        &self,
        user_id: Uuid,
        viewer_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Story>> {
        let now = OffsetDateTime::now_utc();
        let rows = match cursor {
            Some((created_at, story_id)) => {
                sqlx::query(
                    "SELECT s.id, s.user_id, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key, \
                            s.media_id, s.caption, s.created_at, s.expires_at, \
                            s.visibility::text AS visibility, s.view_count, s.reaction_count \
                     FROM stories s \
                     JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE s.user_id = $1 \
                       AND s.expires_at > $2 \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = s.user_id AND blocked_id = $3) \
                              OR (blocker_id = $3 AND blocked_id = s.user_id) \
                       ) \
                       AND (s.visibility = 'public' \
                            OR s.user_id = $3 \
                            OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = $3 AND followee_id = s.user_id) \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $3))) \
                       AND (s.created_at > $4 OR (s.created_at = $4 AND s.id > $5)) \
                     ORDER BY s.created_at ASC, s.id ASC \
                     LIMIT $6",
                )
                .bind(user_id)
                .bind(now)
                .bind(viewer_id)
                .bind(created_at)
                .bind(story_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT s.id, s.user_id, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key, \
                            s.media_id, s.caption, s.created_at, s.expires_at, \
                            s.visibility::text AS visibility, s.view_count, s.reaction_count \
                     FROM stories s \
                     JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE s.user_id = $1 \
                       AND s.expires_at > $2 \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = s.user_id AND blocked_id = $3) \
                              OR (blocker_id = $3 AND blocked_id = s.user_id) \
                       ) \
                       AND (s.visibility = 'public' \
                            OR s.user_id = $3 \
                            OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = $3 AND followee_id = s.user_id) \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $3))) \
                     ORDER BY s.created_at ASC, s.id ASC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(now)
                .bind(viewer_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        rows.iter().map(row_to_story).collect()
    }

    pub async fn get_story(&self, story_id: Uuid, viewer_id: Uuid) -> Result<Option<Story>> {
        let now = OffsetDateTime::now_utc();
        let row = sqlx::query(
            "SELECT s.id, s.user_id, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key, \
                    s.media_id, s.caption, s.created_at, s.expires_at, \
                    s.visibility::text AS visibility, s.view_count, s.reaction_count \
             FROM stories s \
             JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL \
             WHERE s.id = $1 \
               AND s.expires_at > $2 \
               AND NOT EXISTS ( \
                   SELECT 1 FROM blocks \
                   WHERE (blocker_id = s.user_id AND blocked_id = $3) \
                      OR (blocker_id = $3 AND blocked_id = s.user_id) \
               ) \
               AND (s.visibility = 'public' \
                    OR s.user_id = $3 \
                    OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                        AND EXISTS (SELECT 1 FROM follows WHERE follower_id = $3 AND followee_id = s.user_id) \
                        AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $3)))",
        )
        .bind(story_id)
        .bind(now)
        .bind(viewer_id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_story(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn delete_story(&self, story_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM stories WHERE id = $1 AND user_id = $2")
            .bind(story_id)
            .bind(user_id)
            .execute(self.db.pool())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Record a view. Returns true if this was the viewer's first view of the story.
    /// Uses a CTE to atomically insert the view record and increment the counter
    /// only when the insert succeeds (ON CONFLICT DO NOTHING skips duplicates).
    pub async fn mark_seen(&self, story_id: Uuid, viewer_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "WITH view_insert AS ( \
                INSERT INTO story_views (story_id, viewer_id) \
                VALUES ($1, $2) \
                ON CONFLICT DO NOTHING \
                RETURNING story_id \
             ) \
             UPDATE stories \
             SET view_count = view_count + 1 \
             WHERE id IN (SELECT story_id FROM view_insert)",
        )
        .bind(story_id)
        .bind(viewer_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Upsert a reaction. Each user may have at most one reaction per story;
    /// sending a new emoji replaces the previous one without changing the count.
    pub async fn add_reaction(
        &self,
        story_id: Uuid,
        user_id: Uuid,
        emoji: String,
    ) -> Result<StoryReaction> {
        // xmax = 0 is true only for a fresh INSERT (not an ON CONFLICT UPDATE).
        let row = sqlx::query(
            "WITH upserted AS ( \
                INSERT INTO story_reactions (story_id, user_id, emoji) \
                VALUES ($1, $2, $3) \
                ON CONFLICT (story_id, user_id) DO UPDATE SET emoji = EXCLUDED.emoji \
                RETURNING id, story_id, user_id, emoji, created_at, (xmax = 0) AS is_new \
             ), updated AS ( \
                UPDATE stories \
                SET reaction_count = reaction_count + CASE WHEN (SELECT is_new FROM upserted) THEN 1 ELSE 0 END \
                WHERE id = $1 \
                RETURNING id \
             ) \
             SELECT r.id, r.story_id, r.user_id, r.emoji, r.created_at, r.is_new, \
                    u.handle AS user_handle \
             FROM upserted r \
             JOIN users u ON r.user_id = u.id AND u.deleted_at IS NULL",
        )
        .bind(story_id)
        .bind(user_id)
        .bind(&emoji)
        .fetch_one(self.db.pool())
        .await?;

        Ok(StoryReaction {
            id: row.get("id"),
            story_id: row.get("story_id"),
            user_id: row.get("user_id"),
            user_handle: Some(row.get("user_handle")),
            emoji: row.get("emoji"),
            created_at: row.get("created_at"),
        })
    }

    /// Remove a reaction and decrement the counter atomically via CTE.
    pub async fn remove_reaction(&self, story_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "WITH deleted AS ( \
                DELETE FROM story_reactions \
                WHERE story_id = $1 AND user_id = $2 \
                RETURNING story_id \
             ) \
             UPDATE stories \
             SET reaction_count = reaction_count - 1 \
             WHERE id IN (SELECT story_id FROM deleted)",
        )
        .bind(story_id)
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_reactions(
        &self,
        story_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<StoryReaction>> {
        let rows = match cursor {
            Some((created_at, reaction_id)) => {
                sqlx::query(
                    "SELECT r.id, r.story_id, r.user_id, u.handle AS user_handle, r.emoji, r.created_at \
                     FROM story_reactions r \
                     JOIN users u ON r.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE r.story_id = $1 \
                       AND (r.created_at < $2 OR (r.created_at = $2 AND r.id < $3)) \
                     ORDER BY r.created_at DESC, r.id DESC \
                     LIMIT $4",
                )
                .bind(story_id)
                .bind(created_at)
                .bind(reaction_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT r.id, r.story_id, r.user_id, u.handle AS user_handle, r.emoji, r.created_at \
                     FROM story_reactions r \
                     JOIN users u ON r.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE r.story_id = $1 \
                     ORDER BY r.created_at DESC, r.id DESC \
                     LIMIT $2",
                )
                .bind(story_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut reactions = Vec::with_capacity(rows.len());
        for row in &rows {
            reactions.push(StoryReaction {
                id: row.get("id"),
                story_id: row.get("story_id"),
                user_id: row.get("user_id"),
                user_handle: Some(row.get("user_handle")),
                emoji: row.get("emoji"),
                created_at: row.get("created_at"),
            });
        }
        Ok(reactions)
    }

    pub async fn list_viewers(
        &self,
        story_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<StoryView>> {
        let rows = match cursor {
            Some((viewed_at, viewer_id)) => {
                sqlx::query(
                    "SELECT v.viewer_id, u.handle AS viewer_handle, \
                            u.display_name AS viewer_display_name, \
                            u.avatar_key AS viewer_avatar_key, v.viewed_at \
                     FROM story_views v \
                     JOIN users u ON v.viewer_id = u.id AND u.deleted_at IS NULL \
                     WHERE v.story_id = $1 \
                       AND (v.viewed_at < $2 OR (v.viewed_at = $2 AND v.viewer_id < $3)) \
                     ORDER BY v.viewed_at DESC, v.viewer_id DESC \
                     LIMIT $4",
                )
                .bind(story_id)
                .bind(viewed_at)
                .bind(viewer_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT v.viewer_id, u.handle AS viewer_handle, \
                            u.display_name AS viewer_display_name, \
                            u.avatar_key AS viewer_avatar_key, v.viewed_at \
                     FROM story_views v \
                     JOIN users u ON v.viewer_id = u.id AND u.deleted_at IS NULL \
                     WHERE v.story_id = $1 \
                     ORDER BY v.viewed_at DESC, v.viewer_id DESC \
                     LIMIT $2",
                )
                .bind(story_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut views = Vec::with_capacity(rows.len());
        for row in &rows {
            views.push(StoryView {
                viewer_id: row.get("viewer_id"),
                viewer_handle: Some(row.get("viewer_handle")),
                viewer_display_name: Some(row.get("viewer_display_name")),
                viewer_avatar_key: row.get("viewer_avatar_key"),
                viewer_avatar_url: None,
                viewed_at: row.get("viewed_at"),
            });
        }
        Ok(views)
    }

    /// Aggregate metrics for a story. Ownership is enforced by requiring user_id
    /// to match the story owner — returns None if the story doesn't belong to this user.
    pub async fn get_metrics(&self, story_id: Uuid, user_id: Uuid) -> Result<Option<StoryMetrics>> {
        let counts = sqlx::query(
            "SELECT view_count, reaction_count FROM stories WHERE id = $1 AND user_id = $2",
        )
        .bind(story_id)
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await?;

        let Some(counts) = counts else {
            return Ok(None);
        };

        let emoji_rows = sqlx::query(
            "SELECT emoji, COUNT(*) AS count \
             FROM story_reactions \
             WHERE story_id = $1 \
             GROUP BY emoji \
             ORDER BY count DESC",
        )
        .bind(story_id)
        .fetch_all(self.db.pool())
        .await?;

        let reactions_by_emoji: Vec<EmojiCount> = emoji_rows
            .iter()
            .map(|row| EmojiCount {
                emoji: row.get("emoji"),
                count: row.get("count"),
            })
            .collect();

        let viewer_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT viewer_id FROM story_views WHERE story_id = $1 ORDER BY viewed_at DESC LIMIT 200",
        )
        .bind(story_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(Some(StoryMetrics {
            story_id,
            view_count: counts.get("view_count"),
            reaction_count: counts.get("reaction_count"),
            reactions_by_emoji,
            viewer_ids,
        }))
    }

    /// Add a story to a named highlight collection, creating the highlight if it
    /// doesn't exist. The first story added becomes the cover automatically.
    pub async fn add_to_highlight(
        &self,
        user_id: Uuid,
        story_id: Uuid,
        highlight_name: String,
    ) -> Result<StoryHighlight> {
        // Verify story ownership (highlights can persist beyond story expiry)
        let story_owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM stories WHERE id = $1",
        )
        .bind(story_id)
        .fetch_optional(self.db.pool())
        .await?;

        match story_owner {
            Some(owner) if owner == user_id => {}
            _ => return Err(anyhow::anyhow!("story not found")),
        }

        // Upsert highlight; COALESCE keeps the existing cover if one is already set
        let highlight = sqlx::query(
            "INSERT INTO story_highlights (user_id, name, cover_story_id) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (user_id, name) DO UPDATE \
             SET updated_at = now(), \
                 cover_story_id = COALESCE(story_highlights.cover_story_id, EXCLUDED.cover_story_id) \
             RETURNING id, user_id, name, cover_story_id, created_at, updated_at",
        )
        .bind(user_id)
        .bind(&highlight_name)
        .bind(story_id)
        .fetch_one(self.db.pool())
        .await?;

        let highlight_id: Uuid = highlight.get("id");

        // Determine insertion position (append after the current max)
        let max_pos: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(position) FROM story_highlight_items WHERE highlight_id = $1",
        )
        .bind(highlight_id)
        .fetch_one(self.db.pool())
        .await?;

        let next_pos = max_pos.map_or(0, |p| p + 1);

        sqlx::query(
            "INSERT INTO story_highlight_items (highlight_id, story_id, position) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (highlight_id, story_id) DO NOTHING",
        )
        .bind(highlight_id)
        .bind(story_id)
        .bind(next_pos)
        .execute(self.db.pool())
        .await?;

        Ok(StoryHighlight {
            id: highlight.get("id"),
            user_id: highlight.get("user_id"),
            name: highlight.get("name"),
            cover_story_id: highlight.get("cover_story_id"),
            created_at: highlight.get("created_at"),
            updated_at: highlight.get("updated_at"),
        })
    }

    pub async fn get_user_highlights(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<StoryHighlight>> {
        let rows = match cursor {
            Some((created_at, highlight_id)) => {
                sqlx::query(
                    "SELECT id, user_id, name, cover_story_id, created_at, updated_at \
                     FROM story_highlights \
                     WHERE user_id = $1 \
                       AND (created_at > $2 OR (created_at = $2 AND id > $3)) \
                     ORDER BY created_at ASC, id ASC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(created_at)
                .bind(highlight_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, user_id, name, cover_story_id, created_at, updated_at \
                     FROM story_highlights \
                     WHERE user_id = $1 \
                     ORDER BY created_at ASC, id ASC \
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut highlights = Vec::with_capacity(rows.len());
        for row in &rows {
            highlights.push(StoryHighlight {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                cover_story_id: row.get("cover_story_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }
        Ok(highlights)
    }

    /// Fan-out-on-read stories feed: active stories from followed users, filtered
    /// by visibility and blocks. Results are cached for a short window.
    pub async fn get_stories_feed(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<Story>> {
        let should_cache = cursor.is_none();
        let cache_key = format!("feed:stories:{}:{}", user_id, limit);

        if should_cache {
            match self.cache.client().get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    match conn.get::<_, Option<String>>(&cache_key).await {
                        Ok(Some(payload)) => {
                            if let Ok(stories) = serde_json::from_str::<Vec<Story>>(&payload) {
                                return Ok(stories);
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            tracing::warn!(error = ?err, "failed to read stories feed cache");
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "failed to connect to Redis for stories feed cache");
                }
            }
        }

        let now = OffsetDateTime::now_utc();
        let rows = match cursor {
            Some((created_at, story_id)) => {
                sqlx::query(
                    "SELECT s.id, s.user_id, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key, \
                            s.media_id, s.caption, s.created_at, s.expires_at, \
                            s.visibility::text AS visibility, s.view_count, s.reaction_count \
                     FROM stories s \
                     JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE s.expires_at > $1 \
                       AND s.user_id IN (SELECT followee_id FROM follows WHERE follower_id = $2) \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = s.user_id AND blocked_id = $2) \
                              OR (blocker_id = $2 AND blocked_id = s.user_id) \
                       ) \
                       AND (s.visibility = 'public' \
                            OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $2))) \
                       AND (s.created_at < $3 OR (s.created_at = $3 AND s.id < $4)) \
                     ORDER BY s.created_at DESC, s.id DESC \
                     LIMIT $5",
                )
                .bind(now)
                .bind(user_id)
                .bind(created_at)
                .bind(story_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT s.id, s.user_id, u.handle AS user_handle, u.display_name AS user_display_name, \
                           u.avatar_key AS user_avatar_key, \
                            s.media_id, s.caption, s.created_at, s.expires_at, \
                            s.visibility::text AS visibility, s.view_count, s.reaction_count \
                     FROM stories s \
                     JOIN users u ON s.user_id = u.id AND u.deleted_at IS NULL \
                     WHERE s.expires_at > $1 \
                       AND s.user_id IN (SELECT followee_id FROM follows WHERE follower_id = $2) \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM blocks \
                           WHERE (blocker_id = s.user_id AND blocked_id = $2) \
                              OR (blocker_id = $2 AND blocked_id = s.user_id) \
                       ) \
                       AND (s.visibility = 'public' \
                            OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                                AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $2))) \
                     ORDER BY s.created_at DESC, s.id DESC \
                     LIMIT $3",
                )
                .bind(now)
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut stories = Vec::with_capacity(rows.len());
        for row in &rows {
            stories.push(row_to_story(row)?);
        }

        if should_cache {
            if let Ok(mut conn) = self.cache.client().get_multiplexed_async_connection().await {
                if let Ok(payload) = serde_json::to_string(&stories) {
                    if let Err(err) = conn
                        .set_ex::<_, _, ()>(&cache_key, payload, STORIES_FEED_CACHE_TTL)
                        .await
                    {
                        tracing::warn!(error = ?err, "failed to write stories feed cache");
                    }
                }
            }
        }

        Ok(stories)
    }
}

fn row_to_story(row: &PgRow) -> Result<Story> {
    let visibility: String = row.get("visibility");
    let visibility = StoryVisibility::from_db(&visibility)
        .ok_or_else(|| anyhow::anyhow!("unknown story visibility: {}", visibility))?;

    Ok(Story {
        id: row.get("id"),
        user_id: row.get("user_id"),
        user_handle: Some(row.get("user_handle")),
        user_display_name: Some(row.get("user_display_name")),
        user_avatar_key: row.get("user_avatar_key"),
        user_avatar_url: None,
        media_id: row.get("media_id"),
        caption: row.get("caption"),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        visibility,
        view_count: row.get("view_count"),
        reaction_count: row.get("reaction_count"),
    })
}
