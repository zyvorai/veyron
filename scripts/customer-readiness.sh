#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# customer-readiness.sh — one-command go-live gate for a DEPLOYED Veyron cluster.
#
# Runs every readiness phase (P0 node preflight → P9 optional Windows golden
# image), auto-detects cluster shape (multi-node vs single, Velero/Prometheus/
# Ceph present), and emits a GO / NO-GO sign-off report. SKIP (feature N/A) and
# WARN (advisory) never cause a false NO-GO. Destructive checks are opt-in.
#
# Usage:
#   VEYRON_API_KEY=... ./scripts/customer-readiness.sh <host> [port] [options]
#
# Options:
#   --ssh-user U         enable P0 node preflight over SSH (else SKIP)
#   --report-dir DIR     where to write readiness-report.{json,md} (default: ./readiness-report)
#   --full               also run P9 Windows golden-image E2E (long)
#   --allow-destructive  flip ALL opt-in destructive flags on (maintenance window)
#
# Full RBAC coverage (recommended): provide three keyed roles so P3 can prove
# enforcement rather than SKIP it:
#   VEYRON_API_KEYS='ro:k1:readonly,wr:k2:write,admin:k3:admin' \
#   VEYRON_RO_KEY=k1 VEYRON_WRITE_KEY=k2 VEYRON_ADMIN_KEY=k3 \
#   VEYRON_API_KEY=k3 ./scripts/customer-readiness.sh <host>
#
# Opt-in destructive flags (else the relevant check SKIPs):
#   VEYRON_READY_ALLOW_DRAIN / _ALLOW_ORPHAN_DELETE / _ALLOW_HEAL /
#   _ALLOW_UPGRADE / _ALLOW_VELERO / _ALLOW_CLONE_RBAC
#
# Exit code mirrors the verdict: 0 = GO, 1 = NO-GO.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Arg parsing ─────────────────────────────────────────────────────────────
SSH_USER=""
REPORT_DIR="./readiness-report"
RUN_FULL=0
POSARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --ssh-user)        SSH_USER="${2:-}"; shift 2 ;;
        --report-dir)      REPORT_DIR="${2:-}"; shift 2 ;;
        --full)            RUN_FULL=1; shift ;;
        --allow-destructive)
            export VEYRON_READY_ALLOW_DRAIN=1 VEYRON_READY_ALLOW_ORPHAN_DELETE=1 \
                   VEYRON_READY_ALLOW_HEAL=1 VEYRON_READY_ALLOW_UPGRADE=1 \
                   VEYRON_READY_ALLOW_VELERO=1 VEYRON_READY_ALLOW_CLONE_RBAC=1
            shift ;;
        -h|--help)
            sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) POSARGS+=("$1"); shift ;;
    esac
done
set -- "${POSARGS[@]:-}"

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port] [--ssh-user U] [--report-dir DIR] [--full] [--allow-destructive]" >&2; exit 2; }

KEY="${VEYRON_API_KEY:-Admin@321}"
BASE="https://${HOST}:${PORT}"
export HOST PORT KEY BASE
export TEST_NS="${TEST_NS:-veyron-readiness}"
TS="$(date +%s)"; export TS

mkdir -p "${REPORT_DIR}"
REPORT_DIR="$(cd "${REPORT_DIR}" && pwd)"
export RESULTS_JSONL="${REPORT_DIR}/checks.jsonl"
: > "${RESULTS_JSONL}"
LOG_DIR="${REPORT_DIR}/logs"; mkdir -p "${LOG_DIR}"

# shellcheck source=lib/test-helpers.sh
source "${SCRIPT_DIR}/lib/test-helpers.sh"

# Load version baseline for the report header.
[[ -f "${SCRIPT_DIR}/cluster/versions.env" ]] && source "${SCRIPT_DIR}/cluster/versions.env"

printf '%b╔══════════════════════════════════════════════════════════╗%b\n' "${_B}" "${_N}"
printf '%b║  Veyron customer-readiness gate → %-24s ║%b\n' "${_B}" "${HOST}:${PORT}" "${_N}"
printf '%b╚══════════════════════════════════════════════════════════╝%b\n' "${_B}" "${_N}"
printf '  report: %s\n  test namespace: %s\n' "${REPORT_DIR}" "${TEST_NS}"

# ── Phase runners ───────────────────────────────────────────────────────────
declare -a PHASE_LABELS=() PHASE_STATUS=()
P1_PASSED=0

