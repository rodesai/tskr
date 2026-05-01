# tskr

`tskr` makes the engineering team's collective Claude Code usage discoverable.
Every session is uploaded to a central service, persisted to object storage,
and indexed in [opendata vector] so anyone can semantically search past
conversations or browse them by time window.

[opendata vector]: ../opendata/vector

## 5 minutes to first search

### Prerequisites

- `docker` (with the `compose` plugin)
- `curl`
- `jq`
- `cargo` (stable Rust; the toolchain is pinned via `rust-toolchain.toml`)

### One command

```
./scripts/smoke.sh
```

This brings up the full Milestone 1 stack (MinIO, the embedding server,
opendata vector writer/reader, and `tskr-writer`) via `docker compose`,
creates the `tskr` bucket on MinIO, builds the `tskr` CLI in release mode,
backfills the canned fixture sessions under `tests/fixtures/sessions/`, then
runs `tskr search "Linux"` and `tskr show <session_id> --at-event 1` against
the indexed data. On exit it tears the stack down with `docker compose down
-v` unless `--no-teardown` is passed.

### Sample output

```
[tskr] running tskr search 'Linux'
[author=smoketest@example.com repo=tskr] 2024-01-01T00:00:01Z session=00000000-0000-0000-0000-000000000001 event=1 score=0.41
  user: how do I check the kernel version on Linux
[tskr] running tskr show 00000000-0000-0000-0000-000000000001 --at-event 1
session 00000000-0000-0000-0000-000000000001  author=smoketest@example.com  repo=tskr
  [0] user      how do I check the kernel version on Linux
> [1] assistant Run `uname -a` on Linux to print the kernel version and architecture.
  [2] user      thanks
[tskr] smoke test complete
```

## Architecture

```
~/.claude/projects/*.jsonl            tskr-writer (axum)
        |                              | parses NDJSON events
   tskr-daemon  --POST /sessions/upload-->  classifies + chunks
   (poll ~2s)                           |  embeds via fastembed server
        ^                               |  writes 10-event segments to S3
        |                               |  upserts rows to opendata vector
   tskr CLI <---vector-reader + S3-----+
   (search / list / show)
```

`tskr-writer` is the single ingest endpoint. The daemon scans Claude session
files on the laptop and POSTs new event tails. The CLI talks to
`vector-reader` for search and metadata listing, and to S3 (MinIO in
Milestone 1) to fetch the raw events behind `show`.

## Components

**`tskr-core`** — shared library. Holds the JSONL event taxonomy, the
chunk/render rules (assistant text, real `user` turns, `user` tool_result
blocks, and `system.away_summary` become indexed rows; everything else is
S3-only), the segment/manifest schema, and the request/response types used
by both writer and CLI.

**`tskr-writer`** — Rust HTTP service (`axum` + `tokio`) exposing
`POST /sessions/upload`, `GET /healthz`, and `GET /-/ready`. Parses the
uploaded NDJSON tail, classifies and renders each event, embeds the
indexable ones via the fastembed server, persists raw events as 10-event
segments to S3 (`s3://tskr/sessions/<session_id>/seg-<00000>.jsonl` plus a
`manifest.json`), and upserts vector rows to opendata vector. Idempotent on
`(session_id, event_index)`.

**`tskr-cli`** — single static binary built from `crates/tskr-cli`. Provides
`search`, `list`, `show`, `backfill`, and `daemon` subcommands (see below).
`backfill` walks a directory of `.jsonl` fixtures or real Claude project
dumps and uploads each through the writer.

**`tskr-daemon`** — per-laptop uploader. Polls the watch directory roughly
every 2 seconds, tracks `(path, last_uploaded_event_index)` in a JSON state
file, reads the new tail of each changed file, and POSTs it to the writer
with `X-Tskr-Author`, `X-Tskr-Repo`, `X-Tskr-Host`, and
`X-Tskr-Start-Event-Index` headers. Retries with exponential backoff on 5xx
(up to 6 attempts, capped at 30s).

The `docker-compose.yml` at the repo root brings up MinIO (S3-compatible
object store), the opendata vector writer and reader, the fastembed
embedding server (`all-MiniLM-L6-v2`, 384 dims), and `tskr-writer` itself.

## CLI usage

```bash
tskr search <query> [--repo R] [--author A] [--since 7d] [--limit 10]
tskr list   [--repo R] [--author A] [--since 7d] [--limit 10]
tskr show   <session_id> [--at-event N]
tskr backfill <dir> [--author A] [--repo R]
tskr daemon start | status | stop
```

`--since` accepts durations like `7d`, `24h`, `30m`, `90s`. `tskr search`
embeds the query, queries `vector-reader` with metadata filters, and prints
matches with a session id and event index that can be passed straight to
`tskr show`. `tskr list` is metadata-only — no embedding round-trip. `tskr
show` loads the manifest, fetches the segment containing `--at-event` (and
neighbors), and renders the conversation around that event.

The daemon picks up overrides from the environment: `TSKR_WRITER_URL`,
`TSKR_WATCH_DIR`, `TSKR_STATE_FILE`, `TSKR_POLL_INTERVAL_SECS`,
`TSKR_AUTHOR`, `TSKR_REPO`, and `HOSTNAME`.

## Running against your real sessions

After the stack is up (`docker compose up -d --wait` from the repo root, or
leave it running with `./scripts/smoke.sh --no-teardown`):

```
tskr daemon start
```

Defaults:

- Watch directory: `~/.claude/projects` (every `*.jsonl` under it,
  recursively).
- State file: `~/.tskr/state.json`. Records the last uploaded event index
  per session file so reruns only ship new events.
- Poll interval: ~2 seconds. Filesystem-watch (`notify`) is on the roadmap;
  Milestone 1 ships polling.
- Author: `git config user.email`, falling back to `unknown@local`.
- Writer URL: `http://localhost:8090`.

Stop with Ctrl-C; the daemon flushes state on exit.

## Local development

```
cargo build --workspace
cargo test --workspace        # 25 tests pass as of this iteration
cargo run -p tskr-cli -- --help
```

The workspace pins the Rust toolchain via `rust-toolchain.toml`, so a fresh
checkout with `rustup` installed will pick up the right compiler
automatically. Crate layout:

- `crates/tskr-core` — shared types and event/chunk logic.
- `crates/tskr-writer` — HTTP ingest service binary.
- `crates/tskr-cli` — `tskr` CLI binary.
- `crates/tskr-daemon` — uploader binary.

Useful scripts live under `scripts/` (see `scripts/README.md` for the smoke
test details and service ports).

## Project status

Milestone 1 — local end-to-end — is the current target: `docker compose up`
on a laptop, plus the daemon and CLI on the host, ending in `tskr search`
and `tskr show` working against your own past sessions.

Deferred to later milestones:

- Multi-tenancy and auth, Pulumi-driven cloud deploy, daemon as a managed
  service (launchd / systemd) — Milestone 2.
- End-to-end encryption of S3 segments (per-engineer keys held in macOS
  Keychain; vector index stays plaintext) — Milestone 2.
- Embedder bake-off against a real eval set, sub-event chunking for long
  tool results, and chunk-level dedup across sessions — Milestone 3.
- MCP server exposing `tskr.search` and a `/tskr` slash command for running
  Claude sessions — Milestone 4.
- A `ratatui` + `crossterm` TUI for `tskr show` with scroll and lazy
  segment loading — currently rendered as plain text.

Full design and milestone breakdown: see [`PLAN.md`](PLAN.md).

## License

MIT OR Apache-2.0
