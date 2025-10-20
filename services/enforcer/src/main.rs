use std::{
    future::pending,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::serve;
use edge_policy_enforcer::{config::EnforcerConfig, create_router, DecisionEvent, PolicyManager};
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use tokio::{
    net::TcpListener,
    signal,
    sync::{broadcast, mpsc},
};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let config = EnforcerConfig::from_env().context("failed to load configuration")?;
    init_tracing(&config);

    info!("edge-policy-enforcer starting");

    let policy_manager = Arc::new(PolicyManager::new(config.bundles_dir.clone()));
    let tenants_loaded = policy_manager
        .load_all_tenants()
        .context("failed to load tenant bundles")?;
    info!(
        tenants_loaded = tenants_loaded,
        "initial tenant bundles loaded"
    );

    let (event_tx, _event_rx) = broadcast::channel::<DecisionEvent>(256);
    let event_tx = Arc::new(event_tx);

    if config.enable_hot_reload {
        spawn_hot_reload_watcher(Arc::clone(&policy_manager), config.clone())
            .context("failed to start hot reload watcher")?;
    } else {
        info!("hot reload watcher disabled by configuration");
    }

    let router = create_router(Arc::clone(&policy_manager), Arc::clone(&event_tx));

    let addr: SocketAddr = format!("{}:{}", config.server_host, config.server_port)
        .parse()
        .context("invalid server bind address")?;

    let listener = TcpListener::bind(addr)
        .await
        .context("failed to bind TCP listener")?;
    let local_addr = listener
        .local_addr()
        .context("failed to read bound address")?;
    info!(%local_addr, "edge-policy-enforcer listening");

    serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server encountered an unrecoverable error")?;

    info!("edge-policy-enforcer shutdown complete");
    Ok(())
}

fn init_tracing(config: &EnforcerConfig) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| std::env::var("LOG_LEVEL").map(EnvFilter::new))
        .unwrap_or_else(|_| EnvFilter::new(config.log_level.clone()));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn spawn_hot_reload_watcher(
    policy_manager: Arc<PolicyManager>,
    config: EnforcerConfig,
) -> Result<()> {
    let watch_path = config.bundles_dir;
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();

    let mut watcher = recommended_watcher({
        move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                if event_tx.send(event).is_err() {
                    warn!("hot reload watcher receiver dropped");
                }
            }
            Err(err) => error!(error = ?err, "hot reload watch error"),
        }
    })
    .context("failed to create filesystem watcher")?;

    watcher
        .watch(&watch_path, RecursiveMode::Recursive)
        .with_context(|| {
            format!(
                "failed to watch bundles directory '{}'",
                watch_path.display()
            )
        })?;

    info!(path = %watch_path.display(), "hot reload watcher started");

    let manager = Arc::clone(&policy_manager);
    let path_for_task = watch_path.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if !is_relevant_event(&event.kind) {
                continue;
            }

            if let Some(tenant_id) = resolve_tenant_id(&path_for_task, &event.paths) {
                match manager.reload_tenant(&tenant_id) {
                    Ok(()) => info!(
                        tenant = %tenant_id,
                        "tenant bundle reloaded after filesystem change"
                    ),
                    Err(err) => error!(
                        tenant = %tenant_id,
                        error = ?err,
                        "failed to reload tenant bundle after filesystem change"
                    ),
                }
            }
        }
    });

    // Hold watcher for the lifetime of the process.
    tokio::spawn(async move {
        let _watcher = watcher;
        pending::<()>().await;
    });

    Ok(())
}

fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

fn resolve_tenant_id(base: &Path, paths: &[PathBuf]) -> Option<String> {
    paths.iter().find_map(|path| {
        let relative = path.strip_prefix(base).ok()?;
        match relative.components().next()? {
            Component::Normal(component) => component.to_str().map(|s| s.to_string()),
            _ => None,
        }
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
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
