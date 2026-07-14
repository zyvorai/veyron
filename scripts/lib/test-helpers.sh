# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# test-helpers.sh — shared library for the Veyron customer-readiness suite.
#
# Sourced (never executed) by scripts/customer-readiness.sh and the phase
# scripts under scripts/test/. Provides HTTP helpers, PASS/FAIL/WARN/SKIP result
# recording (with a machine-readable JSONL trail), cluster fact detection, wait
# helpers, and a LIFO cleanup/safety framework.
#
# Contract:
#   - NEVER use `set -e` here — the harness must survive an individual failing
#     check and still emit a report. Callers use `set -uo pipefail`.
#   - HTTP results land in the globals _CURL_BODY / _CURL_CODE. curl_api must
#     never run in a subshell (that would drop the globals).
#   - Every mutation targets $TEST_NS with a veyron-readiness-* / veyron-e2e-*
#     name and registers a cleanup immediately (owned_by_test gates every delete).
#
# Required before sourcing: BASE (https://host:port) or HOST+PORT; KEY (admin key).
# Optional: RESULTS_JSONL (append target), CURRENT_PHASE.

# shellcheck shell=bash
# Include-guard: safe to source more than once.
if [[ -n "${_VEYRON_TEST_HELPERS_LOADED:-}" ]]; then
    return 0 2>/dev/null || true
fi
_VEYRON_TEST_HELPERS_LOADED=1

# ── Config / derived globals ────────────────────────────────────────────────
HOST="${HOST:-${1:-${DEPLOY_HOST:-}}}"
PORT="${PORT:-${VEYRON_NODE_PORT:-30151}}"
KEY="${KEY:-${VEYRON_API_KEY:-Admin@321}}"
BASE="${BASE:-https://${HOST}:${PORT}}"
TEST_NS="${TEST_NS:-veyron-readiness}"
RESULTS_JSONL="${RESULTS_JSONL:-}"
CURRENT_PHASE="${CURRENT_PHASE:-general}"

# Result counters (globals; each phase script and the orchestrator read them).
PASS="${PASS:-0}"
FAIL="${FAIL:-0}"
SKIP="${SKIP:-0}"
WARN="${WARN:-0}"

# ── Colors ──────────────────────────────────────────────────────────────────
if [[ -t 1 ]]; then
    _G='\033[0;32m'; _R='\033[0;31m'; _Y='\033[0;33m'; _C='\033[0;36m'; _B='\033[1m'; _N='\033[0m'
else
    _G=''; _R=''; _Y=''; _C=''; _B=''; _N=''
fi

# ── JSON escaping (no jq) ────────────────────────────────────────────────────
# Escape a string for embedding as a JSON value: backslash, quote, control chars.
_json_escape() {
    local s="${1:-}"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/ }"
    s="${s//$'\r'/ }"
    s="${s//$'\t'/ }"
    printf '%s' "${s}"
}

# ── Result recording ────────────────────────────────────────────────────────
# section NAME — set the phase label attached to subsequent results.
section() {
    CURRENT_PHASE="$1"
    printf '\n%b── %s ──%b\n' "${_B}" "$1" "${_N}"
}

# Append one JSONL record: {phase,check,status,detail,http,reason}
_record() {
    local rstatus="$1" check="$2" detail="${3:-}" reason="${4:-}"
    [[ -z "${RESULTS_JSONL}" ]] && return 0
    printf '{"phase":"%s","check":"%s","status":"%s","detail":"%s","http":"%s","reason":"%s"}\n' \
        "$(_json_escape "${CURRENT_PHASE}")" \
        "$(_json_escape "${check}")" \
        "${rstatus}" \
        "$(_json_escape "${detail}")" \
        "$(_json_escape "${_CURL_CODE:-}")" \
        "$(_json_escape "${reason}")" \
        >> "${RESULTS_JSONL}" 2>/dev/null || true
}

