#!/usr/bin/env bash
set -euo pipefail

# scripts/smoke.sh -- Milestone 1 acceptance test.
# See scripts/README.md for usage.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
# shellcheck source=lib.sh
source "${script_dir}/lib.sh"

teardown=1
skip_cli="${TSKR_SKIP_CLI:-0}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-teardown) teardown=0 ;;
        --skip-cli)    skip_cli=1 ;;
        --no-skip-cli) skip_cli=0 ;;
        -h|--help)
            cat <<EOF
Usage: $0 [--no-teardown] [--skip-cli|--no-skip-cli]

Brings up the docker-compose stack, creates the MinIO bucket, backfills the
fixture sessions via the tskr CLI, and (unless --skip-cli) runs \`tskr search\`
and \`tskr show\` against the result.

Environment:
  TSKR_SKIP_CLI   "0" (default) runs CLI assertions; "1" skips them.
EOF
            exit 0 ;;
        *) tskr::die "unknown arg: $1" ;;
    esac
    shift
done

cleanup() {
    if [[ "${teardown}" == "1" ]]; then
        tskr::log "tearing down docker compose stack"
        (cd "${repo_root}" && docker compose down -v) || true
    else
        tskr::log "--no-teardown set; leaving stack running"
    fi
}
trap cleanup EXIT

tskr::require_cmd docker curl jq aws cargo

tskr::log "bringing up docker compose stack"
(cd "${repo_root}" && docker compose up -d --wait)

MINIO_ENDPOINT="http://localhost:9100"
WRITER_ENDPOINT="http://localhost:8090"
BUCKET="tskr"

tskr::log "creating MinIO bucket: ${BUCKET}"
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
AWS_REGION=us-east-1 \
    aws --endpoint-url "${MINIO_ENDPOINT}" s3api create-bucket \
        --bucket "${BUCKET}" 2>/dev/null \
    || tskr::log "bucket ${BUCKET} already exists or unreachable; continuing"

tskr::log "waiting for tskr-writer /-/ready"
tskr::wait_http "${WRITER_ENDPOINT}/-/ready" 60

tskr::log "building tskr CLI in release mode"
(cd "${repo_root}" && cargo build -p tskr-cli --release)
TSKR_BIN="${repo_root}/target/release/tskr"
[[ -x "${TSKR_BIN}" ]] || tskr::die "tskr binary not found at ${TSKR_BIN}"

fixture_dir="${repo_root}/tests/fixtures/sessions"
[[ -d "${fixture_dir}" ]] || tskr::die "fixture directory missing: ${fixture_dir}"

tskr::log "backfilling fixtures via tskr CLI"
"${TSKR_BIN}" backfill "${fixture_dir}" --author smoketest@example.com --repo tskr

if [[ "${skip_cli}" == "1" ]]; then
    tskr::log "--skip-cli set; skipping CLI search/show assertions"
else
    # vector-writer flushes on an interval; let it absorb the backfill before searching.
    tskr::log "sleeping 3s for vector-writer flush before search"
    sleep 3

    tskr::log "running tskr search 'Linux'"
    "${TSKR_BIN}" search "Linux" | tee /tmp/tskr-smoke-search.out
    grep -F "session=00000000-0000-0000-0000-000000000001" /tmp/tskr-smoke-search.out \
        || tskr::die "search did not return short-bug session"

    tskr::log "running tskr show short-bug --at-event 1"
    "${TSKR_BIN}" show 00000000-0000-0000-0000-000000000001 --at-event 1 | tee /tmp/tskr-smoke-show.out
    grep -F "Linux" /tmp/tskr-smoke-show.out \
        || tskr::die "show did not include 'Linux' from event 1"
fi

tskr::log "smoke test complete"
