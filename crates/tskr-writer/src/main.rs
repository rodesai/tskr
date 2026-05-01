use anyhow::Result;
use tracing_subscriber::EnvFilter;

use tskr_writer::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let app = tskr_writer::app();

    tracing::info!(addr = %cfg.bind_addr, "tskr-writer listening");
    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
