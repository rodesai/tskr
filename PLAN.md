# tskr — Plan

`tskr` makes the engineering team's collective Claude Code usage *discoverable*.
Every session that any team member has with Claude is uploaded to a central
service, stored durably in object storage, and indexed in [opendata vector] so
that anyone can semantically search past conversations or browse them by time
window.

[opendata vector]: ../opendata/vector

## Why

Claude session logs already live on each engineer's laptop under
`~/.claude/projects/<project-slug>/<session-id>.jsonl`, but they are:

1. **Not searchable.** Each engineer can only see their own. There is no way
   to ask "has anyone else worked through a similar bug?" or "how did the team
   land on the current vector index design?".
2. **Not browsable.** A session is one giant JSONL file. Even the engineer who
   ran the session can't easily skim it after the fact.

`tskr` fixes both: ingest every session, chunk and embed each event, persist
the raw segments to S3 for fast scrollback, and expose a CLI that turns a
search query into a navigable view of the original conversation.

It is also a high-signal dogfooding target for opendata vector — workload is
small but real, traffic is bursty, and the read pattern (semantic search +
metadata filters + range queries by time) exercises most of the index.

## System Design

```
┌──────────────────────┐
│ engineer's laptop    │
│                      │   POST /sessions/upload
│  ~/.claude/projects/ │ ─────────────────────────────┐
│         ▲            │                              │
│         │ scans      │                              ▼
│  ┌──────┴──────┐     │                  ┌───────────────────────┐
│  │ tskr-daemon │     │                  │ tskr-writer (HTTP)    │
│  └─────────────┘     │                  │ ─ parses JSONL events │
│                      │                  │ ─ chunks → embeds     │
│  ┌─────────────┐     │                  │ ─ writes S3 segments  │
│  │ tskr CLI    │     │  search / show   │ ─ upserts to vector   │
│  └──────┬──────┘     │ ───────────────► └───────────┬───────────┘
└─────────┼────────────┘                              │
          │                                           ▼
          │                ┌───────────────┐   ┌──────────────┐
          ├───────────────►│ vector-reader │   │ S3 (MinIO    │
          │   semantic     │ (opendata)    │   │ in milestone │
          │   search +     └───────┬───────┘   │ 1)           │
          │   metadata             │           └──────┬───────┘
          │                        │                  │
          │                        ▼                  ▼
          │             ┌──────────────────────────────────┐
          └────────────►│ raw session segments (10 events  │
                fetch   │ per file, keyed by session id)   │
                        └──────────────────────────────────┘
```

### Components

#### 1. `tskr-writer` — ingest service

HTTP service that accepts session uploads. The single write endpoint is the
source of truth for both S3 and the vector index.

**API (milestone 1):**

| Method | Path | Body | Purpose |
|--------|------|------|---------|
| `POST` | `/sessions/upload` | NDJSON of session events plus `{author, repo, host}` headers | Append the supplied events to the session. Idempotent on `(session_id, event_index)`. |
| `GET`  | `/healthz`         | —                                              | Liveness. |
| `GET`  | `/-/ready`         | —                                              | Ready when MinIO + vector + embedder are reachable. |

**Per-upload pipeline:**

1. **Parse.** Each line is one event from `~/.claude/projects/.../<session>.jsonl`.
   The session ID is taken from the file name (and validated against the
   `sessionId` field on events that carry one). Events are numbered 0..N in the
   order they appear in the file.
2. **Classify and render.** Every event is persisted to S3 verbatim, but only
   *interesting* events become vector rows. The taxonomy (derived from real
   `~/.claude/projects/*.jsonl` data):

   | Event | S3 segment | Vector row | Embedded text (`text` field) |
   |---|---|---|---|
   | `assistant` | yes | yes | concat of `text` content blocks; for each `tool_use` block append a one-line summary `tool_use: <name>(<truncated input ~200 chars>)`. `thinking` blocks are skipped (their `signature`-encoded payload is not human-readable). |
   | `user` (string content) | yes | yes | the string, *unless* it starts with `<local-command-caveat>` (skip — these are local-command echoes, not real user turns). |
   | `user` (tool_result list) | yes | yes | concat of `tool_result` block content, each block truncated to ~4KB to bound chunk size. |
   | `system` `subtype=away_summary` | yes | yes | the summary text (high-signal — it's Claude's own recap of what happened while the user was away). |
   | `system` other subtypes | yes | no | — |
   | `permission-mode`, `last-prompt`, `attachment`, `file-history-snapshot`, `queue-operation` | yes | no | — |

