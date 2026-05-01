use crate::cli::{DaemonAction, DaemonArgs};

pub async fn run(args: DaemonArgs) -> anyhow::Result<()> {
    match args.action {
        DaemonAction::Start => {
            let cfg = tskr_daemon::Config::from_env()?;
            tskr_daemon::run(cfg).await
        }
        DaemonAction::Status => {
            println!("milestone 1: tskr daemon runs in the foreground; no status check");
            Ok(())
        }
        DaemonAction::Stop => {
            println!("milestone 1: stop with Ctrl-C in the daemon's terminal");
            Ok(())
        }
    }
}
