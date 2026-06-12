use anyhow::{anyhow, Result};
use aws_sdk_s3::primitives::ByteStream;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::io::Cursor;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use tracing::{error, info, warn};

use crate::infra::{db::Db, queue::QueueClient, storage::ObjectStorage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaJob {
    pub upload_id: Uuid,
    pub owner_id: Uuid,
    pub original_key: String,
}

const POLL_WAIT_SECONDS: i32 = 10;
const IDLE_SLEEP_MS: u64 = 200;
const ERROR_BACKOFF_MS: u64 = 1000;

const THUMB_MAX_PX: u32 = 200;
const MEDIUM_MAX_PX: u32 = 800;

/// Decompression-bomb guard: reject images whose decoded dimensions exceed
/// this, regardless of how small the compressed payload is.
const MAX_IMAGE_DIMENSION_PX: u32 = 12_000;

enum ProcessingOutcome {
    Completed,
    RetryLater,
}

/// Returns true if the error is permanent (image decode, unsupported format)
/// and should not be retried.
fn is_permanent_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("failed to decode image")
        || msg.contains("unsupported content type")
}

pub async fn run(
    db: Db,
    storage: ObjectStorage,
    queue: QueueClient,
    shutdown: CancellationToken,
) -> Result<()> {
    info!("media processor started");
    loop {
        let received = tokio::select! {
            _ = shutdown.cancelled() => {
                info!("media processor stopping");
                return Ok(());
            }
            received = queue.receive_media_job(POLL_WAIT_SECONDS) => received,
        };

        match received {
            Ok(Some(message)) => {
                let outcome = match process_job(&db, &storage, &message.job).await {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        if is_permanent_error(&err) {
                            error!(
                                error = ?err,
                                upload_id = %message.job.upload_id,
                                "permanent failure processing media job"
                            );
                            let _ = mark_failed(&db, &message.job).await;
                            ProcessingOutcome::Completed
                        } else {
                            warn!(
                                error = ?err,
                                upload_id = %message.job.upload_id,
                                "transient failure processing media job, will retry"
                            );
                            ProcessingOutcome::RetryLater
                        }
                    }
                };

                if matches!(outcome, ProcessingOutcome::Completed) {
                    if let Err(err) = queue.delete_message(&message.receipt_handle).await {
                        warn!(error = ?err, "failed to delete queue message");
                    }
                }
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_millis(IDLE_SLEEP_MS)).await;
            }
            Err(err) => {
                warn!(error = ?err, "queue receive failed, backing off");
                tokio::time::sleep(Duration::from_millis(ERROR_BACKOFF_MS)).await;
            }
        }
    }
}

async fn process_job(
    db: &Db,
    storage: &ObjectStorage,
    job: &MediaJob,
) -> Result<ProcessingOutcome> {
    // Claim the job. Accepting redeliveries in 'processing' state is what lets
    // an upload recover after a transient mid-processing failure — previously
    // it would stay 'processing' forever. Processing is idempotent: variant
    // keys are deterministic and the final DB writes happen in one transaction.
    let row = sqlx::query(
        "UPDATE media_uploads \
         SET status = 'processing' \
         WHERE id = $1 AND owner_id = $2 AND status IN ('uploaded', 'processing') \
         RETURNING original_key, content_type, bytes",
    )
    .bind(job.upload_id)
    .bind(job.owner_id)
    .fetch_optional(db.pool())
    .await?;

    let (original_key, content_type, bytes) = match row {
        Some(row) => (
            row.get::<String, _>("original_key"),
            row.get::<String, _>("content_type"),
            row.get::<i64, _>("bytes"),
        ),
        None => {
            // Already completed/failed, or the upload row no longer exists.
            // In every case the message is stale and should be consumed.
            return Ok(ProcessingOutcome::Completed);
        }
    };

    let object = storage
        .client()
        .get_object()
        .bucket(storage.bucket())
        .key(&original_key)
        .send()
        .await?;

    let data = object.body.collect().await?.into_bytes();
    let output_format = image_format_from_content_type(&content_type)?;

    // Decode + resize is CPU-bound (can take hundreds of ms for large photos);
    // run it on the blocking pool so it doesn't stall the async runtime.
    let (width, height, thumb_data, medium_data) =
        tokio::task::spawn_blocking(move || -> Result<(u32, u32, Vec<u8>, Vec<u8>)> {
            let image = decode_image(&data)?;
            let (width, height) = image.dimensions();
            let thumb = resize_and_encode(&image, THUMB_MAX_PX, output_format)?;
            let medium = resize_and_encode(&image, MEDIUM_MAX_PX, output_format)?;
            Ok((width, height, thumb, medium))
        })
        .await
        .map_err(|err| anyhow!("image processing task failed: {}", err))??;

    let ext = extension_from_content_type(&content_type)?;
    let thumb_key = format!("media/{}/{}/thumb.{}", job.owner_id, job.upload_id, ext);
    let medium_key = format!("media/{}/{}/medium.{}", job.owner_id, job.upload_id, ext);

    upload_variant(storage, &thumb_key, &content_type, thumb_data.into()).await?;
    upload_variant(storage, &medium_key, &content_type, medium_data.into()).await?;

    // Insert media + flip status atomically so a crash between the two can't
    // leave an orphaned media row that a redelivery would duplicate.
    let mut tx = db.pool().begin().await?;
    let media_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO media (id, owner_id, original_key, thumb_key, medium_key, width, height, bytes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(media_id)
    .bind(job.owner_id)
    .bind(original_key)
    .bind(thumb_key)
    .bind(medium_key)
    .bind(width as i32)
    .bind(height as i32)
    .bind(bytes)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE media_uploads \
         SET status = 'completed', processed_media_id = $1 \
         WHERE id = $2 AND owner_id = $3",
    )
    .bind(media_id)
    .bind(job.upload_id)
    .bind(job.owner_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(upload_id = %job.upload_id, media_id = %media_id, "media processing completed");
    Ok(ProcessingOutcome::Completed)
}

/// Decode with explicit dimension limits so a crafted tiny file that inflates
/// to a gigantic bitmap (decompression bomb) is rejected instead of taking
/// down the worker. Limit errors surface as "failed to decode image", which
/// `is_permanent_error` treats as non-retryable.
fn decode_image(data: &[u8]) -> Result<image::DynamicImage> {
    let mut reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|err| anyhow!("failed to decode image: {}", err))?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION_PX);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION_PX);
    reader.limits(limits);

    reader
        .decode()
        .map_err(|err| anyhow!("failed to decode image: {}", err))
}

