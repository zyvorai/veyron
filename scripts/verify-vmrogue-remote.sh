#!/usr/bin/env bash
# ============================================================================
# verify-vmrogue-remote.sh — Post-deploy API smoke test (VMRogue on NodePort)
# ============================================================================
# Run from your laptop after deploy-all-remote.sh / deploy-remote.sh.
# Uses curl only (no jq). Exits 1 if any check fails.
#
# Usage:
#   ./scripts/verify-vmrogue-remote.sh <host> [https_node_port]
#   VMROGUE_API_KEY='...' ./scripts/verify-vmrogue-remote.sh 192.0.2.1 30151
#
# Environment:
#   DEPLOY_HOST         Default host if arg omitted
#   VMROGUE_NODE_PORT   Default port (default: 30151)
#   VMROGUE_API_KEY     API key (default: Admin@321)
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-${VMROGUE_NODE_PORT:-30151}}"
KEY="${VMROGUE_API_KEY:-Admin@321}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [https_node_port]" >&2
    echo "  or:  DEPLOY_HOST=<host> $0" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required" >&2
    exit 1
fi

BASE="https://${HOST}:${PORT}"
FAIL=0

check_json_ok() {
    local name="$1"
    local url="$2"
    local use_auth="${3:-no}"
    local body
    if [[ "${use_auth}" == "yes" ]]; then
        body=$(curl -skS --connect-timeout 15 --max-time 45 \
            -H "X-API-Key: ${KEY}" "${url}" || true)
    else
        body=$(curl -skS --connect-timeout 15 --max-time 45 "${url}" || true)
    fi
    if echo "${body}" | grep -q '"success":true'; then
        echo "  ✔ ${name}"
    else
        echo "  ✗ ${name}"
        echo "${body}" | head -c 400 | sed 's/^/    /'
        echo ""
        FAIL=$((FAIL + 1))
    fi
}

check_html_contains() {
    local name="$1"
    local url="$2"
    local needle="$3"
    local body
    body=$(curl -skS --connect-timeout 15 --max-time 45 "${url}" || true)
    if echo "${body}" | grep -q "${needle}"; then
        echo "  ✔ ${name}"
    else
        echo "  ✗ ${name}"
        echo "${body}" | head -c 400 | sed 's/^/    /'
        echo ""
        FAIL=$((FAIL + 1))
    fi
}

echo ""
echo "VMRogue remote verify → ${BASE}"
echo ""

echo "  (unauthenticated)"
check_json_ok "GET /api/v1/health" "${BASE}/api/v1/health" no
check_html_contains "GET /dashboard-next/" "${BASE}/dashboard-next/" 'id="root"'
loc=$(curl -skI --connect-timeout 15 --max-time 45 "${BASE}/" | tr -d '\r' | awk -F': ' 'tolower($1)=="location"{print $2; exit}')
if [[ "${loc}" == */dashboard ]]; then
    echo "  ✔ GET / → /dashboard"
else
    echo "  ✗ GET / → /dashboard (got: ${loc:-none})"
    FAIL=$((FAIL + 1))
fi

echo "  (X-API-Key)"
check_json_ok "GET /api/v1/templates" "${BASE}/api/v1/templates" yes
check_json_ok "GET /api/v1/namespaces" "${BASE}/api/v1/namespaces" yes
check_json_ok "GET /api/v1/nodes" "${BASE}/api/v1/nodes" yes
check_json_ok "GET /api/v1/storage/classes" "${BASE}/api/v1/storage/classes" yes
check_json_ok "GET /api/v1/storage/pvcs?ns=all" "${BASE}/api/v1/storage/pvcs?namespace=all" yes
check_json_ok "GET /api/v1/custom-resources" "${BASE}/api/v1/custom-resources" yes
check_json_ok "GET /api/v1/gitops/status" "${BASE}/api/v1/gitops/status?namespace=all" yes
check_json_ok "GET /api/v1/pods?ns=all" "${BASE}/api/v1/pods?namespace=all" yes
check_json_ok "GET /api/v1/events/recent" "${BASE}/api/v1/events/recent?namespace=all" yes
check_json_ok "GET /api/v1/monitoring/status" "${BASE}/api/v1/monitoring/status?namespace=all" yes
check_json_ok "GET /api/v1/security/posture" "${BASE}/api/v1/security/posture?namespace=all" yes
check_json_ok "GET /api/v1/security/findings" "${BASE}/api/v1/security/findings?namespace=all" yes
check_json_ok "GET /api/v1/costs/summary" "${BASE}/api/v1/costs/summary?namespace=all" yes
check_json_ok "GET /api/v1/vms?ns=vmrogue-system" "${BASE}/api/v1/vms?namespace=vmrogue-system" yes
check_json_ok "GET /api/v1/vms?ns=all" "${BASE}/api/v1/vms?namespace=all" yes

echo ""
if [[ "${FAIL}" -eq 0 ]]; then
    echo "All checks passed."
    exit 0
fi
echo "${FAIL} check(s) failed — see VMRogue pod logs on the cluster." >&2
exit 1
