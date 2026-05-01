use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "tskr", version, about = "Search and browse Claude sessions")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Semantic search over indexed sessions.
    Search(SearchArgs),
    /// List recent sessions (metadata only).
    List(ListArgs),
    /// Show events from a session.
    Show(ShowArgs),
    /// Backfill sessions from a directory of `.jsonl` files.
    Backfill(BackfillArgs),
    /// Daemon control (deferred to a later iteration).
    Daemon(DaemonArgs),
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,
    /// Filter by repo.
    #[arg(long)]
    pub repo: Option<String>,
    /// Filter by author email.
    #[arg(long)]
    pub author: Option<String>,
    /// Drop hits older than this duration (e.g. 7d, 24h, 30m, 90s).
    #[arg(long)]
    pub since: Option<String>,
    /// Maximum number of results.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Filter by repo.
    #[arg(long)]
    pub repo: Option<String>,
    /// Filter by author email.
    #[arg(long)]
    pub author: Option<String>,
    /// Drop sessions older than this duration (e.g. 7d, 24h, 30m, 90s).
    #[arg(long)]
    pub since: Option<String>,
    /// Maximum number of sessions.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Session id to render.
    pub session_id: String,
    /// Center the rendered window on this event index.
    #[arg(long)]
    pub at_event: Option<usize>,
}

#[derive(Debug, Args)]
pub struct BackfillArgs {
    /// Directory to walk for `.jsonl` files.
    pub dir: PathBuf,
    /// Author email override (default: detected from `git config user.email`).
    #[arg(long)]
    pub author: Option<String>,
    /// Repo override (default: parent directory name).
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub action: DaemonAction,
}

#[derive(Debug, Subcommand)]
pub enum DaemonAction {
    /// Start the daemon (deferred).
    Start,
    /// Show daemon status (deferred).
    Status,
    /// Stop the daemon (deferred).
    Stop,
}