/// Resize image so the longest side is at most `max_px`, preserving aspect ratio.
/// If the image is already smaller, return the original encoded at the target format.
fn resize_and_encode(
    img: &image::DynamicImage,
    max_px: u32,
    format: image::ImageFormat,
) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    let resized = if w > max_px || h > max_px {
        img.thumbnail(max_px, max_px)
    } else {
        img.clone()
    };

    let mut buf = Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, format)
        .map_err(|err| anyhow!("failed to encode resized image: {}", err))?;
    Ok(buf.into_inner())
}

async fn upload_variant(
    storage: &ObjectStorage,
    key: &str,
    content_type: &str,
    bytes: bytes::Bytes,
) -> Result<()> {
    storage
        .client()
        .put_object()
        .bucket(storage.bucket())
        .key(key)
        .content_type(content_type)
        .body(ByteStream::from(bytes))
        .send()
        .await?;
    Ok(())
}

async fn mark_failed(db: &Db, job: &MediaJob) -> Result<()> {
    sqlx::query(
        "UPDATE media_uploads \
         SET status = 'failed' \
         WHERE id = $1 AND owner_id = $2 AND status = 'processing'",
    )
    .bind(job.upload_id)
    .bind(job.owner_id)
    .execute(db.pool())
    .await?;
    Ok(())
}

fn extension_from_content_type(content_type: &str) -> Result<&'static str> {
    match content_type {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        "image/webp" => Ok("webp"),
        _ => Err(anyhow!("unsupported content type")),
    }
}

fn image_format_from_content_type(content_type: &str) -> Result<image::ImageFormat> {
    match content_type {
        "image/jpeg" => Ok(image::ImageFormat::Jpeg),
        "image/png" => Ok(image::ImageFormat::Png),
        "image/webp" => Ok(image::ImageFormat::WebP),
        _ => Err(anyhow!("unsupported content type")),
    }
}

// --- Serverless worker: HTTP handler for SQS trigger ---

#[derive(Clone)]
pub struct WorkerState {
    pub db: Db,
    pub storage: ObjectStorage,
}

/// Build a minimal router for the serverless worker container.
/// Scaleway SQS trigger POSTs the raw message body to `/`.
pub fn router(db: Db, storage: ObjectStorage) -> Router {
    Router::new()
        .route("/", post(handle_media_job))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(WorkerState { db, storage })
}

async fn handle_media_job(
    State(state): State<WorkerState>,
    Json(job): Json<MediaJob>,
) -> StatusCode {
    info!(upload_id = %job.upload_id, "serverless worker received media job");

    match process_job(&state.db, &state.storage, &job).await {
        Ok(ProcessingOutcome::Completed) => StatusCode::OK,
        Ok(ProcessingOutcome::RetryLater) => {
            warn!(upload_id = %job.upload_id, "media job needs retry");
            StatusCode::INTERNAL_SERVER_ERROR
        }
        Err(err) if is_permanent_error(&err) => {
            error!(error = ?err, upload_id = %job.upload_id, "permanent failure in serverless worker");
            let _ = mark_failed(&state.db, &job).await;
            StatusCode::OK // consume message, don't retry
        }
        Err(err) => {
            error!(error = ?err, upload_id = %job.upload_id, "transient failure in serverless worker");
            StatusCode::INTERNAL_SERVER_ERROR // trigger retry
        }
    }
}
