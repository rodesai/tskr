use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use tskr_cli::cli::{Cli, Command};
use tskr_cli::commands;
use tskr_cli::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Search(args) => {
            let cfg = Config::from_env()?;
            commands::search::run(cfg, args).await
        }
        Command::List(args) => {
            let cfg = Config::from_env()?;
            commands::list::run(cfg, args).await
        }
        Command::Show(args) => {
            let cfg = Config::from_env()?;
            commands::show::run(cfg, args).await
        }
        Command::Backfill(args) => {
            let cfg = Config::from_env()?;
            commands::backfill::run(cfg, args).await
        }
        Command::Daemon(args) => commands::daemon::run(args).await,
    }
}
