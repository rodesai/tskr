#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = tskr_daemon::Config::from_env()?;
    tskr_daemon::run(cfg).await
}