3. **Chunk.** One chunk per indexed event for milestone 1. Each chunk carries:
    - `session_id` (indexed)
    - `event_index` — integer offset within the session (indexed)
    - `segment_index = event_index / 10` (indexed)
    - `author` — engineer who ran the session (indexed)
    - `repo` — derived from project slug (indexed)
    - `model` — pulled from `assistant.message.model` when present (indexed)
    - `role` — one of `user`, `assistant`, `tool_result`, `summary` (indexed)
    - `timestamp` — RFC3339 / unix millis (indexed; supports range queries)
    - `text` — the rendered text from step 2 (not indexed)
4. **Embed.** Calls the same fastembed-based embedding server used by the
   vector quickstart (`all-MiniLM-L6-v2`, 384 dims). Empty/whitespace-only
   chunks are skipped. Embedder choice is revisited in Milestone 3 against
   an eval set; changing dimensions requires a full reindex.
5. **Persist raw segments.** Events are bucketed into 10-event segments. Each
   segment is written to:

   ```
   s3://tskr/sessions/<session_id>/seg-<00000>.jsonl
   ```

   Five-digit zero-padded prefix means a simple `aws s3 ls` over the prefix
   returns segments in event order. A separate
   `s3://tskr/sessions/<session_id>/manifest.json` records `{author, repo,
   started_at, last_event_index, segment_count}` so the CLI can render the
   session header without fetching every segment.
6. **Upsert vector rows.** One vector row per chunk, with the metadata above,
   into the `tskr` vector index running on opendata vector.

The service is intentionally idempotent: re-uploading the same session is a
no-op modulo new events at the tail. This is important because the daemon
will redeliver on retry and on every poll cycle.

#### 2. `tskr-daemon` — per-laptop uploader

Lightweight Python (or Rust) process that:

- Watches `~/.claude/projects/**/*.jsonl`.
- For each file, tracks `(path, last_uploaded_event_index)` in
  `~/.tskr/state.json`.
- On change, reads the new tail and `POST`s it to `tskr-writer`.
- Tags every upload with the engineer's identity (`author = git config
  user.email` by default) and the host name.
- Backs off on 5xx; retries until success.

Milestone 1 runs the daemon as a foreground process started by `tskr daemon
start`. Cron / launchd integration is later.

#### 3. `tskr` — CLI

```
tskr search "<query>" [--repo X] [--author Y] [--since 7d] [--limit 10]
tskr list   [--since 7d] [--repo X] [--author Y]
tskr show   <session_id> [--at-event N]
tskr daemon start | status | stop
```

- **`search`** embeds the query via the embedding server, runs a vector search
  against `vector-reader` with metadata filters, and prints the top matches
  as `[author/repo] <ts> "first 100 chars of event"  (score=...)`.
  Each result has a numeric handle the user can `tskr show` directly.
- **`list`** is a metadata-only query — no embedding needed. It groups by
  `session_id`, sorted by `started_at` descending. Useful when you remember
  *roughly when* but not *what*.
- **`show`** loads the manifest from S3 to learn the segment count, fetches
  the segment containing `--at-event` (and a couple on either side), and
  renders the conversation in a TUI (`textual` for milestone 1) scrolled to
  that event with up/down/page-up/page-down navigation. Additional segments
  are lazy-loaded on scroll.

### Data layout — S3

```
tskr/
└── sessions/
    └── <session_id>/
        ├── manifest.json              { author, repo, started_at,
        │                                last_event_index, segment_count, ... }
        ├── seg-00000.jsonl            events 0..9
        ├── seg-00001.jsonl            events 10..19
        └── ...
