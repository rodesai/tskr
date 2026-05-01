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
    let bind_addr = cfg.bind_addr;
    let s3 = tskr_writer::s3::Client::new(&cfg).await?;
    let embed = tskr_writer::embed::Client::new(&cfg);
    let vector = tskr_writer::vector::Client::new(&cfg);
    let state = std::sync::Arc::new(tskr_writer::routes::AppState {
        cfg,
        s3,
        embed,
        vector,
    });

    tracing::info!(addr = %bind_addr, "tskr-writer listening");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, tskr_writer::routes::app(state)).await?;
    Ok(())
}
