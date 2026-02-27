use axum::Router;
use anyhow::anyhow;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ciel::config::AppConfig;
use ciel::infra::{cache::RedisCache, db::Db, queue::QueueClient, storage::ObjectStorage};
use ciel::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;

    // Serverless worker only needs DB + S3, skip Redis/Queue init
    if config.app_mode == "serverless-worker" {
        tracing::info!("starting serverless worker mode");
        let db = Db::connect(&config).await?;
        let storage = ObjectStorage::new(&config).await?;

        let app = ciel::jobs::media_processor::router(db, storage);
        let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
        tracing::info!("serverless worker listening on {}", config.http_addr);
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        return Ok(());
    }

    let db = Db::connect(&config).await?;
    let cache = RedisCache::connect(&config.redis_url).await?;
    let storage = ObjectStorage::new(&config).await?;
    let queue = QueueClient::new(&config).await?;

    let state = AppState {
        db,
        cache,
        storage,
        queue,
        upload_url_ttl_seconds: config.upload_url_ttl_seconds,
        upload_max_bytes: config.upload_max_bytes,
        admin_token: config.admin_token.clone(),
        paseto_access_key: config.paseto_access_key,
        paseto_refresh_key: config.paseto_refresh_key,
        access_ttl_minutes: config.access_ttl_minutes,
        refresh_ttl_days: config.refresh_ttl_days,
        s3_public_endpoint: config.s3_public_endpoint,
        ip_signup_rate_limit: config.ip_signup_rate_limit,
    };

    match config.app_mode.as_str() {
        "api" => {
            // H7: Spawn background cleanup task for expired data
            let cleanup_db = state.db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                loop {
                    interval.tick().await;
                    // Clean up expired stories (and cascaded views/reactions) older than 48h
                    match sqlx::query(
                        "DELETE FROM stories WHERE expires_at < now() - interval '48 hours'",
                    )
                    .execute(cleanup_db.pool())
                    .await
                    {
                        Ok(result) => {
                            if result.rows_affected() > 0 {
                                tracing::info!(
                                    count = result.rows_affected(),
                                    "cleaned up expired stories"
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!(error = ?err, "failed to clean up expired stories");
                        }
                    }

                    // Clean up expired invites
                    match sqlx::query("SELECT cleanup_expired_invites()")
                        .execute(cleanup_db.pool())
                        .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            tracing::warn!(error = ?err, "failed to clean up expired invites");
                        }
                    }
                }
            });

            let app: Router = ciel::http::router(state).layer(TraceLayer::new_for_http());
            let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
            tracing::info!("listening on {}", config.http_addr);

            // Convert the router to handle ConnectInfo properly
            let app = app.into_make_service_with_connect_info::<SocketAddr>();

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        "worker" => {
            tracing::info!("starting worker mode");
            tokio::select! {
                result = ciel::jobs::media_processor::run(state.db.clone(), state.storage.clone(), state.queue.clone()) => {
                    result?;
                }
                _ = shutdown_signal() => {}
            }
        }
        "combined" => {
            tracing::info!("starting combined mode (api + worker)");

            // Spawn background cleanup task
            let cleanup_db = state.db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                loop {
                    interval.tick().await;
                    match sqlx::query(
                        "DELETE FROM stories WHERE expires_at < now() - interval '48 hours'",
                    )
                    .execute(cleanup_db.pool())
                    .await
                    {
                        Ok(result) => {
                            if result.rows_affected() > 0 {
                                tracing::info!(
                                    count = result.rows_affected(),
                                    "cleaned up expired stories"
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!(error = ?err, "failed to clean up expired stories");
                        }
                    }
                    match sqlx::query("SELECT cleanup_expired_invites()")
                        .execute(cleanup_db.pool())
                        .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            tracing::warn!(error = ?err, "failed to clean up expired invites");
                        }
                    }
                }
            });

            // Spawn media processing worker in the background
            let worker_db = state.db.clone();
            let worker_storage = state.storage.clone();
            let worker_queue = state.queue.clone();
            tokio::spawn(async move {
                if let Err(err) =
                    ciel::jobs::media_processor::run(worker_db, worker_storage, worker_queue).await
                {
                    tracing::error!(error = ?err, "media worker exited with error");
                }
            });

            // Run the API server
            let app: Router = ciel::http::router(state).layer(TraceLayer::new_for_http());
            let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
            tracing::info!("listening on {}", config.http_addr);
            let app = app.into_make_service_with_connect_info::<SocketAddr>();
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        other => return Err(anyhow!("unknown APP_MODE: {}", other)),
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "failed to install SIGTERM handler");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
