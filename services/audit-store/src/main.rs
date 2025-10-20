mod api;
mod config;
mod signing;
mod storage;
mod upload;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Server;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use api::ApiState;
use config::AuditStoreConfig;
use upload::UploadQueue;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let config = AuditStoreConfig::from_env()?;
    let host = config.server_host.clone();
    let port = config.server_port;

    info!(
        host = %host,
        port,
        data_dir = %config.data_dir.display(),
        "starting audit-store service"
    );

    let state = ApiState::new(config)?;
    let state = Arc::new(state);

    if state.config.enable_deferred_upload {
        let queue = UploadQueue::new(
            Arc::clone(&state.database),
            Arc::clone(&state.tenant_registry),
            &state.config,
        );
        queue.start();
    } else {
        warn!("deferred upload disabled");
    }

    let router = api::create_router(Arc::clone(&state));
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    Server::bind(&addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("audit-store service shutting down");
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