pass() { printf '  %b✔%b %s\n' "${_G}" "${_N}" "$1"; PASS=$((PASS + 1)); _record pass "$1" "${2:-}"; }
fail() {
    printf '  %b✗%b %s\n' "${_R}" "${_N}" "$1"
    [[ -n "${2:-}" ]] && printf '      %s\n' "$(printf '%s' "$2" | head -c 500)"
    FAIL=$((FAIL + 1)); _record fail "$1" "${2:-}"
}
warn() {
    printf '  %b!%b %s\n' "${_Y}" "${_N}" "$1"
    [[ -n "${2:-}" ]] && printf '      %s\n' "$(printf '%s' "$2" | head -c 300)"
    WARN=$((WARN + 1)); _record warn "$1" "${2:-}"
}
# skip MSG [REASON] — REASON is a machine token (single_node, velero_absent, …).
skip() {
    printf '  %b○%b %s%b\n' "${_Y}" "${_N}" "$1" ""
    [[ -n "${2:-}" ]] && printf '      (%s)\n' "$2"
    SKIP=$((SKIP + 1)); _record skip "$1" "" "${2:-}"
}

# ── HTTP ────────────────────────────────────────────────────────────────────
# curl_api METHOD PATH [BODY] [auth=yes|no] [KEY_OVERRIDE]
#   Sets globals _CURL_BODY and _CURL_CODE. Must NOT be called in a subshell.
_CURL_CODE=""
_CURL_BODY=""
curl_api() {
    local method="$1" path="$2" body="${3:-}" auth="${4:-yes}" key_override="${5:-}"
    local url="${BASE}${path}"
    local use_key="${key_override:-${KEY}}"
    local -a args=(-skS --connect-timeout 15 --max-time 180 -X "${method}")
    if [[ "${auth}" == "yes" ]]; then
        args+=(-H "X-API-Key: ${use_key}")
    fi
    if [[ -n "${body}" ]]; then
        args+=(-H "Content-Type: application/json" -d "${body}")
    fi
    local tmp
    tmp="$(mktemp)"
    _CURL_CODE="$(curl "${args[@]}" -o "${tmp}" -w '%{http_code}' "${url}" 2>/dev/null || echo "000")"
    _CURL_BODY="$(cat "${tmp}")"
    rm -f "${tmp}"
}

# body_success [BODY]   — "success": true (defaults to last _CURL_BODY)
body_success() {
    printf '%s' "${1:-${_CURL_BODY}}" | grep -q '"success"[[:space:]]*:[[:space:]]*true'
}
# body_error_code BODY CODE
body_error_code() {
    printf '%s' "$1" | grep -qE "\"code\"[[:space:]]*:[[:space:]]*\"$2\""
}
# body_has BODY REGEX
body_has() {
    printf '%s' "$1" | grep -qE "$2"
}
# json_num BODY KEY — first numeric value for "KEY": N (no jq)
json_num() {
    printf '%s' "$1" | sed -n "s/.*\"$2\"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p" | head -1
}
# json_str BODY KEY — first string value for "KEY": "V"
json_str() {
    printf '%s' "$1" | sed -n "s/.*\"$2\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" | head -1
}

