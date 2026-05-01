use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_WRITER_URL: &str = "http://localhost:8090";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 2;
const MAX_RETRIES: u32 = 6;
const MAX_BACKOFF_SECS: u64 = 30;

#[derive(Debug, Error)]
enum UploadError {
    #[error("network error: {0}")]
    Network(reqwest::Error),
    #[error("server error: {status}: {body}")]
    Server { status: u16, body: String },
    #[error("client error: {status}: {body}")]
    Client { status: u16, body: String },
}

pub struct Config {
    pub writer_url: String,
    pub watch_dir: PathBuf,
    pub state_file: PathBuf,
    pub poll_interval_secs: u64,
    pub author: Option<String>,
    pub repo: Option<String>,
    pub host: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let writer_url =
            std::env::var("TSKR_WRITER_URL").unwrap_or_else(|_| DEFAULT_WRITER_URL.to_string());
        let home = std::env::var("HOME").unwrap_or_default();
        let watch_dir = match std::env::var("TSKR_WATCH_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from(&home).join(".claude").join("projects"),
        };
        let state_file = match std::env::var("TSKR_STATE_FILE") {
            Ok(v) => PathBuf::from(v),
            Err(_) => PathBuf::from(&home).join(".tskr").join("state.json"),
        };
        let poll_interval_secs = match std::env::var("TSKR_POLL_INTERVAL_SECS") {
            Ok(v) => v
                .parse::<u64>()
                .with_context(|| format!("TSKR_POLL_INTERVAL_SECS: not a u64: {v}"))?,
            Err(_) => DEFAULT_POLL_INTERVAL_SECS,
        };
        let author = std::env::var("TSKR_AUTHOR").ok();
        let repo = std::env::var("TSKR_REPO").ok();
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            writer_url,
            watch_dir,
            state_file,
            poll_interval_secs,
            author,
            repo,
            host,
        })
    }
}

// last_uploaded_event_index: -1 means nothing has been uploaded yet for this file.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct State {
    pub files: BTreeMap<String, FileState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileState {
    pub last_uploaded_event_index: i64,
}

pub(crate) fn load_state(path: &Path) -> anyhow::Result<State> {
    if !path.exists() {
        return Ok(State::default());
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("read state file {} failed", path.display()))?;
    if bytes.is_empty() {
        return Ok(State::default());
    }
    let state: State = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse state file {} failed", path.display()))?;
    Ok(state)
}

pub(crate) fn save_state(path: &Path, state: &State) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create_dir_all {} failed", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(state)?;
    {
        let mut f = std::fs::File::create(&tmp)
            .with_context(|| format!("create temp state file {} failed", tmp.display()))?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {} failed", tmp.display(), path.display()))?;
    Ok(())
}

pub(crate) fn tail_after(bytes: &[u8], skip_lines: usize) -> &[u8] {
    if skip_lines == 0 {
        return bytes;
    }
    let mut seen = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            seen += 1;
            if seen == skip_lines {
                return &bytes[i + 1..];
            }
        }
    }
    &[]
}

fn count_lines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|b| **b == b'\n').count()
}

fn walk_jsonl(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(dir = %dir.display(), error = %e, "read_dir failed");
                continue;
            }
        };
        for entry in read.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "file_type failed");
                    continue;
                }
            };
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

async fn upload_chunk(
    http: &reqwest::Client,
    upload_url: &str,
    body: Vec<u8>,
    author: &str,
    repo: &str,
    host: &str,
    start_event_index: i64,
) -> Result<(), UploadError> {
    let start = if start_event_index < 0 {
        0
    } else {
        start_event_index
    };
    let resp = http
        .post(upload_url)
        .header("Content-Type", "application/x-ndjson")
        .header("X-Tskr-Author", author)
        .header("X-Tskr-Repo", repo)
        .header("X-Tskr-Host", host)
        .header("X-Tskr-Start-Event-Index", start.to_string())
        .body(body)
        .send()
        .await
        .map_err(UploadError::Network)?;

    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let code = status.as_u16();
    let body = resp.text().await.unwrap_or_default();
    if status.is_server_error() {
        Err(UploadError::Server { status: code, body })
    } else {
        Err(UploadError::Client { status: code, body })
    }
}

