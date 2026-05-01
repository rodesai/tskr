# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [x] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer
- [x] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready
- [x] Chunking + embedding + S3 segment write + vector upsert pipeline
- [x] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl
- [x] Vector schema (writer-side upsert client iter 4; rows produced+upserted by pipeline iter 5)
- [x] tskr CLI: search / list / show / backfill (iter 6, ba6bb74); daemon wired iter 7
- [~] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json (iter 7 — poll-based foreground)
- [x] Fixture sessions under tests/fixtures/sessions/
- [~] scripts/smoke.sh end-to-end test (iter 7 wires real CLI; first full run pending Reviewer execution)
- [ ] README.md "5 minutes to first search" (iter 8)

## Notes
- Iter 1–6 committed (latest ba6bb74). 23 tests pass workspace-wide.
- Iter 7 plan: tskr-daemon as a library (`tskr_daemon::run(Config)`) + thin binary. Polling every 2s. Per-file offsets in `~/.tskr/state.json`. Backoff 1s/2s/4s capped at 30s on 5xx; 4xx logs and skips.
- Iter 7 wiring: `tskr-cli/Cargo.toml` adds `tskr-daemon = { path = "../tskr-daemon" }`. `commands/daemon.rs` becomes async, delegates to `tskr_daemon::run` for Start; Status/Stop print milestone-1 explanations and exit 0.
- Iter 7 smoke.sh: drops `TSKR_SKIP_CLI=1` default (now `0`), keeps `--skip-cli` for debugging. Replaces curl loop with `tskr backfill`. Builds CLI once via `cargo build -p tskr-cli --release` then uses target/release/tskr directly. Sleeps 3s after backfill for vector flush.
- Search assertion: `tskr search "Linux" | grep -F "session=00000000-0000-0000-0000-000000000001"`. Show assertion: `tskr show 00000000-0000-0000-0000-000000000001 --at-event 1 | grep -F "Linux"`.
- Daemon repo: parent-dir basename of each `*.jsonl` file under `~/.claude/projects/`. Author: `git config user.email` cached at start, fallback `unknown@local`. Host: `$HOSTNAME` or `unknown`.
- Daemon dependencies: `tokio`, `reqwest`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`. NO `notify` (poll-based for milestone 1).
- Deferred for iter 8: README.

## Last reviewer rationale
(iter 6) approve — tskr CLI committed as ba6bb74. Daemon stub deferred to iter 7 (now in flight).
