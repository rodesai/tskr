use clap::Parser;
use std::path::PathBuf;
use tskr_cli::cli::{Cli, Command, DaemonAction};

#[test]
fn parses_search() {
    let cli = Cli::try_parse_from(["tskr", "search", "hello world", "--limit", "5"]).unwrap();
    match cli.command {
        Command::Search(args) => {
            assert_eq!(args.query, "hello world");
            assert_eq!(args.limit, Some(5));
        }
        _ => panic!("expected Search"),
    }
}

#[test]
fn parses_show_with_at_event() {
    let cli = Cli::try_parse_from(["tskr", "show", "abc123", "--at-event", "42"]).unwrap();
    match cli.command {
        Command::Show(args) => {
            assert_eq!(args.session_id, "abc123");
            assert_eq!(args.at_event, Some(42));
        }
        _ => panic!("expected Show"),
    }
}

#[test]
fn parses_backfill() {
    let cli = Cli::try_parse_from(["tskr", "backfill", "/tmp/fixtures"]).unwrap();
    match cli.command {
        Command::Backfill(args) => {
            assert_eq!(args.dir, PathBuf::from("/tmp/fixtures"));
        }
        _ => panic!("expected Backfill"),
    }
}

#[test]
fn parses_daemon_start() {
    let cli = Cli::try_parse_from(["tskr", "daemon", "start"]).unwrap();
    match cli.command {
        Command::Daemon(args) => match args.action {
            DaemonAction::Start => {}
            _ => panic!("expected DaemonAction::Start"),
        },
        _ => panic!("expected Daemon"),
    }
}

#[test]
fn rejects_search_without_query() {
    assert!(Cli::try_parse_from(["tskr", "search"]).is_err());
}

#[test]
fn binary_help_exits_zero() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_tskr"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    for sub in ["search", "list", "show", "backfill", "daemon"] {
        assert!(s.contains(sub), "--help missing `{sub}`: {s}");
    }
}
