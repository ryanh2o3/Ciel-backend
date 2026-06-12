//! Bounded in-process queue for best-effort notification side effects (no unbounded spawn).
use serde_json::json;
use sqlx::Row;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::app::notifications::NotificationService;
use crate::infra::db::Db;

#[derive(Debug, Clone)]
pub enum NotificationJob {
    UserFollowed {
        followee_id: Uuid,
        actor_id: Uuid,
    },
    PostLiked {
        post_id: Uuid,
        actor_id: Uuid,
    },
    PostCommented {
        post_id: Uuid,
        actor_id: Uuid,
        preview: String,
    },
    StoryReaction {
        story_id: Uuid,
        actor_id: Uuid,
        emoji: String,
    },
}

pub async fn run_notification_worker(
    mut rx: mpsc::Receiver<NotificationJob>,
    db: Db,
    shutdown: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                // Stop accepting new jobs, then drain what's already queued so
                // notifications accepted before shutdown aren't silently lost.
                rx.close();
                let mut drained = 0usize;
                while let Ok(job) = rx.try_recv() {
                    if let Err(err) = process_job(&db, job).await {
                        metrics::counter!("notifications_worker_failures_total").increment(1);
                        tracing::warn!(error = ?err, "notification job failed during drain");
                    }
                    drained += 1;
                }
                tracing::info!(drained, "notification worker stopping");
                break;
            }
            job = rx.recv() => {
                let Some(job) = job else { break };
                if let Err(err) = process_job(&db, job).await {
                    metrics::counter!("notifications_worker_failures_total").increment(1);
                    tracing::warn!(error = ?err, "notification job failed");
                }
            }
        }
    }
}

async fn actor_handle(db: &Db, user_id: Uuid) -> Option<String> {
    sqlx::query_scalar(
        "SELECT handle FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten()
}

async fn process_job(db: &Db, job: NotificationJob) -> anyhow::Result<()> {
    let notif_svc = NotificationService::new(db.clone());
    match job {
        NotificationJob::UserFollowed {
            followee_id,
            actor_id,
        } => {
            let handle = actor_handle(db, actor_id).await;
            let mut payload = json!({
                "follower_id": actor_id.to_string(),
            });
            if let Some(handle) = handle {
                payload["follower_handle"] = json!(handle);
            }
            notif_svc
                .create_if_not_self(followee_id, actor_id, "user_followed", payload)
                .await?;
        }
        NotificationJob::PostLiked { post_id, actor_id } => {
            let owner_row =
                sqlx::query("SELECT owner_id FROM posts WHERE id = $1")
                    .bind(post_id)
                    .fetch_optional(db.pool())
                    .await?;
            if let Some(row) = owner_row {
                let owner_id: Uuid = row.get("owner_id");
                let handle = actor_handle(db, actor_id).await;
                let mut payload = json!({
                    "post_id": post_id.to_string(),
                    "liker_id": actor_id.to_string(),
                });
                if let Some(handle) = handle {
                    payload["liker_handle"] = json!(handle);
                }
                notif_svc
                    .create_if_not_self(owner_id, actor_id, "post_liked", payload)
                    .await?;
            }
        }
        NotificationJob::PostCommented {
            post_id,
            actor_id,
            preview,
        } => {
            let owner_row =
                sqlx::query("SELECT owner_id FROM posts WHERE id = $1")
                    .bind(post_id)
                    .fetch_optional(db.pool())
                    .await?;
            if let Some(row) = owner_row {
                let owner_id: Uuid = row.get("owner_id");
                let handle = actor_handle(db, actor_id).await;
                let mut payload = json!({
                    "post_id": post_id.to_string(),
                    "comment_preview": preview,
                    "commenter_id": actor_id.to_string(),
                });
                if let Some(handle) = handle {
                    payload["commenter_handle"] = json!(handle);
                }
                notif_svc
                    .create_if_not_self(owner_id, actor_id, "post_commented", payload)
                    .await?;
            }
        }
        NotificationJob::StoryReaction {
            story_id,
            actor_id,
            emoji,
        } => {
            let owner_row =
                sqlx::query("SELECT user_id FROM stories WHERE id = $1")
                    .bind(story_id)
                    .fetch_optional(db.pool())
                    .await?;
            if let Some(row) = owner_row {
                let owner_id: Uuid = row.get("user_id");
                let handle = actor_handle(db, actor_id).await;
                let mut payload = json!({
                    "story_id": story_id.to_string(),
                    "emoji": emoji,
                    "reactor_id": actor_id.to_string(),
                });
                if let Some(handle) = handle {
                    payload["reactor_handle"] = json!(handle);
                }
                notif_svc
                    .create_if_not_self(owner_id, actor_id, "story_reaction", payload)
                    .await?;
            }
        }
    }
    Ok(())
}
