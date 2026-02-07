use anyhow::{anyhow, Result};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::infra::db::Db;

#[derive(Clone)]
pub struct InviteService {
    db: Db,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCode {
    pub code: String,
    pub created_by: Uuid,
    pub used_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub is_valid: bool,
    pub invite_type: String,
    pub use_count: i32,
    pub max_uses: i32,
}

impl InviteService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Generate a new invite code
    pub async fn create_invite(
        &self,
        user_id: Uuid,
        days_valid: i64,
    ) -> Result<InviteCode> {
        let mut tx = self.db.pool().begin().await?;

        // Check user's invite quota
        let quota_check = sqlx::query(
            "SELECT trust_level, invites_sent, successful_invites \
             FROM user_trust_scores \
             WHERE user_id = $1 \
             FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (trust_level, invites_sent, _successful_invites) = match quota_check {
            Some(row) => (
                row.get::<i32, _>("trust_level"),
                row.get::<i32, _>("invites_sent"),
                row.get::<i32, _>("successful_invites"),
            ),
            None => return Err(anyhow!("User trust score not found")),
        };

        // Calculate max invites based on trust level
        let max_invites = match trust_level {
            0 => 3,   // New users: 3 invites
            1 => 10,  // Basic: 10 invites
            2 => 50,  // Trusted: 50 invites
            3 => 200, // Verified: 200 invites
            _ => 3,
        };

        if invites_sent >= max_invites {
            return Err(anyhow!(
                "Maximum invite limit reached for your trust level ({})",
                max_invites
            ));
        }

        // Generate unique code
        let code = self.generate_unique_code().await?;

        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(days_valid);

        sqlx::query(
            "INSERT INTO invite_codes (code, created_by, expires_at) \
             VALUES ($1, $2, $3)",
        )
        .bind(&code)
        .bind(user_id)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;

        // Update invite count
        sqlx::query(
            "UPDATE user_trust_scores \
             SET invites_sent = invites_sent + 1 \
             WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::info!(
            user_id = %user_id,
            code = &code,
            "Invite code created"
        );

        Ok(InviteCode {
            code,
            created_by: user_id,
            used_by: None,
            created_at: OffsetDateTime::now_utc(),
            used_at: None,
            expires_at,
            is_valid: true,
            invite_type: "standard".to_string(),
            use_count: 0,
            max_uses: 1,
        })
    }

    /// Generate a unique invite code
    async fn generate_unique_code(&self) -> Result<String> {
        for _ in 0..10 {
            // Try up to 10 times
            let candidate: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(12)
                .map(char::from)
                .collect::<String>()
                .to_uppercase();

            // Check uniqueness
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM invite_codes WHERE code = $1)",
            )
            .bind(&candidate)
            .fetch_one(self.db.pool())
            .await?;