```

### Data layout — opendata vector

Single index named `tskr`, dimensions=384, distance=L2 (matches the
quickstart embedder). Indexed metadata: `session_id`, `event_index`,
`segment_index`, `author`, `repo`, `model`, `role`, `timestamp`. Non-indexed
payload: `text`.

## Milestones

### Milestone 1 — local end-to-end (this is the first deliverable)

Everything runs on a single laptop with `docker compose up`, plus the daemon
and CLI on the host. Goal: I can search my own past sessions, click through
to an event, and scroll the original conversation.

In scope:

- `docker-compose.yml` that starts:
    - `minio` (single-node, local volume) — stands in for S3.
    - `embedding-server` — reused from `vector/quickstart`.
    - `vector-writer` and `vector-reader` — opendata vector, configured for
      the `tskr` schema and pointed at `minio`.
    - `tskr-writer` — Python (FastAPI) service implementing the pipeline above.
- `tskr` CLI (Python, `click` + `textual`).
- `tskr-daemon` (Python) that scans `~/.claude/projects` and uploads.
- `tskr backfill` one-shot command that sweeps every existing session under
  `~/.claude/projects` (so we have something to search the first time).
- Smoke test (`scripts/smoke.sh`) that:
    1. `docker compose up -d` and waits for ready.
    2. Runs `tskr backfill` against a fixture directory of canned sessions.
    3. Runs `tskr search "<known phrase>"` and asserts the matching session
       appears in the top result.
    4. Runs `tskr show <session_id> --at-event N` headlessly and asserts the
       rendered output contains the expected event.
- README that explains "from zero to searching your own sessions in 5
  minutes".

Out of scope for milestone 1:

- Multi-tenancy / auth.
- Sub-event chunking, summarization, dedup beyond `(session_id, event_index)`.
- Pulumi / cloud deploy.
- Daemon as a managed service (launchd / systemd unit).
- MCP server that injects search results into new Claude sessions.

### Milestone 2 — shared instance

- Pulumi program that deploys `tskr-writer`, `vector-writer`, `vector-reader`,
  and the embedding server to AWS, backed by real S3.
- Auth — at least an HMAC shared-secret per engineer so we know who uploaded
  what.
- Daemon shipped as a launchd plist (macOS) so it survives logout.
- `tskr` CLI configurable via `~/.tskr/config.toml` (endpoint, credentials).
- **End-to-end encryption of S3 segments.** See [§Privacy](#privacy).

### Milestone 3 — quality

- Sub-event chunking for long tool results / large user paste-ins.
- Per-session summary written into the manifest at upload time and surfaced
  in `tskr list`.
- Dedup of identical chunks across sessions (one vector row, multiple
  references) — measures how much context the team actually duplicates.
- Time-decay weighting in search.
- **Embedder bake-off.** Build a labeled eval set of `(query, relevant
  session_id)` pairs from real team queries; measure recall@10 across
  `all-MiniLM-L6-v2`, `BAAI/bge-small-en-v1.5`, `nomic-embed-text-v1.5`, and
  (optionally) Voyage `voyage-code-3`. Switching embedder = full reindex
  (vector dimensions are fixed in the writer config).

### Milestone 4 — closing the loop into Claude

- MCP server exposing `tskr.search` as a tool so a running Claude session can
  pull in relevant prior conversations as context.
- `/tskr` slash command that runs that search and offers to attach the top
  results to the current session's context.

## Privacy

Sessions can contain secrets (API keys, customer data in stack traces, .env
contents) and personal back-and-forth that engineers reasonably consider
private even within the company. The plan:

- **Milestone 1 has no new privacy gating.** Each engineer runs the whole
  stack on their own laptop against their own `~/.claude/projects` directory.
  Nothing leaves the host. Opting in is the act of running `tskr backfill`
  / `tskr daemon start`.
- **Milestone 2 — end-to-end encrypted S3 segments.**
    - The daemon generates a long-lived data key (per-engineer or per-team,
      TBD), stored in macOS Keychain, never sent to the writer service.
    - Before upload, the daemon encrypts each segment payload (AES-GCM) with
      that key and only ships the ciphertext + iv + key id to `tskr-writer`,
      which puts it in S3 unchanged. `tskr-writer` itself never sees
      plaintext segment bytes.
    - The vector index continues to hold **plaintext** embeddings and
      metadata (`session_id`, `author`, `repo`, `model`, `role`,
      `timestamp`, `event_index`). This is a deliberate trade-off: search
      needs them in the clear, and they're already much lower-risk than the
      raw segment text.
    - The CLI fetches ciphertext from S3, then decrypts locally using the
      same Keychain key. `tskr show` cannot render sessions encrypted under
      a key the local user doesn't hold — that's the access-control story.
    - Key scope is the open call: per-engineer keys (private journals,
      others can search metadata but not read content) vs. shared team key
      (everyone reads everyone's). Probably ship per-engineer first and add
      explicit "publish to team" later.
- **Deferred.** Pre-upload secret redaction (regex/entropy), per-row ACLs in
  vector, audit logging — all later, once we see how the team actually uses
  it.

## Other resolved questions

- **Embedder.** Pinned to `all-MiniLM-L6-v2` (384 dim) for milestones 1–2
  because it's already in the opendata vector quickstart and runs
  CPU-cheaply. Bake-off vs. larger models is a Milestone 3 deliverable
  driven by a real eval set, not vibes.
- **Event taxonomy.** Resolved in [§Components — `tskr-writer` step 2](#1-tskr-writer--ingest-service).
  Every event goes to S3; only `assistant`, real `user`, `user`-with-
  `tool_result`, and `system.away_summary` events become vector rows.
