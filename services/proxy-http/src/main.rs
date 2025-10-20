use anyhow::{Context, Result};
use edge_policy_proxy_http::config::ProxyConfig;
use edge_policy_proxy_http::server::ProxyServer;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = ProxyConfig::from_env().context("Failed to load configuration")?;

    // Initialize tracing with the configured log level
    init_tracing(&config.log_level);

    info!("edge-policy-proxy-http service starting");
    info!(
        "Configuration loaded: upstream={}, enforcer={}",
        config.upstream_url, config.enforcer_url
    );
    info!(
        "mTLS enabled: {}, JWT enabled: {}",
        config.enable_mtls, config.enable_jwt
    );

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        return Err(e);
    }

    // Create and start server
    let server = ProxyServer::new(config).context("Failed to create proxy server")?;

    // Run server with graceful shutdown
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(e);
            }
        }
        _ = shutdown_signal() => {
            info!("Received shutdown signal");
        }
    }

    info!("edge-policy-proxy-http service stopped");
    Ok(())
}

fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .compact()
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
