use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use pasetors::claims::{Claims, ClaimsValidationRules};
use pasetors::keys::SymmetricKey;
use pasetors::token::UntrustedToken;
use pasetors::{local, Local, version4::V4};
use sha2::{Digest, Sha256};
use sqlx::Row;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::user::User;
use crate::infra::db::Db;

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: OffsetDateTime,
    pub refresh_expires_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct AuthService {
    db: Db,
    access_key: [u8; 32],
    refresh_key: [u8; 32],
    access_ttl_minutes: u64,
    refresh_ttl_days: u64,
}

impl AuthService {
    pub fn new(
        db: Db,
        access_key: [u8; 32],
        refresh_key: [u8; 32],
        access_ttl_minutes: u64,
        refresh_ttl_days: u64,
    ) -> Self {
        Self {
            db,
            access_key,
            refresh_key,
            access_ttl_minutes,
            refresh_ttl_days,
        }
    }

    pub async fn signup(
        &self,
        handle: String,
        email: String,
        display_name: String,
        bio: Option<String>,
        avatar_key: Option<String>,
        password: String,
    ) -> Result<User> {
        let password_hash = hash_password(&password)?;
        let row = sqlx::query(
            "INSERT INTO users (handle, email, display_name, bio, avatar_key, password_hash) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, handle, email, display_name, bio, avatar_key, created_at",
        )
        .bind(handle)
        .bind(email)
        .bind(display_name)
        .bind(bio)
        .bind(avatar_key)
        .bind(password_hash)
        .fetch_one(self.db.pool())
        .await?;

        Ok(User {
            id: row.get("id"),
            handle: row.get("handle"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            bio: row.get("bio"),
            avatar_key: row.get("avatar_key"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn login(&self, identifier: &str, password: &str) -> Result<Option<TokenPair>> {
        let row = sqlx::query(
            "SELECT id, password_hash \
             FROM users WHERE email = $1 OR handle = $1",
        )
        .bind(identifier)
        .fetch_optional(self.db.pool())
        .await?;

        let row = match row {
            Some(row) => row,
            None => return Ok(None),
        };

        let user_id: Uuid = row.get("id");
        let password_hash: String = row.get("password_hash");
        if password_hash.is_empty() {
            return Ok(None);
        }

        if !verify_password(password, &password_hash)? {
            return Ok(None);
        }

        let tokens = self.issue_token_pair(user_id).await?;
        Ok(Some(tokens))
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Option<TokenPair>> {
        let (user_id, refresh_id) = match self.verify_refresh_token(refresh_token) {
            Ok((user_id, refresh_id)) => (user_id, refresh_id),
            Err(_) => return Ok(None),
        };
        let token_hash = hash_token(refresh_token);

        let mut tx = self.db.pool().begin().await?;
        let row = sqlx::query(
            "SELECT id \
             FROM refresh_tokens \
             WHERE id = $1 \
               AND user_id = $2 \
               AND token_hash = $3 \
               AND revoked_at IS NULL \
               AND expires_at > now()",
        )
        .bind(refresh_id)
        .bind(user_id)
        .bind(&token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        if row.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }

        let tokens = self.issue_token_pair_with_tx(user_id, &mut tx).await?;
        sqlx::query(
            "UPDATE refresh_tokens \
             SET revoked_at = now(), replaced_by = $1 \
             WHERE id = $2 AND revoked_at IS NULL",
        )
        .bind(tokens.refresh_id)
        .bind(refresh_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(tokens.pair))
    }

    pub async fn revoke_refresh_token(&self, refresh_token: &str) -> Result<bool> {
        let (user_id, refresh_id) = match self.verify_refresh_token(refresh_token) {
            Ok((user_id, refresh_id)) => (user_id, refresh_id),
            Err(_) => return Ok(false),
        };
        let token_hash = hash_token(refresh_token);

        let result = sqlx::query(
            "UPDATE refresh_tokens \
             SET revoked_at = now() \
             WHERE id = $1 AND user_id = $2 AND token_hash = $3 AND revoked_at IS NULL",
        )
        .bind(refresh_id)
        .bind(user_id)
        .bind(token_hash)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn authenticate_access_token(&self, token: &str) -> Result<Option<AuthSession>> {
        let claims = match self.decrypt_claims(token, self.access_key)? {
            Some(claims) => claims,
            None => return Ok(None),
        };
        if !has_token_type(&claims, "access") {
            return Ok(None);
        }
        let user_id = claim_uuid(&claims, "sub")?;
        Ok(Some(AuthSession { user_id }))
    }

    pub async fn get_current_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, handle, email, display_name, bio, avatar_key, created_at \
             FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await?;

        let user = row.map(|row| User {
            id: row.get("id"),
            handle: row.get("handle"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            bio: row.get("bio"),
            avatar_key: row.get("avatar_key"),
            created_at: row.get("created_at"),
        });

        Ok(user)
    }

    fn decrypt_claims(&self, token: &str, key_bytes: [u8; 32]) -> Result<Option<Claims>> {
        let key = SymmetricKey::<V4>::from(&key_bytes)?;
        let mut rules = ClaimsValidationRules::new();
        rules.validate_issuer_with("lumine");
        rules.validate_audience_with("lumine");

        let untrusted = match UntrustedToken::<Local, V4>::try_from(token) {
            Ok(token) => token,
            Err(_) => return Ok(None),
        };
        let trusted = match local::decrypt(&key, &untrusted, &rules, None, None) {
            Ok(token) => token,
            Err(_) => return Ok(None),
        };
        Ok(trusted.payload_claims().cloned())
    }

    fn build_access_claims(&self, user_id: Uuid) -> Result<(Claims, OffsetDateTime)> {
        let duration = std::time::Duration::from_secs(self.access_ttl_minutes * 60);
        let mut claims = Claims::new_expires_in(&duration)?;
        claims.issuer("lumine")?;
        claims.audience("lumine")?;
        claims.subject(&user_id.to_string())?;
        claims.add_additional("typ", "access")?;
        let expires_at = OffsetDateTime::now_utc() + Duration::minutes(self.access_ttl_minutes as i64);
        Ok((claims, expires_at))
    }

    fn build_refresh_claims(
        &self,
        user_id: Uuid,
        refresh_id: Uuid,
    ) -> Result<(Claims, OffsetDateTime)> {
        let duration = std::time::Duration::from_secs(self.refresh_ttl_days * 24 * 60 * 60);
        let mut claims = Claims::new_expires_in(&duration)?;
        claims.issuer("lumine")?;
        claims.audience("lumine")?;
        claims.subject(&user_id.to_string())?;
        claims.token_identifier(&refresh_id.to_string())?;
        claims.add_additional("typ", "refresh")?;
        let expires_at = OffsetDateTime::now_utc() + Duration::days(self.refresh_ttl_days as i64);
        Ok((claims, expires_at))
    }

    async fn issue_token_pair(&self, user_id: Uuid) -> Result<TokenPair> {
        let mut tx = self.db.pool().begin().await?;
        let tokens = self.issue_token_pair_with_tx(user_id, &mut tx).await?;
        tx.commit().await?;
        Ok(tokens.pair)
    }

    async fn issue_token_pair_with_tx(
        &self,
        user_id: Uuid,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<IssuedTokens> {
        let (access_claims, access_expires_at) = self.build_access_claims(user_id)?;
        let access_key = SymmetricKey::<V4>::from(&self.access_key)?;
        let access_token = local::encrypt(&access_key, &access_claims, None, None)?;

        let refresh_id = Uuid::new_v4();
        let (refresh_claims, refresh_expires_at) = self.build_refresh_claims(user_id, refresh_id)?;
        let refresh_key = SymmetricKey::<V4>::from(&self.refresh_key)?;
        let refresh_token = local::encrypt(&refresh_key, &refresh_claims, None, None)?;
        let token_hash = hash_token(&refresh_token);

        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(refresh_id)
        .bind(user_id)
        .bind(token_hash)
        .bind(refresh_expires_at)
        .execute(&mut **tx)
        .await?;

        Ok(IssuedTokens {
            refresh_id,
            pair: TokenPair {
                access_token,
                refresh_token,
                access_expires_at,
                refresh_expires_at,
            },
        })
    }

    fn verify_refresh_token(&self, token: &str) -> Result<(Uuid, Uuid)> {
        let claims = match self.decrypt_claims(token, self.refresh_key)? {
            Some(claims) => claims,
            None => return Err(anyhow!("invalid refresh token")),
        };
        if !has_token_type(&claims, "refresh") {
            return Err(anyhow!("invalid refresh token"));
        }
        let user_id = claim_uuid(&claims, "sub")?;
        let refresh_id = claim_uuid(&claims, "jti")?;
        Ok((user_id, refresh_id))
    }
}

struct IssuedTokens {
    refresh_id: Uuid,
    pair: TokenPair,
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow!("failed to hash password: {}", err))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|err| anyhow!("failed to parse password hash: {}", err))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn claim_uuid(claims: &Claims, name: &str) -> Result<Uuid> {
    let value = claims
        .get_claim(name)
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("missing {} claim", name))?;
    Ok(Uuid::parse_str(value)?)
}

fn has_token_type(claims: &Claims, expected: &str) -> bool {
    claims
        .get_claim("typ")
        .and_then(|value| value.as_str())
        .map(|value| value == expected)
        .unwrap_or(false)
}