# Run a JSONL-emitting phase script (P3–P6). Records go straight to RESULTS_JSONL.
run_new_phase() {
    local label="$1" logfile="$2"; shift 2
    printf '\n%b━━━ %s ━━━%b\n' "${_C}" "${label}" "${_N}"
    local rc
    "$@" >"${logfile}" 2>&1; rc=$?
    cat "${logfile}"
    PHASE_LABELS+=("${label}"); PHASE_STATUS+=("$([[ ${rc} -eq 0 ]] && echo pass || echo fail)")
    return "${rc}"
}

# Run a legacy script (no JSONL) and emit ONE aggregate record from its exit code.
run_legacy_phase() {
    local label="$1" phase_tag="$2" logfile="$3"; shift 3
    printf '\n%b━━━ %s ━━━%b\n' "${_C}" "${label}" "${_N}"
    local rc
    "$@" >"${logfile}" 2>&1; rc=$?
    cat "${logfile}"
    CURRENT_PHASE="${phase_tag}"
    if [[ ${rc} -eq 0 ]]; then
        pass "${label} (exit 0)"
    else
        fail "${label} (exit ${rc})" "see ${logfile}"
    fi
    PHASE_LABELS+=("${label}"); PHASE_STATUS+=("$([[ ${rc} -eq 0 ]] && echo pass || echo fail)")
    return "${rc}"
}

skip_phase() {  # LABEL PHASE_TAG MSG REASON
    printf '\n%b━━━ %s (skipped) ━━━%b\n' "${_C}" "$1" "${_N}"
    CURRENT_PHASE="$2"; skip "$3" "$4"
    PHASE_LABELS+=("$1"); PHASE_STATUS+=("skip")
}

# ── P0 · Node preflight (SSH) ───────────────────────────────────────────────
if [[ -n "${SSH_USER}" && -x "${SCRIPT_DIR}/cluster/preflight-node.sh" ]]; then
    run_legacy_phase "P0 node preflight" "P0-node" "${LOG_DIR}/p0-node.log" \
        "${SCRIPT_DIR}/cluster/preflight-node.sh" "${HOST}" "${SSH_USER}" || true
else
    skip_phase "P0 node preflight" "P0-node" "node preflight over SSH" \
        "$([[ -z "${SSH_USER}" ]] && echo 'no_ssh_user:pass --ssh-user U' || echo 'preflight_node_missing')"
fi

# ── P1 · Acceptance gate (FAIL-FAST) ────────────────────────────────────────
if run_legacy_phase "P1 acceptance gate" "P1-gate" "${LOG_DIR}/p1-gate.log" \
    env VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/preflight-veyron-remote.sh" "${HOST}" "${PORT}"; then
    P1_PASSED=1
else
    P1_PASSED=0
    printf '\n%b‼ P1 acceptance gate FAILED — stopping before any mutation. Verdict: NO-GO.%b\n' "${_R}" "${_N}"
fi

