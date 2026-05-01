# scripts/lib.sh -- shared bash helpers for tskr scripts.
#
# This file is intended to be sourced, not executed:
#
#     source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
#
# The caller is responsible for enabling strict mode (set -euo pipefail).
# Do NOT enable strict mode here -- it would leak into the caller's shell.

# Print a timestamped log line to stderr.
tskr::log() {
    local msg="${*:-}"
    local ts
    ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    printf '[smoke] %s %s\n' "${ts}" "${msg}" >&2
}

# Log a message and exit non-zero.
tskr::die() {
    tskr::log "FATAL: ${*:-unknown error}"
    exit 1
}

# Echo the absolute path to the repo root, computed relative to this file.
tskr::repo_root() {
    cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

# Poll <url> with curl every 2s until it returns success, or die after
# <max_seconds>.
#
# Usage: tskr::wait_http <url> <max_seconds>
tskr::wait_http() {
    local url="${1:-}"
    local max_seconds="${2:-60}"
    [[ -n "${url}" ]] || tskr::die "tskr::wait_http: url is required"

    local deadline=$(( SECONDS + max_seconds ))
    while (( SECONDS < deadline )); do
        if curl -fsS "${url}" -o /dev/null 2>/dev/null; then
            tskr::log "ready: ${url}"
            return 0
        fi
        sleep 2
    done
    tskr::die "timed out after ${max_seconds}s waiting for ${url}"
}

# Verify each listed command is on PATH; die with a list of any missing ones.
#
# Usage: tskr::require_cmd docker curl jq aws
tskr::require_cmd() {
    local missing=()
    local cmd
    for cmd in "$@"; do
        if ! command -v "${cmd}" >/dev/null 2>&1; then
            missing+=( "${cmd}" )
        fi
    done
    if (( ${#missing[@]} > 0 )); then
        tskr::die "missing required commands: ${missing[*]}"
    fi
}
