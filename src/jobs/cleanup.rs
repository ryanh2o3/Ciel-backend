use sqlx::Row;
use tokio_util::sync::CancellationToken;

use crate::infra::db::Db;
use crate::infra::queue::QueueClient;
use crate::jobs::media_processor::MediaJob;

/// Uploads stuck in 'uploaded'/'processing' longer than this are re-enqueued.
/// Covers lost queue messages, enqueue failures and worker crashes.
const STALE_UPLOAD_THRESHOLD: &str = "15 minutes";
const STALE_UPLOAD_BATCH: i64 = 100;

pub async fn run_cleanup_loop(db: Db, queue: QueueClient, shutdown: CancellationToken) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    let mut backoff_secs: u64 = 0;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("cleanup loop stopping");
                break;
            }
            _ = interval.tick() => {}
        }

        if backoff_secs > 0 {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("cleanup loop stopping");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)) => {}
            }
        }

        let stories_result =
            sqlx::query("DELETE FROM stories WHERE expires_at < now() - interval '48 hours'")
                .execute(db.pool())
                .await;

        match stories_result {
            Ok(result) => {
                backoff_secs = 0;
                if result.rows_affected() > 0 {
                    tracing::info!(
                        count = result.rows_affected(),
                        "cleaned up expired stories"
                    );
                }
            }
            Err(err) => {
                metrics::counter!("cleanup_failures_total", "job" => "stories").increment(1);
                tracing::warn!(error = ?err, "failed to clean up expired stories");
                backoff_secs = (backoff_secs.max(1) * 2).min(300);
                continue;
            }
        }

        match sqlx::query("SELECT cleanup_expired_invites()")
            .execute(db.pool())
            .await
        {
            Ok(_) => {
                metrics::counter!("cleanup_runs_total", "job" => "invites").increment(1);
            }
            Err(err) => {
                metrics::counter!("cleanup_failures_total", "job" => "invites").increment(1);
                tracing::warn!(error = ?err, "failed to clean up expired invites");
                backoff_secs = (backoff_secs.max(1) * 2).min(300);
            }
        }

        // Keep trust-score account ages current (feeds trust level promotion).
        match sqlx::query("SELECT update_account_ages()")
            .execute(db.pool())
            .await
        {
            Ok(_) => {
                metrics::counter!("cleanup_runs_total", "job" => "account_ages").increment(1);
            }
            Err(err) => {
                metrics::counter!("cleanup_failures_total", "job" => "account_ages").increment(1);
                tracing::warn!(error = ?err, "failed to update account ages");
            }
        }

        requeue_stale_uploads(&db, &queue).await;
    }
}

/// Re-enqueue uploads that have sat in a non-terminal state too long.
/// Safe because the media worker accepts jobs in 'uploaded' or 'processing'
/// and processing is idempotent.
async fn requeue_stale_uploads(db: &Db, queue: &QueueClient) {
    let rows = sqlx::query(
        "SELECT id, owner_id, original_key \
         FROM media_uploads \
         WHERE status IN ('uploaded', 'processing') \
           AND COALESCE(uploaded_at, created_at) < now() - $1::interval \
         ORDER BY created_at \
         LIMIT $2",
    )
    .bind(STALE_UPLOAD_THRESHOLD)
    .bind(STALE_UPLOAD_BATCH)
    .fetch_all(db.pool())
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(err) => {
            metrics::counter!("cleanup_failures_total", "job" => "stale_uploads").increment(1);
            tracing::warn!(error = ?err, "failed to query stale uploads");
            return;
        }
    };

    if rows.is_empty() {
        return;
    }

    tracing::info!(count = rows.len(), "re-enqueueing stale media uploads");
    for row in rows {
        let job = MediaJob {
            upload_id: row.get("id"),
            owner_id: row.get("owner_id"),
            original_key: row.get("original_key"),
        };
        if let Err(err) = queue.enqueue_media_job(&job).await {
            metrics::counter!("cleanup_failures_total", "job" => "stale_uploads").increment(1);
            tracing::warn!(error = ?err, upload_id = %job.upload_id, "failed to re-enqueue stale upload");
        }
    }
}
