//! Runs the todo API server.

use anyhow::Context as _;
use rust_api_playground::config::Config;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    let config = Config::from_env()?;
    let pool = rust_api_playground::connect(&config.database_url).await?;
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!(
        addr = %listener.local_addr()?,
        "listening; docs at /swagger-ui",
    );

    axum::serve(listener, rust_api_playground::app(pool))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")
}

/// Resolves on Ctrl-C. If the handler can't be installed, never resolves.
async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %err, "failed to listen for Ctrl-C");
        std::future::pending::<()>().await;
    }
    tracing::info!("shutting down");
}
