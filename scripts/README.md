# scripts/

End-to-end test scripts for tskr. Today this is just the Milestone 1 smoke
test, which exercises the docker-compose stack from MinIO up through the
`tskr-writer` HTTP ingestion endpoint.

## smoke.sh

`scripts/smoke.sh` is the Milestone 1 acceptance test. In order, it:

1. Validates required host commands are on PATH (`docker`, `curl`, `jq`, `aws`).
2. Brings the full stack up via `docker compose up -d --wait` from the repo root.
3. Creates the `tskr` bucket in MinIO via `aws s3api create-bucket`
   (idempotent: a pre-existing bucket is logged and skipped).
4. Polls `http://localhost:8090/-/ready` until `tskr-writer` is healthy
   (max 60s).
5. Uploads each `tests/fixtures/sessions/*.jsonl` fixture to
   `POST /sessions/upload` on `tskr-writer` with the headers
   `Content-Type: application/x-ndjson`, `X-Tskr-Author`, `X-Tskr-Repo`,
   `X-Tskr-Host`. Each response is pretty-printed via `jq`.
6. By default (iter 3): logs that CLI assertions are skipped and exits 0.
   Once the CLI lands in iter 6/7, this step will run `tskr search` and
   `tskr show` against the uploaded data.
7. On exit: tears the stack down with `docker compose down -v`, unless
   `--no-teardown` was passed.

### Usage

```
./scripts/smoke.sh                       # default: bring up, upload, tear down
./scripts/smoke.sh --no-teardown         # leave the stack running for debugging
TSKR_SKIP_CLI=0 ./scripts/smoke.sh       # once iter 6/7 lands, run CLI asserts
./scripts/smoke.sh --no-skip-cli         # equivalent to TSKR_SKIP_CLI=0
./scripts/smoke.sh --help                # usage text
```

As of iter 3 the CLI portion is stubbed via `TSKR_SKIP_CLI=1` (the default).
Passing `--no-skip-cli` or `TSKR_SKIP_CLI=0` will currently exit non-zero
with a TODO message; this is intentional until iter 7 wires the CLI in.

### Required host commands

- `docker` (with the `compose` plugin)
- `curl`
- `jq`
- `aws` (the AWS CLI v2)

### Service ports

| Service          | Host port | Container port | Notes                  |
|------------------|-----------|----------------|------------------------|
| MinIO API (S3)   | 9100      | 9000           | `minioadmin`/`minioadmin` |
| MinIO console    | 9101      | 9001           | browser UI             |
| embedding-server | 9000      | 9000           |                        |
| vector-writer    | 8080      | 8080           | `/-/ready`             |
| vector-reader    | 8081      | 8080           | `/-/ready`             |
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
