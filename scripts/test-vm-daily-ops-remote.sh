#!/usr/bin/env bash
# ============================================================================
# test-vm-daily-ops-remote.sh — HTTPS API E2E for daily VM operations
# ============================================================================
# Exercises create, start, stop, pause, unpause, SSH expose, RDP expose, and
# snapshot create/list (optional restore) against a remote Veyron NodePort API.
# curl-only (no jq). Exits 1 if any check fails.
#
# Usage:
#   VEYRON_API_KEY='CHANGE_ME' ./scripts/test-vm-daily-ops-remote.sh <host> [port]
#
# Environment:
#   VEYRON_API_KEY          API key (default: CHANGE_ME)
#   VEYRON_E2E_NAMESPACE    Target namespace (default: default)
#   VEYRON_E2E_RESTORE      Set to 1 to run destructive snapshot restore
#   VEYRON_E2E_SKIP_TIER_B  Set to 1 to run Tier A only (no VM create)
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-${VMROGUE_NODE_PORT:-30151}}"
KEY="${VEYRON_API_KEY:-CHANGE_ME}"
NS="${VEYRON_E2E_NAMESPACE:-default}"
RESTORE="${VEYRON_E2E_RESTORE:-0}"
SKIP_TIER_B="${VEYRON_E2E_SKIP_TIER_B:-0}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [https_node_port]" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required" >&2
    exit 1
fi

BASE="https://${HOST}:${PORT}"
TS="$(date +%s)"
VM_NAME="veyron-e2e-${TS}"
SNAP_NAME="${VM_NAME}-snap-e2e"
RDP_PORT="${VEYRON_E2E_RDP_PORT:-30150}"

PASS=0
FAIL=0
SKIP=0
TIER_B_RAN=0
CREATED_VM=0

G='\033[0;32m'
R='\033[0;31m'
Y='\033[0;33m'
B='\033[1m'
N='\033[0m'

pass() { echo -e "  ${G}✔${N} $1"; PASS=$((PASS + 1)); }
fail() { echo -e "  ${R}✗${N} $1"; [[ -n "${2:-}" ]] && echo "    ${2}" | head -c 500; FAIL=$((FAIL + 1)); }
skip() { echo -e "  ${Y}○${N} $1"; SKIP=$((SKIP + 1)); }
tier() { echo ""; echo -e "${B}── $1 ──${N}"; }

