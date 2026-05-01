#!/usr/bin/env bash
set -euo pipefail

# scripts/smoke.sh -- Milestone 1 acceptance test.
# See scripts/README.md for usage.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
# shellcheck source=lib.sh
source "${script_dir}/lib.sh"

teardown=1
skip_cli="${TSKR_SKIP_CLI:-1}"  # default ON for iter 3; flipped OFF in iter 7.

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-teardown) teardown=0 ;;
        --skip-cli)    skip_cli=1 ;;
        --no-skip-cli) skip_cli=0 ;;
        -h|--help)
            cat <<EOF
Usage: $0 [--no-teardown] [--skip-cli|--no-skip-cli]

Brings up the docker-compose stack, creates the MinIO bucket, uploads each
fixture session to tskr-writer, and (when --no-skip-cli) runs \`tskr search\`
and \`tskr show\` against the result.

Environment:
  TSKR_SKIP_CLI   "1" (default) skips CLI assertions; "0" requires CLI.
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

tskr::require_cmd docker curl jq aws

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

fixture_dir="${repo_root}/tests/fixtures/sessions"
shopt -s nullglob
fixtures=( "${fixture_dir}"/*.jsonl )
shopt -u nullglob
[[ ${#fixtures[@]} -gt 0 ]] || tskr::die "no fixtures under ${fixture_dir}"

for fx in "${fixtures[@]}"; do
    name="$(basename "${fx}" .jsonl)"
    tskr::log "uploading fixture: ${name}"
    curl -fsS \
        -H "Content-Type: application/x-ndjson" \
        -H "X-Tskr-Author: smoketest@example.com" \
        -H "X-Tskr-Repo: tskr" \
        -H "X-Tskr-Host: localhost" \
        --data-binary "@${fx}" \
        "${WRITER_ENDPOINT}/sessions/upload" \
        | jq .
done

if [[ "${skip_cli}" == "1" ]]; then
    tskr::log "--skip-cli set (iter 3 default); skipping CLI assertions"
    # TODO(iter 6): wire `tskr search "Linux"` and assert short-bug appears.
    # TODO(iter 6): wire `tskr show <session_id> --at-event N` headlessly.
else
    # TODO(iter 7): replace these with real CLI invocations.
    tskr::die "--no-skip-cli not yet supported; CLI lands in iter 6 and is wired into smoke.sh in iter 7"
fi

tskr::log "smoke test complete"
