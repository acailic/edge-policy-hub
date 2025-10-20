mod api;
mod config;
mod storage;
mod tracker;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Server;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use api::ApiState;
use config::QuotaTrackerConfig;
use storage::QuotaDatabase;
use tracker::QuotaManager;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let config = QuotaTrackerConfig::from_env()?;
    let host = config.server_host.clone();
    let port = config.server_port;

    info!(
        host = %host,
        port,
        data_dir = %config.data_dir.display(),
        "starting quota-tracker service"
    );

    let database = Arc::new(QuotaDatabase::new(config.data_dir.clone())?);
    let manager = Arc::new(QuotaManager::new(Arc::clone(&database), &config));

    let loaded = match manager.load_from_database() {
        Ok(val) => val,
        Err(err) => {
            warn!(error = %err, "failed to restore quota metrics from persistence");
            0
        }
    };
    info!(loaded_tenants = loaded, "restored quota metrics from persistence");

    let _persistence_task = manager.start_persistence_task();

    let state = Arc::new(ApiState::new(Arc::clone(&manager), config));
    let router = api::create_router(Arc::clone(&state));
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    Server::bind(&addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("quota-tracker service shutting down");
    Ok(())
}

fn init_tracing() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).try_init()?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
