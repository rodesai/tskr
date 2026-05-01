use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::cli::BackfillArgs;
use crate::config::Config;

pub async fn run(cfg: Config, args: BackfillArgs) -> anyhow::Result<()> {
    let files = walk_jsonl(&args.dir)?;
    let writer_url = cfg.writer_url.trim_end_matches('/').to_string();
    let upload_url = format!("{writer_url}/sessions/upload");
    let http = reqwest::Client::new();
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());

    let total = files.len();
    let mut success = 0_usize;
    tracing::info!(total, dir = %args.dir.display(), "backfill starting");

    for path in &files {
        let bytes = tokio::fs::read(path)
            .await
            .with_context(|| format!("read {} failed", path.display()))?;
        if bytes.is_empty() {
            continue;
        }
        let author = args.author.clone().unwrap_or_else(detect_author);
        let repo = args.repo.clone().unwrap_or_else(|| detect_repo(path));

        let result = http
            .post(&upload_url)
            .header("X-Tskr-Author", &author)
            .header("X-Tskr-Repo", &repo)
            .header("X-Tskr-Host", &host)
            .body(bytes)
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    eprintln!("{}: upload failed: {} {}", path.display(), status, text);
                    continue;
                }
                let body: UploadResponse = match resp.json().await {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{}: failed to decode response: {e}", path.display());
                        continue;
                    }
                };
                println!(
                    "{}: accepted={} indexed={}",
                    path.display(),
                    body.accepted,
                    body.indexed
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("{}: upload failed: {e}", path.display());
            }
        }
    }

    println!("backfill complete: {success}/{total} files");
    if success != total {
        std::process::exit(1);
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct UploadResponse {
    accepted: usize,
    indexed: usize,
}

fn walk_jsonl(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) => {
                anyhow::bail!("read_dir {} failed: {e}", dir.display());
            }
        };
        for entry in read {
            let entry =
                entry.with_context(|| format!("read_dir entry under {} failed", dir.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("file_type for {} failed", path.display()))?;
            if file_type.is_dir() {
                queue.push_back(path);
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn detect_author() -> String {
    let output = Command::new("git").args(["config", "user.email"]).output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                "unknown@local".to_string()
            } else {
                s
            }
        }
        _ => "unknown@local".to_string(),
    }
}

fn detect_repo(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
