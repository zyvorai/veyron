#!/usr/bin/env bash
# ============================================================================
# verify-veyron-remote.sh — Post-deploy API smoke test (Veyron on NodePort)
# ============================================================================
# Run from your laptop after deploy-all-remote.sh / deploy-remote.sh.
# Uses curl only (no jq). Exits 1 if any check fails.
#
# Usage:
#   ./scripts/verify-veyron-remote.sh <host> [https_node_port]
#   VEYRON_API_KEY='...' ./scripts/verify-veyron-remote.sh 192.0.2.1 30151
#
# Environment:
#   DEPLOY_HOST         Default host if arg omitted
#   VEYRON_NODE_PORT   Default port (default: 30151)
#   VEYRON_API_KEY     API key (default: saved ~/.config/veyron/hosts/<host>.env from deploy)
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-${VEYRON_NODE_PORT:-30151}}"
KEY="${VEYRON_API_KEY:-$(source "$(dirname "${BASH_SOURCE[0]}")/lib/api-key.sh" && veyron_require_api_key "${HOST}" 2>/dev/null)}"
if [[ -n "${HOST}" && -z "${KEY}" ]]; then
    echo "VEYRON_API_KEY is not set and no saved key for ${HOST} (deploy prints and saves it to ~/.config/veyron/hosts/)" >&2
    exit 1
fi

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
    local tmp
    tmp="$(mktemp)"
    if curl -skS --connect-timeout 15 --max-time 90 -o "${tmp}" "${url}" 2>/dev/null \
        && grep -q "${needle}" "${tmp}"; then
        echo "  ✔ ${name}"
    else
        echo "  ✗ ${name}"
        head -c 400 "${tmp}" 2>/dev/null | sed 's/^/    /' || true
        echo ""
        FAIL=$((FAIL + 1))
    fi
    rm -f "${tmp}"
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
echo "Veyron remote verify → ${BASE}"
echo ""

echo "  (unauthenticated)"
check_json_ok "GET /api/v1/health" "${BASE}/api/v1/health" no
check_http_200 "GET /console" "${BASE}/console" no
check_json_grep "GET /api/v1/auth/oidc/config" "${BASE}/api/v1/auth/oidc/config" '"enabled"'
oidc_token_code=$(curl -skS --connect-timeout 15 --max-time 45 -o /dev/null -w '%{http_code}' \
    -X POST -H 'Content-Type: application/json' -d '{}' "${BASE}/api/v1/auth/oidc/token" || echo "000")
if [[ "${oidc_token_code}" == "400" || "${oidc_token_code}" == "422" ]]; then
    echo "  ✔ POST /api/v1/auth/oidc/token (HTTP ${oidc_token_code} without valid body)"
else
    echo "  ✗ POST /api/v1/auth/oidc/token (HTTP ${oidc_token_code})"
    FAIL=$((FAIL + 1))
fi
check_json_grep "GET / (service info)" "${BASE}/" '"console"\s*:\s*"/console"'

echo "  (X-API-Key)"
check_json_ok "GET /api/v1/templates" "${BASE}/api/v1/templates" yes
check_json_items "GET /api/v1/crds/templates" "${BASE}/api/v1/crds/templates"
check_json_items "GET /api/v1/crds/profiles" "${BASE}/api/v1/crds/profiles"
check_json_items "GET /api/v1/crds/insights" "${BASE}/api/v1/crds/insights?namespace=all"
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
    "${BASE}/api/v1/gitops/status?namespace=all" '"veyron_context"'
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
check_json_ok "GET /api/v1/vms?ns=veyron-system" "${BASE}/api/v1/vms?namespace=veyron-system" yes
check_json_ok "GET /api/v1/vms?ns=all" "${BASE}/api/v1/vms?namespace=all" yes
# check_json_grep would hard-fail here whenever the cluster simply has zero
# VMs at the moment this runs (e.g. right after P7's E2E VM self-cleans) —
# there's no VM object for the "veyron_managed" field to appear on, which is
# not a defect. Only fail when the list is non-empty and the field is still
# missing.
vms_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -H "X-API-Key: ${KEY}" "${BASE}/api/v1/vms?namespace=all" || true)
if echo "${vms_body}" | grep -q '"veyron_managed"'; then
    echo "  ✔ GET /api/v1/vms (drift fields)"
elif echo "${vms_body}" | grep -qE '"data"\s*:\s*\[\]'; then
    echo "  ○ GET /api/v1/vms (drift fields) — skipped, no VMs currently exist"
else
    echo "  ✗ GET /api/v1/vms (drift fields)"
    echo "${vms_body}" | head -c 400 | sed 's/^/    /'
    echo ""
    FAIL=$((FAIL + 1))
fi
drift_code=$(curl -skS --connect-timeout 15 --max-time 45 -o /dev/null -w '%{http_code}' \
    -H "X-API-Key: ${KEY}" \
    "${BASE}/api/v1/vms/default/nonexistent-vm/drift" || echo "000")
