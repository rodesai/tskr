# scripts/

End-to-end test scripts for tskr. Today this is just the Milestone 1 smoke
test, which exercises the docker-compose stack from MinIO up through the
`tskr-writer` HTTP ingestion endpoint.

## smoke.sh

`scripts/smoke.sh` is the Milestone 1 acceptance test. In order, it:

1. Validates required host commands are on PATH (`docker`, `curl`, `jq`).
2. Brings the full stack up via `docker compose up -d --wait` from the repo root.
3. Polls `http://localhost:8090/-/ready` until `tskr-writer` is healthy
   (max 60s).
4. Uploads each `tests/fixtures/sessions/*.jsonl` fixture to
   `POST /sessions/upload` on `tskr-writer` with the headers
   `Content-Type: application/x-ndjson`, `X-Tskr-Author`, `X-Tskr-Repo`,
   `X-Tskr-Host`. Each response is pretty-printed via `jq`.
5. Runs `tskr search` and `tskr show` against the uploaded data to assert
   the fixture sessions are searchable end-to-end.
6. On exit: tears the stack down with `docker compose down -v`, unless
   `--no-teardown` was passed.

`tskr-writer` creates the MinIO bucket on startup if it doesn't exist.

### Usage

```
./scripts/smoke.sh                       # default: bring up, upload, run CLI, tear down
./scripts/smoke.sh --no-teardown         # leave the stack running for debugging
TSKR_SKIP_CLI=1 ./scripts/smoke.sh       # debug-only: skip CLI search/show assertions
./scripts/smoke.sh --skip-cli            # equivalent to TSKR_SKIP_CLI=1
./scripts/smoke.sh --help                # usage text
```

By default `smoke.sh` drives the real `tskr` CLI for `search` and `show`
assertions. `--skip-cli` / `TSKR_SKIP_CLI=1` is for debugging only (e.g.
when iterating on the writer in isolation).

### Required host commands

- `docker` (with the `compose` plugin)
- `curl`
- `jq`
- `cargo`

### Service ports

| Service          | Host port | Container port | Notes                  |
|------------------|-----------|----------------|------------------------|
| MinIO API (S3)   | 9100      | 9000           | `minioadmin`/`minioadmin` |
| MinIO console    | 9101      | 9001           | browser UI             |
| embedding-server | 9000      | 9000           |                        |
| vector-writer    | 8080      | 8080           | `/-/ready`             |
| vector-reader    | 18081     | 8080           | `/-/ready`             |
| tskr-writer      | 8090      | 8090           | `/-/ready`, `/sessions/upload` |

## lib.sh

`scripts/lib.sh` is a sourced bash library used by `smoke.sh`. It provides:

- `tskr::log <msg>` -- timestamped stderr log line
- `tskr::die <msg>` -- log and exit 1
- `tskr::repo_root` -- echo the absolute repo root
- `tskr::wait_http <url> <max_seconds>` -- poll a URL until it returns 2xx
- `tskr::require_cmd <cmd>...` -- assert each command is on PATH

The library deliberately does NOT enable `set -euo pipefail`; that is the
caller's responsibility.