# guest_exec NS NAME PATH [ARG...] — run a command in the guest via the
# QGA-mediated GuestKit exec. The API expects {"path","arg":[...]} (NOT
# {"command":[...]}), and returns the output base64-encoded in "out-data".
# Sets _GUEST_STDOUT (decoded text) and _GUEST_RC (0 on API success).
_GUEST_STDOUT=""
_GUEST_RC=1
guest_exec() {
    local ns="$1" name="$2" path="$3"; shift 3
    local argjson="[]" a first=1
    if [[ $# -gt 0 ]]; then
        argjson="["
        for a in "$@"; do
            [[ ${first} -eq 1 ]] || argjson="${argjson},"
            argjson="${argjson}\"$(_json_escape "${a}")\""
            first=0
        done
        argjson="${argjson}]"
    fi
    curl_api POST "/api/v1/vms/${ns}/${name}/guest/exec" \
        "$(printf '{"path":"%s","arg":%s}' "${path}" "${argjson}")"
    _GUEST_STDOUT=""
    _GUEST_RC=1
    if body_success "${_CURL_BODY}"; then
        local b64
        b64="$(json_str "${_CURL_BODY}" out-data)"
        if [[ -n "${b64}" ]]; then
            _GUEST_STDOUT="$(printf '%s' "${b64}" | base64 -d 2>/dev/null || printf '%s' "${b64}" | base64 -D 2>/dev/null || printf '')"
        fi
        _GUEST_RC=0
        return 0
    fi
    return 1
}

# ── Cluster facts ───────────────────────────────────────────────────────────
NODE_COUNT="${NODE_COUNT:-}"
READY_NODES="${READY_NODES:-}"
_CAPS_JSON=""

# detect_node_count — count entries in GET /api/v1/nodes; sets NODE_COUNT.
detect_node_count() {
    curl_api GET "/api/v1/nodes"
    # Count "name" occurrences at node granularity; fall back to 0 on parse trouble.
    NODE_COUNT="$(printf '%s' "${_CURL_BODY}" | grep -oE '"name"[[:space:]]*:' | wc -l | tr -d ' ')"
    [[ -z "${NODE_COUNT}" ]] && NODE_COUNT=0
    printf '%s' "${NODE_COUNT}"
}
detect_ready_nodes() {
    curl_api GET "/api/v1/nodes"
    READY_NODES="$(printf '%s' "${_CURL_BODY}" | grep -oiE '"(ready|status)"[[:space:]]*:[[:space:]]*("?true"?|"Ready")' | wc -l | tr -d ' ')"
    [[ -z "${READY_NODES}" ]] && READY_NODES=0
    printf '%s' "${READY_NODES}"
}
is_multinode() {
    [[ -z "${NODE_COUNT}" ]] && detect_node_count >/dev/null
    [[ "${NODE_COUNT:-0}" -ge 2 ]]
}

# caps_json — cache GET /platform/capabilities into _CAPS_JSON, echo it.
caps_json() {
    if [[ -z "${_CAPS_JSON}" ]]; then
        curl_api GET "/api/v1/platform/capabilities"
        _CAPS_JSON="${_CURL_BODY}"
    fi
    printf '%s' "${_CAPS_JSON}"
}
# caps_bool DOTTED.KEY — python3-grade a nested boolean from capabilities.
#   e.g. caps_bool day2_ops.live_migration
caps_bool() {
    local key="$1" caps
    caps="$(caps_json)"
    command -v python3 >/dev/null 2>&1 || { return 2; }
    python3 - "${caps}" "${key}" <<'PY'
import sys, json
try:
    d = json.loads(sys.argv[1])
except Exception:
    sys.exit(2)
cur = d
for part in sys.argv[2].split('.'):
    if isinstance(cur, dict) and part in cur:
        cur = cur[part]
    else:
        sys.exit(1)
sys.exit(0 if cur is True else 1)
PY
}

# platform_version COMPONENT (kubevirt|cdi) — observedVersion from /platform/versions.
# The endpoint returns {"kubevirt":{"observedVersion":..},"cdi":{..}} (nested).
platform_version() {
    local comp="$1"
    curl_api GET "/api/v1/platform/versions"
    command -v python3 >/dev/null 2>&1 || { printf ''; return; }
    python3 - "${_CURL_BODY}" "${comp}" <<'PY'
import sys, json
try:
    d = json.loads(sys.argv[1])
    c = d.get(sys.argv[2], {}) or {}
    print(c.get("observedVersion") or c.get("operatorVersion") or "")
except Exception:
    print("")
PY
}

# ensure_ns — resolve TEST_NS to a namespace that exists so VM creates don't fail
# with "Namespace not found". Order: (1) already present → use it; (2) create the
# dedicated ns via kubectl if available (register cleanup); (3) fall back to
# 'default'. Safety does NOT depend on the namespace — every test resource is
# name-prefixed veyron-readiness-* and owned_by_test-gated regardless of ns.
# Sets TEST_NS (possibly to 'default') and TEST_NS_FELL_BACK=1 on fallback.
TEST_NS_FELL_BACK=0
ensure_ns() {
    curl_api GET "/api/v1/namespaces"
    if body_has "${_CURL_BODY}" "\"${TEST_NS}\""; then
        return 0
    fi
    if command -v kubectl >/dev/null 2>&1 && kubectl create namespace "${TEST_NS}" >/dev/null 2>&1; then
        register_cleanup "dispose_ns ${TEST_NS}"
        return 0
    fi
    TEST_NS_FELL_BACK=1
    TEST_NS="default"
    return 0
}

# ceph_health — echoes ok|present|absent. "present" means Ceph-backed storage is
# detected but this API doesn't expose cluster health, so the caller SKIPs rather
# than guessing degraded. The capabilities JSON carries StorageClass names like
# zyvor-rbd-prod / zyvor-cephfs-shared, so match ceph/cephfs/rook substrings.
ceph_health() {
    local caps
    caps="$(caps_json)"
    if printf '%s' "${caps}" | grep -qiE 'ceph|rook'; then
        if printf '%s' "${caps}" | grep -qiE 'HEALTH_OK|"ceph_healthy"[[:space:]]*:[[:space:]]*true'; then
            printf 'ok'
        else
            printf 'present'
        fi
    else
        printf 'absent'
    fi
}

# tls_issuer — X.509 issuer of the served cert (empty if openssl unavailable).
tls_issuer() {
    command -v openssl >/dev/null 2>&1 || { printf ''; return; }
    printf '' | openssl s_client -connect "${HOST}:${PORT}" -servername "${HOST}" 2>/dev/null \
        | openssl x509 -noout -issuer 2>/dev/null | sed 's/^issuer=//'
}
is_self_signed_dev_cert() {
    local iss
    iss="$(tls_issuer)"
    [[ -z "${iss}" ]] && return 1
    printf '%s' "${iss}" | grep -qiE 'veyron|self.?signed|localhost|kubevirt|default'
}

# ── Wait helpers ────────────────────────────────────────────────────────────
# wait_for TIMEOUT INTERVAL PRED_CMD... — poll until PRED returns 0 or timeout.
wait_for() {
    local timeout="$1" interval="$2"; shift 2
    local elapsed=0
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        if "$@"; then return 0; fi
        sleep "${interval}"
        elapsed=$((elapsed + interval))
    done
    return 1
}

_vm_status() {
    local ns="$1" name="$2"
    curl_api GET "/api/v1/vms/${ns}/${name}"
    if body_has "${_CURL_BODY}" '"status"[[:space:]]*:[[:space:]]*"Running"'; then echo Running
    elif body_has "${_CURL_BODY}" '"status"[[:space:]]*:[[:space:]]*"Stopped"'; then echo Stopped
    elif body_has "${_CURL_BODY}" '"status"[[:space:]]*:[[:space:]]*"Starting"'; then echo Starting
    else echo unknown; fi
}
wait_vm_status() {  # NS NAME WANT [TIMEOUT]
    local ns="$1" name="$2" want="$3" timeout="${4:-360}"
    local elapsed=0
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        [[ "$(_vm_status "${ns}" "${name}")" == "${want}" ]] && return 0
        sleep 5; elapsed=$((elapsed + 5))
    done
    return 1
}
wait_guest_connected() {  # NS NAME [TIMEOUT]
    local ns="$1" name="$2" timeout="${3:-360}" elapsed=0
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        curl_api GET "/api/v1/vms/${ns}/${name}/guest/status"
        if body_success "${_CURL_BODY}" && body_has "${_CURL_BODY}" '"connected"[[:space:]]*:[[:space:]]*true'; then
            return 0
        fi
        sleep 10; elapsed=$((elapsed + 10))
    done
    return 1
}
wait_snapshot_ready() {  # NS VM SNAP [TIMEOUT]
    local ns="$1" vm="$2" snap="$3" timeout="${4:-360}" elapsed=0
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        curl_api GET "/api/v1/snapshots/${ns}/${vm}"
        if body_has "${_CURL_BODY}" "${snap}" && body_has "${_CURL_BODY}" '"ready"[[:space:]]*:[[:space:]]*true'; then
            return 0
        fi
        sleep 10; elapsed=$((elapsed + 10))
    done
    return 1
}
wait_migration_complete() {  # ID [TIMEOUT]
    local id="$1" timeout="${2:-300}" elapsed=0
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        curl_api GET "/api/v1/migrations"
        if body_has "${_CURL_BODY}" "\"id\"[[:space:]]*:[[:space:]]*\"${id}\"" \
            && body_has "${_CURL_BODY}" '"status"[[:space:]]*:[[:space:]]*"Succeeded"'; then
            return 0
        fi
        if body_has "${_CURL_BODY}" '"status"[[:space:]]*:[[:space:]]*"Failed"'; then
            return 1
        fi
        sleep 10; elapsed=$((elapsed + 10))
    done
    return 1
}

# ── Safety framework ────────────────────────────────────────────────────────
# LIFO cleanup stack; a single trap tears everything down best-effort.
CLEANUP_STACK=()
register_cleanup() { CLEANUP_STACK+=("$1"); }
run_cleanup() {
    local i
    for (( i=${#CLEANUP_STACK[@]}-1; i>=0; i-- )); do
        eval "${CLEANUP_STACK[$i]}" >/dev/null 2>&1 || true
    done
    CLEANUP_STACK=()
}
# Callers that own the process lifecycle install the trap:
#   trap run_cleanup EXIT INT TERM

# owned_by_test NAME — assert a test-owned prefix before ANY destructive op.
# Guarantees a bug can never delete a customer VM/PVC/snapshot.
owned_by_test() {
    case "$1" in
        veyron-readiness-*|veyron-e2e-*) return 0 ;;
        *) printf '  %b‼ REFUSING to touch non-test resource: %s%b\n' "${_R}" "$1" "${_N}" >&2; return 1 ;;
    esac
}

# guard OPT_ENV LABEL — returns 0 only if the opt-in env is truthy.
# Callers SKIP with reason opt_in_required:<OPT_ENV> when this returns non-zero.
guard() {
    local val="${!1:-0}"
    case "${val}" in
        1|true|TRUE|yes|YES) return 0 ;;
        *) return 1 ;;
    esac
}

# ── Idempotent disposers (all owned_by_test-gated) ──────────────────────────
dispose_vm() {  # NS NAME
    local ns="$1" name="$2"
    owned_by_test "${name}" || return 1
    curl_api POST "/api/v1/vms/${ns}/${name}/stop" '{}' >/dev/null 2>&1 || true
    sleep 2
    curl_api DELETE "/api/v1/vms/${ns}/${name}" '' >/dev/null 2>&1 || true
}
dispose_snapshot() {  # NS SNAP
    local ns="$1" snap="$2"
    owned_by_test "${snap}" || return 1
    curl_api POST "/api/v1/snapshots/${ns}/${snap}/delete" '{}' >/dev/null 2>&1 || true
}
dispose_pvc() {  # NS PVC  (via storage orphan reclaim path is indirect; use k8s-free API)
    local ns="$1" pvc="$2"
    owned_by_test "${pvc}" || return 1
    curl_api DELETE "/api/v1/storage/pvcs/${ns}/${pvc}" '' >/dev/null 2>&1 || true
}
dispose_ns() {  # NAME  (only throwaway tenant namespaces)
    local ns="$1"
    case "${ns}" in
        veyron-readiness*|veyron-e2e*) : ;;
        *) printf '  %b‼ REFUSING to delete namespace %s%b\n' "${_R}" "${ns}" "${_N}" >&2; return 1 ;;
    esac
    curl_api DELETE "/api/v1/namespaces/${ns}" '' >/dev/null 2>&1 || true
}

# new_name PREFIX — deterministic test-owned name with a caller-supplied stamp.
# (No Date.now equivalent needed; callers pass a shared TS.)
new_name() { printf 'veyron-readiness-%s-%s' "$1" "${TS:-run}"; }