if [[ "${drift_code}" == "404" || "${drift_code}" == "200" ]]; then
    echo "  ✔ GET /api/v1/vms/:ns/:name/drift (HTTP ${drift_code})"
else
    echo "  ✗ GET /api/v1/vms/:ns/:name/drift (HTTP ${drift_code})"
    FAIL=$((FAIL + 1))
fi

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
check_json_grep "GET /api/v1/soc/detections" \
    "${BASE}/api/v1/soc/detections?namespace=all" '"detections"'
check_json_grep "GET /api/v1/soc/events" \
    "${BASE}/api/v1/soc/events?limit=5&namespace=all" '"events"'
check_json_grep "GET /api/v1/soc/export/status" \
    "${BASE}/api/v1/soc/export/status" '"exporters"'
check_json_grep "GET /api/v1/soc/attack-surface" \
    "${BASE}/api/v1/soc/attack-surface?namespace=all" '"assets"'
check_json_grep "GET /api/v1/experience/home" \
    "${BASE}/api/v1/experience/home?namespace=all" '"health_score"'
translate_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -X POST -H "X-API-Key: ${KEY}" -H 'Content-Type: application/json' \
    -d '{"message":"PersistentVolumeClaim not bound","context":"vm_start"}' \
    "${BASE}/api/v1/experience/errors/translate" || true)
if echo "${translate_body}" | grep -q '"explanation"'; then
    echo "  ✔ POST /api/v1/experience/errors/translate"
else
    echo "  ✗ POST /api/v1/experience/errors/translate"
    echo "${translate_body}" | head -c 400 | sed 's/^/    /'
    echo ""
    FAIL=$((FAIL + 1))
fi
check_json_grep "GET /api/v1/experience/fleet/health" \
    "${BASE}/api/v1/experience/fleet/health?namespace=all" '"items"'
check_json_grep "GET /api/v1/experience/templates" \
    "${BASE}/api/v1/experience/templates" '"templates"'
check_json_grep "GET /api/v1/experience/session" \
    "${BASE}/api/v1/experience/session" '"role"'
check_json_grep "GET /api/v1/experience/locations" \
    "${BASE}/api/v1/experience/locations" '"locations"'
copilot_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -X POST -H "X-API-Key: ${KEY}" -H 'Content-Type: application/json' \
    -d '{"query":"show unhealthy vms"}' \
    "${BASE}/api/v1/experience/copilot/ask" || true)
if echo "${copilot_body}" | grep -q '"module"'; then
    echo "  ✔ POST /api/v1/experience/copilot/ask"
else
    echo "  ✗ POST /api/v1/experience/copilot/ask"
    echo "${copilot_body}" | head -c 400 | sed 's/^/    /'
    echo ""
    FAIL=$((FAIL + 1))
fi
check_json_grep "GET /api/v1/experience/copilot/backup" \
    "${BASE}/api/v1/experience/copilot/backup?namespace=all" '"module"'
check_json_grep "GET /api/v1/crds/blueprints" \
    "${BASE}/api/v1/crds/blueprints?namespace=all" '"items"'
check_json_grep "GET /api/v1/experience/copilot/cost" \
    "${BASE}/api/v1/experience/copilot/cost?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/performance" \
    "${BASE}/api/v1/experience/copilot/performance?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/gitops" \
    "${BASE}/api/v1/experience/copilot/gitops?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/forecast" \
    "${BASE}/api/v1/experience/copilot/forecast?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/integrations" \
    "${BASE}/api/v1/experience/copilot/integrations" '"module"'
check_json_grep "GET /api/v1/experience/copilot/scheduling" \
    "${BASE}/api/v1/experience/copilot/scheduling?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/compliance" \
    "${BASE}/api/v1/experience/copilot/compliance?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/observability" \
    "${BASE}/api/v1/experience/copilot/observability?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/cilium" \
    "${BASE}/api/v1/experience/copilot/cilium?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/nodes" \
    "${BASE}/api/v1/experience/copilot/nodes?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/drift" \
    "${BASE}/api/v1/experience/copilot/drift?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/alerts" \
    "${BASE}/api/v1/experience/copilot/alerts?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/slo" \
    "${BASE}/api/v1/experience/copilot/slo?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/migrations" \
    "${BASE}/api/v1/experience/copilot/migrations?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/quotas" \
    "${BASE}/api/v1/experience/copilot/quotas?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/catalog" \
    "${BASE}/api/v1/experience/copilot/catalog" '"module"'
check_json_grep "GET /api/v1/experience/copilot/velero-dr" \
    "${BASE}/api/v1/experience/copilot/velero-dr?namespace=all" '"module"'
check_json_grep "GET /api/v1/experience/copilot/status" \
    "${BASE}/api/v1/experience/copilot/status" '"mode"'
chat_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -H "X-API-Key: ${KEY}" -H "Content-Type: application/json" \
    -d '{"query":"help","messages":[]}' \
    "${BASE}/api/v1/experience/copilot/chat" || true)
