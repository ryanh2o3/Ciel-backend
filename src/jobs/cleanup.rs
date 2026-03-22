use tokio_util::sync::CancellationToken;

use crate::infra::db::Db;

pub async fn run_cleanup_loop(db: Db, shutdown: CancellationToken) {
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
    }
}
