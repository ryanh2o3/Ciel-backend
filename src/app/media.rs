use anyhow::{anyhow, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use serde::Serialize;
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;
use url::Url;

use crate::domain::media::Media;
use crate::infra::{db::Db, queue::QueueClient, storage::ObjectStorage};
use crate::jobs::media_processor::MediaJob;

#[derive(Clone)]
pub struct MediaService {
    db: Db,
    storage: ObjectStorage,
    queue: QueueClient,
    s3_public_endpoint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadIntent {
    pub upload_id: Uuid,
    pub object_key: String,
    pub upload_url: String,
    pub expires_in_seconds: u64,
    pub headers: Vec<UploadHeader>,
}

#[derive(Debug, Serialize)]
pub struct UploadHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct UploadStatus {
    pub status: String,
    pub processed_media_id: Option<Uuid>,
}

impl MediaService {
    pub fn new(db: Db, storage: ObjectStorage, queue: QueueClient, s3_public_endpoint: Option<String>) -> Self {
        Self { db, storage, queue, s3_public_endpoint }
    }

    pub async fn create_upload(
        &self,
        owner_id: Uuid,
        content_type: String,
        bytes: i64,
        expires_in_seconds: u64,
    ) -> Result<UploadIntent> {
        let ext = extension_from_content_type(&content_type)?;
        let upload_id = Uuid::new_v4();
        let object_key = format!("uploads/{}/{}.{}", owner_id, upload_id, ext);

        sqlx::query(
            "INSERT INTO media_uploads (id, owner_id, original_key, content_type, bytes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(upload_id)
        .bind(owner_id)
        .bind(&object_key)
        .bind(&content_type)
        .bind(bytes)
        .execute(self.db.pool())
        .await?;

        let presign_config = PresigningConfig::expires_in(Duration::from_secs(expires_in_seconds))?;
        let presigned = self
            .storage
            .client()
            .put_object()
            .bucket(self.storage.bucket())
            .key(&object_key)
            .content_type(content_type)
            .content_length(bytes)
            .presigned(presign_config)
            .await?;

        let headers = presigned
            .headers()
            .map(|(name, value)| UploadHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect();

        let mut upload_url = presigned.uri().to_string();

        if let Some(ref public_endpoint) = self.s3_public_endpoint {
            match rewrite_presigned_url(&upload_url, public_endpoint) {
                Ok(rewritten) => upload_url = rewritten,
                Err(err) => tracing::warn!(error = ?err, "failed to rewrite presigned upload URL"),
            }
        }

        Ok(UploadIntent {
            upload_id,
            object_key,
            upload_url,
            expires_in_seconds,
            headers,
        })
    }

    pub async fn complete_upload(&self, upload_id: Uuid, owner_id: Uuid) -> Result<bool> {
        let row = sqlx::query(
            "UPDATE media_uploads \
             SET status = 'uploaded', uploaded_at = now() \
             WHERE id = $1 AND owner_id = $2 AND status = 'pending' \
             RETURNING original_key",
        )
        .bind(upload_id)
        .bind(owner_id)
        .fetch_optional(self.db.pool())
        .await?;

        let original_key: String = match row {
            Some(row) => row.get("original_key"),
            None => return Ok(false),
        };

        self.enqueue_processing(upload_id, owner_id, original_key).await?;
        Ok(true)
    }

    pub async fn enqueue_processing(
        &self,
        upload_id: Uuid,
        owner_id: Uuid,
        original_key: String,
    ) -> Result<()> {
        let job = MediaJob {
            upload_id,
            owner_id,
            original_key,
        };

        self.queue.enqueue_media_job(&job).await?;
        Ok(())
    }

    pub async fn get_media(&self, media_id: Uuid) -> Result<Option<Media>> {
        let row = sqlx::query(
            "SELECT id, owner_id, original_key, thumb_key, medium_key, width, height, bytes, created_at \
             FROM media WHERE id = $1",
        )
        .bind(media_id)
        .fetch_optional(self.db.pool())
        .await?;

        let media = match row {
            Some(row) => {
                let original_key: String = row.get("original_key");
                let thumb_key: String = row.get("thumb_key");
                let medium_key: String = row.get("medium_key");

                let mut media = Media {
                    id: row.get("id"),
                    owner_id: row.get("owner_id"),
                    original_key: original_key.clone(),
                    thumb_key: thumb_key.clone(),
                    medium_key: medium_key.clone(),
                    width: row.get("width"),
                    height: row.get("height"),
                    bytes: row.get("bytes"),
                    created_at: row.get("created_at"),
                    thumb_url: None,
                    medium_url: None,
                    original_url: None,
                };

                // Generate presigned URLs
                let presign_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;
                
                let keys = [
                    (&original_key, &mut media.original_url),
                    (&thumb_key, &mut media.thumb_url),
                    (&medium_key, &mut media.medium_url),
                ];

                for (key, url_field) in keys {
                    let presigned = self.storage.client()
                        .get_object()
                        .bucket(self.storage.bucket())
                        .key(key.clone())
                        .presigned(presign_config.clone())
                        .await?;
                    
                    let mut url = presigned.uri().to_string();
                    if let Some(ref public_endpoint) = self.s3_public_endpoint {
                        if let Ok(rewritten) = rewrite_presigned_url(&url, public_endpoint) {
                            url = rewritten;
                        }
                    }
                    *url_field = Some(url);
                }

                Some(media)
            },
            None => None,
        };

        Ok(media)
    }

    pub async fn get_media_for_user(
        &self,
        media_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<Option<Media>> {
        let now = time::OffsetDateTime::now_utc();
        let row = sqlx::query(
            "SELECT m.id, m.owner_id, m.original_key, m.thumb_key, m.medium_key, \
                    m.width, m.height, m.bytes, m.created_at \
             FROM media m \
             WHERE m.id = $1 \
               AND (m.owner_id = $2 \
                    OR EXISTS ( \
                        SELECT 1 FROM posts p \
                        WHERE p.media_id = m.id \
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
                    ) \
                    OR EXISTS ( \
                        SELECT 1 FROM stories s \
                        WHERE s.media_id = m.id \
                          AND s.expires_at > $3 \
                          AND (s.visibility = 'public' \
                               OR s.user_id = $2 \
                               OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only') \
                                   AND EXISTS (SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = s.user_id) \
                                   AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $2))) \
                          AND NOT EXISTS ( \
                              SELECT 1 FROM blocks \
                              WHERE (blocker_id = s.user_id AND blocked_id = $2) \
                                 OR (blocker_id = $2 AND blocked_id = s.user_id) \
                          ) \
                    ))",
        )
        .bind(media_id)
        .bind(viewer_id)
        .bind(now)
        .fetch_optional(self.db.pool())
        .await?;

        let media = match row {
            Some(row) => {
                let original_key: String = row.get("original_key");
                let thumb_key: String = row.get("thumb_key");
                let medium_key: String = row.get("medium_key");

                let mut media = Media {
                    id: row.get("id"),
                    owner_id: row.get("owner_id"),
                    original_key: original_key.clone(),
                    thumb_key: thumb_key.clone(),
                    medium_key: medium_key.clone(),
                    width: row.get("width"),
                    height: row.get("height"),
                    bytes: row.get("bytes"),
                    created_at: row.get("created_at"),
                    thumb_url: None,
                    medium_url: None,
                    original_url: None,
                };

                let presign_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;
                let keys = [
                    (&original_key, &mut media.original_url),
                    (&thumb_key, &mut media.thumb_url),
                    (&medium_key, &mut media.medium_url),
                ];

                for (key, url_field) in keys {
                    let presigned = self
                        .storage
                        .client()
                        .get_object()
                        .bucket(self.storage.bucket())
                        .key(key.clone())
                        .presigned(presign_config.clone())
                        .await?;

                    let mut url = presigned.uri().to_string();
                    if let Some(ref public_endpoint) = self.s3_public_endpoint {
                        if let Ok(rewritten) = rewrite_presigned_url(&url, public_endpoint) {
                            url = rewritten;
                        }
                    }
                    *url_field = Some(url);
                }

                Some(media)
            }
            None => None,
        };

        Ok(media)
    }

    pub async fn get_upload_status(
        &self,
        upload_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<UploadStatus>> {
        let row = sqlx::query(
            "SELECT status::text AS status, processed_media_id \
             FROM media_uploads WHERE id = $1 AND owner_id = $2",
        )
        .bind(upload_id)
        .bind(owner_id)
        .fetch_optional(self.db.pool())
        .await?;

        let status = row.map(|row| UploadStatus {
            status: row.get("status"),
            processed_media_id: row.get("processed_media_id"),
        });

        Ok(status)
    }

    pub async fn delete_media(&self, media_id: Uuid, owner_id: Uuid) -> Result<bool> {
        let row = sqlx::query(
            "SELECT id, owner_id, original_key, thumb_key, medium_key \
             FROM media WHERE id = $1 AND owner_id = $2",
        )
        .bind(media_id)
        .bind(owner_id)
        .fetch_optional(self.db.pool())
        .await?;

        let row = match row {
            Some(row) => row,
            None => return Ok(false),
        };

        let original_key: String = row.get("original_key");
        let thumb_key: String = row.get("thumb_key");
        let medium_key: String = row.get("medium_key");

        for key in [original_key, thumb_key, medium_key] {
            self.storage
                .client()
                .delete_object()
                .bucket(self.storage.bucket())
                .key(key)
                .send()
                .await?;
        }

        let result = sqlx::query("DELETE FROM media WHERE id = $1 AND owner_id = $2")
            .bind(media_id)
            .bind(owner_id)
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Generate a presigned GET URL for any object key (e.g., avatars)
    /// Returns None if the key is None or URL generation fails
    pub async fn generate_presigned_get_url(
        &self,
        object_key: Option<&str>,
        expires_in_seconds: u64,
    ) -> Option<String> {
        let key = object_key?;
        
        let presign_config = PresigningConfig::expires_in(Duration::from_secs(expires_in_seconds))
            .ok()?;
        
        let presigned = self
            .storage
            .client()
            .get_object()
            .bucket(self.storage.bucket())
            .key(key)
            .presigned(presign_config)
            .await
            .ok()?;
        
        let mut url = presigned.uri().to_string();
        
        // Rewrite to public endpoint if configured
        if let Some(ref public_endpoint) = self.s3_public_endpoint {
            if let Ok(rewritten) = rewrite_presigned_url(&url, public_endpoint) {
                url = rewritten;
            }
        }
        
        Some(url)
    }

    /// Populate avatar_url for a User by generating a presigned URL from avatar_key
    pub async fn populate_user_avatar_url(&self, user: &mut crate::domain::user::User) {
        if user.avatar_key.is_some() {
            user.avatar_url = self
                .generate_presigned_get_url(user.avatar_key.as_deref(), 3600)
                .await;
        }
    }

    /// Populate avatar URLs for a list of Users
    pub async fn populate_users_avatar_urls(&self, users: &mut [crate::domain::user::User]) {
        for user in users.iter_mut() {
            self.populate_user_avatar_url(user).await;
        }
    }
}

fn extension_from_content_type(content_type: &str) -> Result<&'static str> {
    match content_type {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        "image/webp" => Ok("webp"),
        _ => Err(anyhow!("unsupported content type")),
    }
}

fn rewrite_presigned_url(original: &str, public_endpoint: &str) -> Result<String> {
    let mut original_url = Url::parse(original)?;
    let public_url = Url::parse(public_endpoint)
        .or_else(|_| Url::parse(&format!("http://{}", public_endpoint)))?;

    original_url
        .set_scheme(public_url.scheme())
        .map_err(|_| anyhow!("invalid scheme for public endpoint"))?;
    original_url
        .set_host(public_url.host_str())
        .map_err(|_| anyhow!("invalid host for public endpoint"))?;
    original_url.set_port(public_url.port()).ok();

    Ok(original_url.to_string())
}