if echo "${chat_body}" | grep -q '"copilot"'; then
    echo "  ✔ POST /api/v1/experience/copilot/chat"
else
    echo "  ✗ POST /api/v1/experience/copilot/chat"
    echo "${chat_body}" | head -c 400 | sed 's/^/    /'
    echo ""
    FAIL=$((FAIL + 1))
fi
check_json_grep "GET /api/v1/experience/copilot/guest/filesystem" \
    "${BASE}/api/v1/experience/copilot/guest/default/nonexistent-vm/filesystem" '"module"'
gitops_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -H "X-API-Key: ${KEY}" -H "Content-Type: application/json" \
    -d '{"yaml":"apiVersion: kubevirt.io/v1\nkind: VirtualMachine\n","app_name":"verify","namespace":"default"}' \
    "${BASE}/api/v1/experience/copilot/gitops/export" || true)
if echo "${gitops_body}" | grep -q '"files"'; then
    echo "  ✔ POST /api/v1/experience/copilot/gitops/export"
else
    echo "  ✗ POST /api/v1/experience/copilot/gitops/export"
    echo "${gitops_body}" | head -c 400 | sed 's/^/    /'
    echo ""
    FAIL=$((FAIL + 1))
fi
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
check_html_contains "GET /console (SPA root)" "${BASE}/console" 'id="root"'

echo "  (console SPA — Template Foundry, Cost Explorer, Network Intelligence)"
check_json_grep "GET /api/v1/experience/templates (store)" \
    "${BASE}/api/v1/experience/templates" '"templates"\s*:\s*\['
tpl_body=$(curl -skS --connect-timeout 15 --max-time 45 \
    -H "X-API-Key: ${KEY}" "${BASE}/api/v1/experience/templates" || true)
tpl_n=$(echo "${tpl_body}" | grep -o '"id"' | wc -l | tr -d ' ')
if [[ "${tpl_n}" -ge 25 ]]; then
    echo "  ✔ GET /api/v1/experience/templates (≥25 templates: ${tpl_n})"
else
    echo "  ✗ GET /api/v1/experience/templates (count ${tpl_n:-0}, expected ≥25)"
    FAIL=$((FAIL + 1))
fi
check_json_grep "GET /api/v1/costs (list)" \
    "${BASE}/api/v1/costs?namespace=all" '"costs"\s*:'
check_json_grep "GET /api/v1/costs/forecast" \
    "${BASE}/api/v1/costs/forecast?namespace=all" 'projected_monthly'
netra_status=$(curl -skS --connect-timeout 15 --max-time 45 \
    -H "X-API-Key: ${KEY}" "${BASE}/api/v1/netra/status" || true)
if echo "${netra_status}" | grep -q '"configured"\s*:\s*true'; then
    if echo "${netra_status}" | grep -q '"api_authorized"\s*:\s*true'; then
        echo "  ✔ GET /api/v1/netra/status"
        check_json_grep "GET /api/v1/netra/flows/summary" \
            "${BASE}/api/v1/netra/flows/summary?number=100" '"verdicts"\s*:'
        check_json_grep "GET /api/v1/netra/vms" \
            "${BASE}/api/v1/netra/vms" '"items"\s*:'
    else
        echo "  ✗ GET /api/v1/netra/status (found but not reachable or API key rejected)"
        echo "${netra_status}" | head -c 400 | sed 's/^/    /'
        echo ""
        FAIL=$((FAIL + 1))
    fi
else
    echo "  ○ GET /api/v1/netra/status (Netra not installed in netra-system and VEYRON_NETRA_URL unset)"
fi
check_json_grep "GET /api/v1/cilium/flows" \
    "${BASE}/api/v1/cilium/flows?namespace=all" 'flow_source'
# Console assets are content-hashed by the Vite build — extract the actual
# filenames from the served shell rather than hardcoding them.
console_html=$(curl -skS --connect-timeout 15 --max-time 45 "${BASE}/console" || true)
js_asset=$(echo "${console_html}" | grep -oE '/console/assets/[A-Za-z0-9_.-]+\.js' | head -1)
css_asset=$(echo "${console_html}" | grep -oE '/console/assets/[A-Za-z0-9_.-]+\.css' | head -1)
if [[ -n "${js_asset}" ]]; then
    check_http_200 "GET ${js_asset}" "${BASE}${js_asset}" no
else
    echo "  ✗ GET /console (no JS asset reference found in shell HTML)"
    FAIL=$((FAIL + 1))
fi
if [[ -n "${css_asset}" ]]; then
    check_http_200 "GET ${css_asset}" "${BASE}${css_asset}" no
else
    echo "  ✗ GET /console (no CSS asset reference found in shell HTML)"
    FAIL=$((FAIL + 1))
fi

echo ""
if [[ "${FAIL}" -eq 0 ]]; then
    echo "All checks passed."
    exit 0
fi
echo "${FAIL} check(s) failed — see Veyron pod logs on the cluster." >&2
exit 1