            if !exists {
                return Ok(candidate);
            }
        }

        Err(anyhow!("Failed to generate unique invite code after 10 attempts"))
    }

    /// Validate and consume an invite code during signup
    #[allow(dead_code)]
    pub async fn consume_invite(&self, code: &str, new_user_id: Uuid) -> Result<Uuid> {
        let mut tx = self.db.pool().begin().await?;
        let created_by = self.consume_invite_with_tx(code, new_user_id, &mut tx).await?;
        tx.commit().await?;
        Ok(created_by)
    }

    pub async fn consume_invite_with_tx(
        &self,
        code: &str,
        new_user_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Uuid> {
        // Fetch and validate invite with row lock
        let row = sqlx::query(
            "SELECT code, created_by, is_valid, expires_at, use_count, max_uses \
             FROM invite_codes \
             WHERE code = $1 \
             FOR UPDATE",
        )
        .bind(code)
        .fetch_optional(&mut **tx)
        .await?;

        let row = row.ok_or_else(|| anyhow!("Invalid invite code"))?;

        let is_valid: bool = row.get("is_valid");
        let expires_at: OffsetDateTime = row.get("expires_at");
        let use_count: i32 = row.get("use_count");
        let max_uses: i32 = row.get("max_uses");
        let created_by: Uuid = row.get("created_by");

        // Validation checks
        if !is_valid {
            return Err(anyhow!("Invite code has been revoked"));
        }

        if expires_at < OffsetDateTime::now_utc() {
            return Err(anyhow!("Invite code has expired"));
        }

        if use_count >= max_uses {
            return Err(anyhow!("Invite code has been fully used"));
        }

        // Mark as used
        let is_fully_used = use_count + 1 >= max_uses;
        sqlx::query(
            "UPDATE invite_codes \
             SET used_by = $1, \
                 used_at = NOW(), \
                 use_count = use_count + 1, \
                 is_valid = $2 \
             WHERE code = $3",
        )
        .bind(new_user_id)
        .bind(!is_fully_used) // Mark invalid if fully used
        .bind(code)
        .execute(&mut **tx)
        .await?;

        // Record relationship
        sqlx::query(
            "INSERT INTO invite_relationships (inviter_id, invitee_id, invite_code) \
             VALUES ($1, $2, $3)",
        )
        .bind(created_by)
        .bind(new_user_id)
        .bind(code)
        .execute(&mut **tx)
        .await?;

        // Update inviter's successful invite count and reward with trust points
        sqlx::query(
            "UPDATE user_trust_scores \
             SET successful_invites = successful_invites + 1, \
                 trust_points = trust_points + 10 \
             WHERE user_id = $1",
        )
        .bind(created_by)
        .execute(&mut **tx)
        .await?;

        tracing::info!(
            inviter_id = %created_by,
            invitee_id = %new_user_id,
            code = code,
            "Invite code consumed"
        );

        Ok(created_by)
    }

    /// Get user's invite codes
    pub async fn list_user_invites(&self, user_id: Uuid) -> Result<Vec<InviteCode>> {
        let rows = sqlx::query(
            "SELECT code, created_by, used_by, created_at, used_at, expires_at, \
                    is_valid, invite_type, use_count, max_uses \
             FROM invite_codes \
             WHERE created_by = $1 \
             ORDER BY created_at DESC \
             LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(self.db.pool())
        .await?;

        let invites = rows
            .into_iter()
            .map(|row| InviteCode {
                code: row.get("code"),
                created_by: row.get("created_by"),
                used_by: row.get("used_by"),
                created_at: row.get("created_at"),
                used_at: row.get("used_at"),
                expires_at: row.get("expires_at"),
                is_valid: row.get("is_valid"),
                invite_type: row.get("invite_type"),
                use_count: row.get("use_count"),
                max_uses: row.get("max_uses"),
            })
            .collect();

        Ok(invites)
    }

    /// Get invite statistics for a user
    pub async fn get_invite_stats(&self, user_id: Uuid) -> Result<InviteStats> {
        let row = sqlx::query(
            "SELECT invites_sent, successful_invites, trust_level \
             FROM user_trust_scores \
             WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await?;

        let invites_sent: i32 = row.get("invites_sent");
        let successful_invites: i32 = row.get("successful_invites");
        let trust_level: i32 = row.get("trust_level");

        let max_invites = match trust_level {
            0 => 3,
            1 => 10,
            2 => 50,
            3 => 200,
            _ => 3,
        };

        let remaining = max_invites - invites_sent;

        Ok(InviteStats {
            invites_sent,
            successful_invites,
            remaining_invites: remaining.max(0),
            max_invites,
        })
    }

    /// Revoke an invite code (mark as invalid)
    pub async fn revoke_invite(&self, code: &str, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE invite_codes \
             SET is_valid = FALSE \
             WHERE code = $1 AND created_by = $2 AND is_valid = TRUE",
        )
        .bind(code)
        .bind(user_id)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get invite tree (who invited whom) for a user
    #[allow(dead_code)]
    pub async fn get_invite_tree(&self, user_id: Uuid, depth: i32) -> Result<Vec<InviteRelationship>> {
        let mut relationships = Vec::new();
        self.get_invite_tree_recursive(user_id, depth, &mut relationships).await?;
        Ok(relationships)
    }

    #[allow(dead_code)]
    fn get_invite_tree_recursive<'a>(
        &'a self,
        user_id: Uuid,
        depth: i32,
        relationships: &'a mut Vec<InviteRelationship>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if depth <= 0 {
                return Ok(());
            }

            let rows = sqlx::query(
                "SELECT inviter_id, invitee_id, invite_code, invited_at \
                 FROM invite_relationships \
                 WHERE inviter_id = $1",
            )
            .bind(user_id)
            .fetch_all(self.db.pool())
            .await?;

            for row in rows {
                let invitee_id: Uuid = row.get("invitee_id");
                relationships.push(InviteRelationship {
                    inviter_id: row.get("inviter_id"),
                    invitee_id,
                    invite_code: row.get("invite_code"),
                    invited_at: row.get("invited_at"),
                });

                // Recurse for children
                self.get_invite_tree_recursive(invitee_id, depth - 1, relationships).await?;
            }

            Ok(())
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InviteStats {
    pub invites_sent: i32,
    pub successful_invites: i32,
    pub remaining_invites: i32,
    pub max_invites: i32,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct InviteRelationship {
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub invite_code: String,
    #[serde(with = "time::serde::rfc3339")]
    pub invited_at: OffsetDateTime,
}
