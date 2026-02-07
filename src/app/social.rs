use anyhow::Result;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::user::User;
use crate::infra::db::Db;

#[derive(Clone)]
pub struct SocialService {
    db: Db,
}

#[derive(Debug, Clone)]
pub struct SocialUserEdge {
    pub user: User,
    pub followed_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct RelationshipStatus {
    pub is_following: bool,
    pub is_followed_by: bool,
    pub is_blocking: bool,
    pub is_blocked_by: bool,
}

impl SocialService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn follow(&self, follower_id: Uuid, followee_id: Uuid) -> Result<bool> {
        const MAX_FOLLOWERS: i64 = 5000;

        let mut tx = self.db.pool().begin().await?;

        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(followee_id)
            .fetch_one(&mut *tx)
            .await?;

        let follower_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM follows WHERE followee_id = $1",
        )
        .bind(followee_id)
        .fetch_one(&mut *tx)
        .await?;

        if follower_count >= MAX_FOLLOWERS {
            tx.rollback().await?;
            return Err(anyhow::anyhow!("follower limit reached"));
        }

        let result = sqlx::query(
            "INSERT INTO follows (follower_id, followee_id) \
             SELECT $1, $2 \
             WHERE $1 <> $2 \
               AND NOT EXISTS ( \
                   SELECT 1 FROM blocks \
                   WHERE (blocker_id = $1 AND blocked_id = $2) \
                      OR (blocker_id = $2 AND blocked_id = $1) \
               ) \
             ON CONFLICT DO NOTHING",
        )
        .bind(follower_id)
        .bind(followee_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn unfollow(&self, follower_id: Uuid, followee_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM follows WHERE follower_id = $1 AND followee_id = $2",
        )
        .bind(follower_id)
        .bind(followee_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn block(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<bool> {
        let mut tx = self.db.pool().begin().await?;

        let inserted = sqlx::query(
            "INSERT INTO blocks (blocker_id, blocked_id) \
             SELECT $1, $2 \
             WHERE $1 <> $2 \
             ON CONFLICT DO NOTHING",
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "DELETE FROM follows \
             WHERE (follower_id = $1 AND followee_id = $2) \
                OR (follower_id = $2 AND followee_id = $1)",
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(inserted.rows_affected() > 0)
    }

    pub async fn unblock(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM blocks WHERE blocker_id = $1 AND blocked_id = $2",
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_followers(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<SocialUserEdge>> {
        let rows = match cursor {
            Some((created_at, follower_id)) => {
                sqlx::query(
                    "SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, \
                            u.created_at, f.created_at AS followed_at \
                     FROM follows f \
                     JOIN users u ON u.id = f.follower_id \
                     WHERE f.followee_id = $1 \
                       AND (f.created_at < $2 OR (f.created_at = $2 AND f.follower_id < $3)) \
                     ORDER BY f.created_at DESC, f.follower_id DESC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(created_at)
                .bind(follower_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, \
                            u.created_at, f.created_at AS followed_at \
                     FROM follows f \
                     JOIN users u ON u.id = f.follower_id \
                     WHERE f.followee_id = $1 \
                     ORDER BY f.created_at DESC, f.follower_id DESC \
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user = User {
                id: row.get("id"),
                handle: row.get("handle"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                bio: row.get("bio"),
                avatar_key: row.get("avatar_key"),
                avatar_url: None,
                created_at: row.get("created_at"),
            };
            items.push(SocialUserEdge {
                user,
                followed_at: row.get("followed_at"),
            });
        }

        Ok(items)
    }

    pub async fn list_following(
        &self,
        user_id: Uuid,
        cursor: Option<(OffsetDateTime, Uuid)>,
        limit: i64,
    ) -> Result<Vec<SocialUserEdge>> {
        let rows = match cursor {
            Some((created_at, followee_id)) => {
                sqlx::query(
                    "SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, \
                            u.created_at, f.created_at AS followed_at \
                     FROM follows f \
                     JOIN users u ON u.id = f.followee_id \
                     WHERE f.follower_id = $1 \
                       AND (f.created_at < $2 OR (f.created_at = $2 AND f.followee_id < $3)) \
                     ORDER BY f.created_at DESC, f.followee_id DESC \
                     LIMIT $4",
                )
                .bind(user_id)
                .bind(created_at)
                .bind(followee_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, \
                            u.created_at, f.created_at AS followed_at \
                     FROM follows f \
                     JOIN users u ON u.id = f.followee_id \
                     WHERE f.follower_id = $1 \
                     ORDER BY f.created_at DESC, f.followee_id DESC \
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(self.db.pool())
                .await?
            }
        };

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let user = User {
                id: row.get("id"),
                handle: row.get("handle"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                bio: row.get("bio"),
                avatar_key: row.get("avatar_key"),
                avatar_url: None,
                created_at: row.get("created_at"),
            };
            items.push(SocialUserEdge {
                user,
                followed_at: row.get("followed_at"),
            });
        }

        Ok(items)
    }

    pub async fn relationship_status(
        &self,
        viewer_id: Uuid,
        other_id: Uuid,
    ) -> Result<RelationshipStatus> {
        let row = sqlx::query(
            "SELECT \
                EXISTS (SELECT 1 FROM follows WHERE follower_id = $1 AND followee_id = $2) AS is_following, \
                EXISTS (SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = $1) AS is_followed_by, \
                EXISTS (SELECT 1 FROM blocks WHERE blocker_id = $1 AND blocked_id = $2) AS is_blocking, \
                EXISTS (SELECT 1 FROM blocks WHERE blocker_id = $2 AND blocked_id = $1) AS is_blocked_by",
        )
        .bind(viewer_id)
        .bind(other_id)
        .fetch_one(self.db.pool())
        .await?;

        Ok(RelationshipStatus {
            is_following: row.get("is_following"),
            is_followed_by: row.get("is_followed_by"),
            is_blocking: row.get("is_blocking"),
            is_blocked_by: row.get("is_blocked_by"),
        })
    }
}