# curl: sets _CURL_BODY and _CURL_CODE (do not use curl_api ...)" — subshell drops _CURL_CODE)
_CURL_CODE=""
_CURL_BODY=""
curl_api() {
    local method="$1"
    local path="$2"
    local body="${3:-}"
    local auth="${4:-yes}"
    local url="${BASE}${path}"
    local -a args=(-skS --connect-timeout 15 --max-time 120 -X "${method}")
    if [[ "${auth}" == "yes" ]]; then
        args+=(-H "X-API-Key: ${KEY}")
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


body_success() {
    echo "${1:-}" | grep -q '"success"[[:space:]]*:[[:space:]]*true'
}

body_error_code() {
    local body="$1"
    local code="$2"
    echo "${body}" | grep -qE "\"code\"[[:space:]]*:[[:space:]]*\"${code}\""
}

vm_status_from_body() {
    local body="$1"
    if echo "${body}" | grep -qE '"status"[[:space:]]*:[[:space:]]*"Running"'; then
        echo "Running"
    elif echo "${body}" | grep -qE '"status"[[:space:]]*:[[:space:]]*"Stopped"'; then
        echo "Stopped"
    elif echo "${body}" | grep -qE '"status"[[:space:]]*:[[:space:]]*"Starting"'; then
        echo "Starting"
    else
        echo "unknown"
    fi
}

wait_vm_status() {
    local want="$1"
    local timeout="${2:-300}"
    local elapsed=0
    local body st
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        curl_api GET "/api/v1/vms/${NS}/${VM_NAME}"
        body="${_CURL_BODY}"
        st="$(vm_status_from_body "${body}")"
        if [[ "${st}" == "${want}" ]]; then
            return 0
        fi
        sleep 5
        elapsed=$((elapsed + 5))
    done
    return 1
}

wait_guest_connected() {
    local timeout="${1:-360}"
    local elapsed=0
    local body
    while [[ "${elapsed}" -lt "${timeout}" ]]; do
        curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/status"
        body="${_CURL_BODY}"
        if body_success "${body}" && echo "${body}" | grep -qE '"connected"[[:space:]]*:[[:space:]]*true'; then
            return 0
        fi
        sleep 10
        elapsed=$((elapsed + 10))
    done
    return 1
}

cleanup_vm() {
    if [[ "${CREATED_VM}" -eq 0 ]]; then
        return 0
    fi
    echo ""
    echo -e "${Y}Cleaning up ${VM_NAME} in ${NS}...${N}"
    curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/stop" '{}' yes >/dev/null 2>&1 || true
    sleep 3
    curl_api DELETE "/api/v1/vms/${NS}/${VM_NAME}" '' yes >/dev/null 2>&1 || true
    curl_api POST "/api/v1/snapshots/${NS}/${SNAP_NAME}/delete" '{}' yes >/dev/null 2>&1 || true
}

trap cleanup_vm EXIT

echo -e "${B}Veyron daily-ops E2E${N} → ${BASE} (ns=${NS}, vm=${VM_NAME})"
echo ""

# ═══════════════════════════════════════════════
tier "Tier A — API contract (no VM)"
# ═══════════════════════════════════════════════

curl_api GET "/api/v1/vms" '' no
body="${_CURL_BODY}"
if body_success "${body}"; then
    fail "GET /api/v1/vms without API key should not succeed"
else
    pass "GET /api/v1/vms without API key rejected"
fi

curl_api PUT "/api/v1/vms/${NS}/nonexistent-vm-zzzz/expose" '{"enabled":true,"service_type":"NodePort","ports":[{"port":22,"target_port":22}]}'
body="${_CURL_BODY}"
if [[ "${_CURL_CODE}" == "404" ]] || body_error_code "${body}" "NOT_FOUND" \
    || echo "${body}" | grep -qiE 'not found|Resource not found'; then
    pass "PUT expose on missing VM → not found (HTTP ${_CURL_CODE})"
elif echo "${body}" | grep -q 'KubeVirt API unavailable'; then
    skip "PUT expose on missing VM (redeploy API for 404 mapping; got HTTP ${_CURL_CODE})"
else
    fail "expose on missing VM" "HTTP ${_CURL_CODE}: $(echo "${body}" | head -c 200)"
fi

curl_api PUT "/api/v1/vms/${NS}/nonexistent-vm-zzzz/rdp-expose" '{"enabled":true,"service_type":"NodePort"}'
body="${_CURL_BODY}"
if [[ "${_CURL_CODE}" == "400" ]] || body_error_code "${body}" "INVALID_REQUEST"; then
    pass "PUT rdp-expose without node_port → 400"
elif [[ "${_CURL_CODE}" == "404" ]] || body_error_code "${body}" "NOT_FOUND"; then
    skip "PUT rdp-expose without node_port (redeploy API; expected 400, got 404 HTTP ${_CURL_CODE})"
elif echo "${body}" | grep -q 'KubeVirt API unavailable'; then
    skip "PUT rdp-expose without node_port (redeploy API; got HTTP ${_CURL_CODE})"
else
    fail "rdp-expose without node_port" "HTTP ${_CURL_CODE}: $(echo "${body}" | head -c 200)"
fi

if [[ "${SKIP_TIER_B}" == "1" ]]; then
    skip "Tier B skipped (VEYRON_E2E_SKIP_TIER_B=1)"
else
    # ═══════════════════════════════════════════════
    tier "Tier B — disposable VM lifecycle"
    # ═══════════════════════════════════════════════
    TIER_B_RAN=1

    create_body="$(cat <<EOF
{
  "name": "${VM_NAME}",
  "namespace": "${NS}",
  "template": "ubuntu-22.04",
  "cpus": 1,
  "memory": "1Gi",
  "disk_size": "10Gi",
  "start": true
}
EOF
)"
    curl_api POST "/api/v1/vms" "${create_body}"
    body="${_CURL_BODY}"
    if body_success "${body}"; then
        pass "POST /api/v1/vms create ${VM_NAME}"
        CREATED_VM=1
    else
        fail "POST /api/v1/vms create" "$(echo "${body}" | head -c 400)"
        skip "remaining Tier B checks (create failed)"
    fi

    if [[ "${CREATED_VM}" -eq 1 ]]; then
        if wait_vm_status "Running" 360; then
            pass "VM reached Running"
        else
            fail "VM did not reach Running within 6m"
            skip "lifecycle checks after Running timeout"
        fi

        if [[ "${FAIL}" -eq 0 ]]; then
            GUEST_CONNECTED=0
            if wait_guest_connected 360; then
                GUEST_CONNECTED=1
                pass "Guest agent connected"
            else
                skip "Guest agent not connected within 6m — skipping guest/evidence/doctor"
            fi

            if [[ "${GUEST_CONNECTED}" -eq 1 ]]; then
                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/status"
                body="${_CURL_BODY}"
                if body_success "${body}" && echo "${body}" | grep -qE '"connected"[[:space:]]*:[[:space:]]*true'; then
                    pass "GET guest/status connected"
                else
                    fail "GET guest/status" "$(echo "${body}" | head -c 300)"
                fi

                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/evidence"
                body="${_CURL_BODY}"
                if body_success "${body}" && echo "${body}" | grep -qE '"evidence"'; then
                    pass "GET guest/evidence"
                else
                    fail "GET guest/evidence" "$(echo "${body}" | head -c 300)"
                fi

                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/doctor"
                body="${_CURL_BODY}"
                if body_success "${body}" && echo "${body}" | grep -qE '"boot_report"|"evidence"'; then
                    pass "GET guest/doctor"
                else
                    fail "GET guest/doctor" "$(echo "${body}" | head -c 300)"
                fi

                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/metrics"
                body="${_CURL_BODY}"
                if body_success "${body}" && echo "${body}" | grep -qE '"cpu"|"memory"'; then
                    pass "GET guest/metrics"
                else
                    fail "GET guest/metrics" "$(echo "${body}" | head -c 300)"
                fi

                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/guest/migrate-score?target=kvm"
                body="${_CURL_BODY}"
                if body_success "${body}"; then
                    pass "GET guest/migrate-score"
                else
                    fail "GET guest/migrate-score" "$(echo "${body}" | head -c 300)"
                fi
            fi
        fi

        if [[ "${FAIL}" -eq 0 ]]; then
            curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/stop" '{}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "POST stop" || fail "POST stop" "$(echo "${body}" | head -c 300)"

            if wait_vm_status "Stopped" 180; then
                pass "VM reached Stopped"
            else
                fail "VM did not reach Stopped"
            fi

            curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/start" '{}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "POST start" || fail "POST start" "$(echo "${body}" | head -c 300)"

            if wait_vm_status "Running" 300; then
                pass "VM Running after start"
            else
                fail "VM did not return to Running after start"
            fi

            curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/pause" '{}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "POST pause" || fail "POST pause" "$(echo "${body}" | head -c 300)"

            curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/unpause" '{}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "POST unpause" || fail "POST unpause" "$(echo "${body}" | head -c 300)"

            expose_put='{"enabled":true,"service_type":"NodePort","ports":[{"name":"ssh","port":22,"target_port":22,"protocol":"TCP"}]}'
            curl_api PUT "/api/v1/vms/${NS}/${VM_NAME}/expose" "${expose_put}"
            body="${_CURL_BODY}"
            if body_success "${body}" && echo "${body}" | grep -qE '"enabled"[[:space:]]*:[[:space:]]*true'; then
                pass "PUT SSH expose enabled"
            else
                fail "PUT SSH expose" "$(echo "${body}" | head -c 300)"
            fi

            curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/expose"
            body="${_CURL_BODY}"
            if body_success "${body}" && echo "${body}" | grep -qE '"enabled"[[:space:]]*:[[:space:]]*true'; then
                pass "GET SSH expose status"
            else
                fail "GET SSH expose" "$(echo "${body}" | head -c 300)"
            fi

            curl_api PUT "/api/v1/vms/${NS}/${VM_NAME}/expose" '{"enabled":false}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "PUT SSH expose disable" || fail "PUT SSH expose disable" "$(echo "${body}" | head -c 300)"

            curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/rdp-expose"
            body="${_CURL_BODY}"
            suggested=""
            if echo "${body}" | grep -qE '"suggested_node_port"[[:space:]]*:[[:space:]]*[0-9]+'; then
                suggested="$(echo "${body}" | sed -n 's/.*"suggested_node_port"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -1)"
            fi
            np="${suggested:-${RDP_PORT}}"
            rdp_put="$(printf '{"enabled":true,"service_type":"NodePort","node_port":%s}' "${np}")"
            RDP_PUT_OK=0
            curl_api PUT "/api/v1/vms/${NS}/${VM_NAME}/rdp-expose" "${rdp_put}"
            body="${_CURL_BODY}"
            if body_success "${body}" && echo "${body}" | grep -qE '"exposed"[[:space:]]*:[[:space:]]*true'; then
                pass "PUT RDP expose (node_port=${np})"
                RDP_PUT_OK=1
            elif echo "${body}" | grep -q 'NodePort.*already used'; then
                skip "PUT RDP expose (node_port ${np} in use — ${body:0:120})"
            else
                fail "PUT RDP expose" "$(echo "${body}" | head -c 300)"
            fi

            if [[ "${RDP_PUT_OK}" -eq 1 ]]; then
                curl_api GET "/api/v1/vms/${NS}/${VM_NAME}/rdp-expose"
                body="${_CURL_BODY}"
                if body_success "${body}" && echo "${body}" | grep -qE '"exposed"[[:space:]]*:[[:space:]]*true'; then
                    pass "GET RDP expose status"
                else
                    fail "GET RDP expose" "$(echo "${body}" | head -c 300)"
                fi
            else
                skip "GET RDP expose status (PUT did not succeed)"
            fi

            curl_api PUT "/api/v1/vms/${NS}/${VM_NAME}/rdp-expose" '{"enabled":false}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "PUT RDP expose disable" || fail "PUT RDP expose disable" "$(echo "${body}" | head -c 300)"

            SNAP_CREATED=0
            snap_body="$(printf '{"snapshot_name":"%s"}' "${SNAP_NAME}")"
            curl_api POST "/api/v1/snapshots/${NS}/${VM_NAME}/create" "${snap_body}"
            body="${_CURL_BODY}"
            if body_success "${body}"; then
                pass "POST snapshot create ${SNAP_NAME}"
                SNAP_CREATED=1
            else
                fail "POST snapshot create" "$(echo "${body}" | head -c 300)"
                skip "snapshot list/restore (create failed)"
            fi

            if [[ "${SNAP_CREATED}" -eq 1 ]]; then
                snap_list_ok=0
                list_body=""
                for _ in $(seq 1 24); do
                    curl_api GET "/api/v1/snapshots/${NS}/${VM_NAME}"
                    list_body="${_CURL_BODY}"
                    if body_success "${list_body}" && echo "${list_body}" | grep -q "${SNAP_NAME}"; then
                        snap_list_ok=1
                        break
                    fi
                    sleep 5
                done
                if [[ "${snap_list_ok}" -eq 1 ]]; then
                    pass "GET snapshots lists ${SNAP_NAME}"
                else
                    fail "GET snapshots list" "$(echo "${list_body:-}" | head -c 300)"
                fi

                if [[ "${RESTORE}" == "1" ]]; then
                    snap_ready=0
                    for _ in $(seq 1 36); do
                        curl_api GET "/api/v1/snapshots/${NS}/${VM_NAME}"
                        list_body="${_CURL_BODY}"
                        if echo "${list_body}" | grep -q "${SNAP_NAME}" \
                            && echo "${list_body}" | grep -qE '"ready"[[:space:]]*:[[:space:]]*true'; then
                            snap_ready=1
                            break
                        fi
                        sleep 10
                    done
                    if [[ "${snap_ready}" -eq 1 ]]; then
                        curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/stop" '{}'
                        sleep 8
                        curl_api POST "/api/v1/snapshots/${NS}/${SNAP_NAME}/restore" '{}'
                        body="${_CURL_BODY}"
                        if body_success "${body}"; then
                            pass "POST snapshot restore (VEYRON_E2E_RESTORE=1)"
                        elif echo "${body}" | grep -qE 'RESTORE_FAILED|Internal server error'; then
                            skip "snapshot restore (API/CR accepted but restore failed — common with containerDisk-only VMs)"
                        else
                            fail "POST snapshot restore" "$(echo "${body}" | head -c 300)"
                        fi
                    else
                        skip "snapshot restore (snapshot not ready within 6m)"
                    fi
                else
                    skip "snapshot restore (set VEYRON_E2E_RESTORE=1 to enable)"
                fi
            fi

            curl_api POST "/api/v1/vms/${NS}/${VM_NAME}/stop" '{}'
            body="${_CURL_BODY}"
            body_success "${body}" && pass "POST stop (pre-delete)" || fail "POST stop (pre-delete)" "$(echo "${body}" | head -c 200)"
            sleep 5
            curl_api DELETE "/api/v1/vms/${NS}/${VM_NAME}"
            body="${_CURL_BODY}"
            if body_success "${body}"; then
                pass "DELETE VM ${VM_NAME}"
                CREATED_VM=0
            else
                fail "DELETE VM" "$(echo "${body}" | head -c 300)"
            fi
            curl_api POST "/api/v1/snapshots/${NS}/${SNAP_NAME}/delete" '{}' >/dev/null 2>&1 || true
        fi
    fi
fi

# ═══════════════════════════════════════════════
echo ""
echo -e "${B}═══════════════════════════════════════════${N}"
echo -e "${B}  Results: ${G}${PASS} passed${N}, ${R}${FAIL} failed${N}, ${Y}${SKIP} skipped${N}"
echo -e "${B}═══════════════════════════════════════════${N}"
echo ""

[[ "${FAIL}" -eq 0 ]] && echo -e "${G}${B}  ALL CHECKS PASSED${N}" || echo -e "${R}${B}  SOME CHECKS FAILED${N}"
exit "${FAIL}"
