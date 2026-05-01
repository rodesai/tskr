use crate::cli::DaemonArgs;

pub fn run(_args: DaemonArgs) -> anyhow::Result<()> {
    eprintln!("tskr daemon: deferred to iter 7");
    std::process::exit(2);
}