# Everything past here mutates cluster state — only proceed if the gate passed.
if [[ "${P1_PASSED}" -eq 1 ]]; then
    detect_node_count >/dev/null
    printf '  detected %s node(s)\n' "${NODE_COUNT}"

    # Resolve the test namespace once (create dedicated ns if possible, else fall
    # back to default) and export it so every phase child uses the same one.
    CURRENT_PHASE="setup"
    ensure_ns
    export TEST_NS
    if [[ "${TEST_NS_FELL_BACK}" -eq 1 ]]; then
        warn "dedicated namespace unavailable — tests run in 'default' (resources still name-scoped veyron-readiness-* and cleaned up)"
    else
        pass "test namespace ready: ${TEST_NS}"
    fi

    # ── P2 · Read-only smoke ────────────────────────────────────────────────
    run_legacy_phase "P2 read-only smoke" "P2-smoke" "${LOG_DIR}/p2-smoke.log" \
        env VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/verify-veyron-remote.sh" "${HOST}" "${PORT}" || true

    # ── P3–P6 · New coverage (JSONL) ────────────────────────────────────────
    run_new_phase "P3 security & multi-tenancy" "${LOG_DIR}/p3-security.log" \
        "${SCRIPT_DIR}/test/security-rbac-remote.sh" "${HOST}" "${PORT}" "${SSH_USER}" || true
    run_new_phase "P4 data safety" "${LOG_DIR}/p4-data-safety.log" \
        "${SCRIPT_DIR}/test/data-safety-remote.sh" "${HOST}" "${PORT}" || true
    run_new_phase "P5 resilience & recovery" "${LOG_DIR}/p5-resilience.log" \
        "${SCRIPT_DIR}/test/resilience-remote.sh" "${HOST}" "${PORT}" || true
    run_new_phase "P6 upgrade & ops" "${LOG_DIR}/p6-upgrade-ops.log" \
        "${SCRIPT_DIR}/test/upgrade-ops-remote.sh" "${HOST}" "${PORT}" || true

    # ── P7 · Lifecycle E2E (into TEST_NS) ───────────────────────────────────
    run_legacy_phase "P7 lifecycle E2E" "P7-lifecycle" "${LOG_DIR}/p7-lifecycle.log" \
        env VEYRON_API_KEY="${KEY}" VEYRON_E2E_NAMESPACE="${TEST_NS}" \
        "${SCRIPT_DIR}/test-vm-daily-ops-remote.sh" "${HOST}" "${PORT}" || true

    # ── P8 · Dashboard console sweep ────────────────────────────────────────
    if [[ -x "${SCRIPT_DIR}/dashboard-console-check.sh" ]]; then
        run_legacy_phase "P8 dashboard console sweep" "P8-dashboard" "${LOG_DIR}/p8-dashboard.log" \
            env VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/dashboard-console-check.sh" --host "${HOST}" || true
    else
        skip_phase "P8 dashboard console sweep" "P8-dashboard" "dashboard console sweep" "console_check_missing"
    fi

    # ── P9 · Windows golden image (opt-in --full) ───────────────────────────
    # test-windows-golden-image-remote.sh needs SSH+kubectl access on the node
    # (builder-VM VNC keypress injection, sysprep polling) — same requirement
    # as P0, so it's gated on --ssh-user too, not just --full.
    if [[ "${RUN_FULL}" -eq 1 && -n "${SSH_USER}" && -x "${SCRIPT_DIR}/test-windows-golden-image-remote.sh" ]]; then
        run_legacy_phase "P9 Windows golden image" "P9-windows" "${LOG_DIR}/p9-windows.log" \
            env VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/test-windows-golden-image-remote.sh" "${HOST}" "${SSH_USER}" "${PORT}" || true
    else
        skip_phase "P9 Windows golden image" "P9-windows" "Windows golden-image E2E" \
            "$([[ "${RUN_FULL}" -eq 0 ]] && echo 'not_requested:pass --full' || { [[ -z "${SSH_USER}" ]] && echo 'no_ssh_user:pass --ssh-user U' || echo 'windows_test_missing'; })"
    fi
fi

# ── Report generation ───────────────────────────────────────────────────────
NODE_COUNT="${NODE_COUNT:-0}"
CEPH="$(ceph_health 2>/dev/null || echo unknown)"
TLS_ISS="$(tls_issuer 2>/dev/null || echo '')"
KV_VER="$(platform_version kubevirt 2>/dev/null || echo '')"
CDI_VER="$(platform_version cdi 2>/dev/null || echo '')"

REPORT_JSON="${REPORT_DIR}/readiness-report.json"
REPORT_MD="${REPORT_DIR}/readiness-report.md"

if command -v python3 >/dev/null 2>&1; then
    VERDICT="$(python3 "${SCRIPT_DIR}/lib/readiness-report.py" \
        --jsonl "${RESULTS_JSONL}" \
        --json-out "${REPORT_JSON}" \
        --md-out "${REPORT_MD}" \
        --host "${HOST}:${PORT}" \
        --nodes "${NODE_COUNT}" \
        --kubevirt "${KV_VER}" --kubevirt-baseline "${KUBEVIRT_VERSION:-}" \
        --cdi "${CDI_VER}" --cdi-baseline "${CDI_VERSION:-}" \
        --ceph "${CEPH}" --tls "${TLS_ISS}" \
        --p1-passed "${P1_PASSED}")"
    RC=$?
else
    # Minimal fallback: count statuses with grep.
    n_fail="$(grep -c '"status":"fail"' "${RESULTS_JSONL}" 2>/dev/null || echo 0)"
    if [[ "${P1_PASSED}" -eq 1 && "${n_fail}" -eq 0 ]]; then VERDICT="GO"; RC=0; else VERDICT="NO-GO"; RC=1; fi
    { echo "# Veyron readiness — ${VERDICT}"; echo "host: ${HOST}:${PORT}  nodes: ${NODE_COUNT}  fails: ${n_fail}"; } > "${REPORT_MD}"
fi

printf '\n%b════════════════════════════════════════════════════════════%b\n' "${_B}" "${_N}"
if [[ "${VERDICT}" == "GO" ]]; then
    printf '%b  VERDICT: GO%b — cluster is ready for customer deployment\n' "${_G}" "${_N}"
else
    printf '%b  VERDICT: NO-GO%b — resolve blocking failures before go-live\n' "${_R}" "${_N}"
fi
printf '%b════════════════════════════════════════════════════════════%b\n' "${_B}" "${_N}"
printf '  report: %s\n' "${REPORT_MD}"
exit "${RC:-1}"
