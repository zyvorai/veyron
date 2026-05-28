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
#   VMROGUE_API_KEY     API key (default: CHANGE_ME)
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-${VMROGUE_NODE_PORT:-30151}}"
KEY="${VMROGUE_API_KEY:-CHANGE_ME}"

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

# Endpoints that return CrdListResponse { items, total } without ApiResponse wrapper.
check_json_items() {
    local name="$1"
    local url="$2"
    local body
    body=$(curl -skS --connect-timeout 15 --max-time 45 \
        -H "X-API-Key: ${KEY}" "${url}" || true)
    if echo "${body}" | grep -qE '"items"\s*:\s*\['; then
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
    if curl -skS --connect-timeout 15 --max-time 45 "${url}" 2>/dev/null | grep -q "${needle}"; then
        echo "  ✔ ${name}"
    else
        echo "  ✗ ${name}"
        curl -skS --connect-timeout 15 --max-time 45 "${url}" 2>/dev/null | head -c 400 | sed 's/^/    /' || true
        echo ""
        FAIL=$((FAIL + 1))
    fi
}

check_http_200() {
    local name="$1"
    local url="$2"
    local use_auth="${3:-yes}"
    local code
    if [[ "${use_auth}" == "yes" ]]; then
        code=$(curl -skS --connect-timeout 15 --max-time 45 -o /dev/null -w '%{http_code}' \
            -H "X-API-Key: ${KEY}" "${url}" || echo "000")
    else
        code=$(curl -skS --connect-timeout 15 --max-time 45 -o /dev/null -w '%{http_code}' \
            "${url}" || echo "000")
    fi
    if [[ "${code}" == "200" ]]; then
        echo "  ✔ ${name}"
    else
        echo "  ✗ ${name} (HTTP ${code})"
        FAIL=$((FAIL + 1))
    fi
}

check_json_grep() {
    local name="$1"
    local url="$2"
    local pattern="$3"
    local body
    body=$(curl -skS --connect-timeout 15 --max-time 45 \
        -H "X-API-Key: ${KEY}" "${url}" || true)
    if echo "${body}" | grep -qE "${pattern}"; then
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
if [[ "${loc}" == */dashboard-next/* || "${loc}" == */dashboard-next ]]; then
    echo "  ✔ GET / → ${loc}"
elif [[ "${loc}" == */dashboard/* || "${loc}" == */dashboard ]]; then
    echo "  ✔ GET / → ${loc} (legacy)"
else
    echo "  ✗ GET / root redirect (got: ${loc:-none})"
    FAIL=$((FAIL + 1))
fi

echo "  (X-API-Key)"
check_json_ok "GET /api/v1/templates" "${BASE}/api/v1/templates" yes
check_json_items "GET /api/v1/crds/templates" "${BASE}/api/v1/crds/templates"
check_json_items "GET /api/v1/crds/profiles" "${BASE}/api/v1/crds/profiles"
check_json_grep "GET /api/v1/catalog/status" \
    "${BASE}/api/v1/catalog/status" 'embedded_templates'
check_json_grep "GET /api/v1/images/catalog" \
    "${BASE}/api/v1/images/catalog?namespace=all" '"images"\s*:\s*\['
check_json_ok "GET /api/v1/namespaces" "${BASE}/api/v1/namespaces" yes
check_json_ok "GET /api/v1/nodes" "${BASE}/api/v1/nodes" yes
check_json_ok "GET /api/v1/storage/classes" "${BASE}/api/v1/storage/classes" yes
check_json_ok "GET /api/v1/storage/pvcs?ns=all" "${BASE}/api/v1/storage/pvcs?namespace=all" yes
check_json_grep "GET /api/v1/custom-resources" \
    "${BASE}/api/v1/custom-resources" '"kind"\s*:'
check_json_grep "GET /api/v1/gitops/status" \
    "${BASE}/api/v1/gitops/status?namespace=all" '"vmrogue_context"'
check_json_ok "GET /api/v1/pods?ns=all" "${BASE}/api/v1/pods?namespace=all" yes
check_json_ok "GET /api/v1/events/recent" "${BASE}/api/v1/events/recent?namespace=all" yes
check_json_grep "GET /api/v1/monitoring/status" \
    "${BASE}/api/v1/monitoring/status?namespace=all" 'prometheus_available'
check_json_grep "GET /api/v1/security/posture" \
    "${BASE}/api/v1/security/posture?namespace=all" 'overall_score'
check_json_grep "GET /api/v1/security/findings" \
    "${BASE}/api/v1/security/findings?namespace=all" '"findings"\s*:\s*\['
check_json_grep "GET /api/v1/costs/summary" \
    "${BASE}/api/v1/costs/summary?namespace=all" 'total_cost'
check_json_ok "GET /api/v1/vms?ns=vmrogue-system" "${BASE}/api/v1/vms?namespace=vmrogue-system" yes
check_json_ok "GET /api/v1/vms?ns=all" "${BASE}/api/v1/vms?namespace=all" yes

echo "  (feature-gap routes)"
check_json_grep "GET /api/v1/compliance/status" \
    "${BASE}/api/v1/compliance/status?namespace=all" '"framework"'
check_json_grep "GET /api/v1/heatmap/resources" \
    "${BASE}/api/v1/heatmap/resources" '"nodes"'
check_http_200 "GET /api/v1/dashboards" \
    "${BASE}/api/v1/dashboards?namespace=all" yes
check_http_200 "GET /api/v1/network-policies" \
    "${BASE}/api/v1/network-policies?namespace=all" yes
check_json_grep "GET /api/v1/integrations/status" \
    "${BASE}/api/v1/integrations/status" '"integrations"'
check_json_grep "GET /api/v1/gitops/status" \
    "${BASE}/api/v1/gitops/status?namespace=all" '"sync_status"'
check_json_ok "GET /api/v1/snapshots?ns=all" \
    "${BASE}/api/v1/snapshots?namespace=all" yes
check_json_ok "GET /api/v1/snapshot-schedules" \
    "${BASE}/api/v1/snapshot-schedules?namespace=all" yes
check_json_grep "GET /api/v1/backups" \
    "${BASE}/api/v1/backups?namespace=all" '^\['
check_json_grep "GET /api/v1/velero/status" \
    "${BASE}/api/v1/velero/status" 'velero_available'
dr_code=$(curl -skS --connect-timeout 15 --max-time 45 -o /dev/null -w '%{http_code}' \
    -H "X-API-Key: ${KEY}" \
    "${BASE}/api/v1/dr/export?namespace=default&vm_name=nonexistent-vm" || echo "000")
if [[ "${dr_code}" == "404" || "${dr_code}" == "200" ]]; then
    echo "  ✔ GET /api/v1/dr/export (HTTP ${dr_code})"
else
    echo "  ✗ GET /api/v1/dr/export (HTTP ${dr_code})"
    FAIL=$((FAIL + 1))
fi
check_http_200 "GET /dashboard (classic)" "${BASE}/dashboard" no

echo ""
if [[ "${FAIL}" -eq 0 ]]; then
    echo "All checks passed."
    exit 0
fi
echo "${FAIL} check(s) failed — see VMRogue pod logs on the cluster." >&2
exit 1