async fn process_file(
    http: &reqwest::Client,
    upload_url: &str,
    path: &Path,
    state: &mut State,
    author: &str,
    repo_override: Option<&str>,
    host: &str,
) -> anyhow::Result<bool> {
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "read failed");
            return Ok(false);
        }
    };
    let line_count = count_lines(&bytes);
    let key = path.to_string_lossy().to_string();
    let last_uploaded = state
        .files
        .get(&key)
        .map(|f| f.last_uploaded_event_index)
        .unwrap_or(-1);

    if (line_count as i64 - 1) <= last_uploaded {
        return Ok(false);
    }
    let skip = (last_uploaded + 1) as usize;
    let tail = tail_after(&bytes, skip);
    if tail.is_empty() {
        return Ok(false);
    }

    let repo = repo_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| detect_repo(path));
    let start_event_index = last_uploaded + 1;

    let mut attempt: u32 = 0;
    let mut delay_secs: u64 = 1;
    loop {
        attempt += 1;
        match upload_chunk(
            http,
            upload_url,
            tail.to_vec(),
            author,
            &repo,
            host,
            start_event_index,
        )
        .await
        {
            Ok(()) => {
                let new_index = line_count as i64 - 1;
                state.files.insert(
                    key,
                    FileState {
                        last_uploaded_event_index: new_index,
                    },
                );
                tracing::info!(
                    path = %path.display(),
                    start_event_index,
                    new_last_index = new_index,
                    "uploaded"
                );
                return Ok(true);
            }
            Err(UploadError::Client { status, body }) => {
                tracing::warn!(
                    path = %path.display(),
                    status,
                    body = %body,
                    "client error from writer; skipping"
                );
                return Ok(false);
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    tracing::warn!(
                        path = %path.display(),
                        attempts = attempt,
                        error = %e,
                        "giving up after retries; will retry next poll"
                    );
                    return Ok(false);
                }
                tracing::warn!(
                    path = %path.display(),
                    attempt,
                    error = %e,
                    delay_secs,
                    "upload failed; backing off"
                );
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                delay_secs = (delay_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    if let Some(parent) = cfg.state_file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create_dir_all {} failed", parent.display()))?;
    }
    let mut state = load_state(&cfg.state_file)?;
    let author = cfg.author.clone().unwrap_or_else(detect_author);
    let http = reqwest::Client::new();
    let writer_url = cfg.writer_url.trim_end_matches('/').to_string();
    let upload_url = format!("{writer_url}/sessions/upload");

    tracing::info!(
        watch_dir = %cfg.watch_dir.display(),
        state_file = %cfg.state_file.display(),
        writer_url = %writer_url,
        poll_interval_secs = cfg.poll_interval_secs,
        author = %author,
        host = %cfg.host,
        "tskr-daemon starting"
    );

    // TODO(future): switch to notify-based fsevents instead of polling.
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("ctrl_c received; persisting state and exiting");
                save_state(&cfg.state_file, &state)?;
                return Ok(());
            }
            _ = poll_once(
                &http,
                &upload_url,
                &cfg.watch_dir,
                &cfg.state_file,
                &mut state,
                &author,
                cfg.repo.as_deref(),
                &cfg.host,
                cfg.poll_interval_secs,
            ) => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn poll_once(
    http: &reqwest::Client,
    upload_url: &str,
    watch_dir: &Path,
    state_file: &Path,
    state: &mut State,
    author: &str,
    repo_override: Option<&str>,
    host: &str,
    poll_interval_secs: u64,
) {
    let files = match walk_jsonl(watch_dir) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, "walk failed");
            tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
            return;
        }
    };
    for path in files {
        match process_file(http, upload_url, &path, state, author, repo_override, host).await {
            Ok(true) => {
                if let Err(e) = save_state(state_file, state) {
                    tracing::error!(error = %e, "failed to persist state");
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "process_file failed");
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_after_skips_lines() {
        let bytes = b"a\nb\nc\nd\n";
        assert_eq!(tail_after(bytes, 0), &b"a\nb\nc\nd\n"[..]);
        assert_eq!(tail_after(bytes, 2), &b"c\nd\n"[..]);
        assert_eq!(tail_after(bytes, 4), &b""[..]);
    }

    #[test]
    fn state_json_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let mut state = State::default();
        state.files.insert(
            "/foo".to_string(),
            FileState {
                last_uploaded_event_index: 5,
            },
        );
        state.files.insert(
            "/bar".to_string(),
            FileState {
                last_uploaded_event_index: -1,
            },
        );
        save_state(&path, &state).unwrap();
        let loaded = load_state(&path).unwrap();
        assert_eq!(loaded, state);
    }
}
