#!/usr/bin/env bash
# Smoke console-wired VM day-2 + product APIs against a live lab.
# Usage: VEYRON_API_KEY=Admin@321 ./scripts/console-actions-smoke.sh HOST [PORT] [NS] [VM]
set -euo pipefail
HOST="${1:?host}"
PORT="${2:-30151}"
NS="${3:-default}"
VM="${4:-}"
KEY="${VEYRON_API_KEY:-Admin@321}"
BASE="https://${HOST}:${PORT}"
PASS=0
FAIL=0
SKIP=0

curl_api() {
  local method="$1" path="$2" body="${3:-}"
  local args=(-sk -o /tmp/veyron-smoke.json -w "%{http_code}" -X "$method"
    -H "X-API-Key: ${KEY}" -H "Content-Type: application/json" "${BASE}${path}")
  if [[ -n "$body" ]]; then args+=(-d "$body"); fi
  _CURL_CODE=$(curl "${args[@]}" || echo 000)
  _CURL_BODY=$(cat /tmp/veyron-smoke.json 2>/dev/null || true)
}

pass() { echo "  [✓] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [✗] $1 (HTTP ${_CURL_CODE}) ${_CURL_BODY:0:160}"; FAIL=$((FAIL + 1)); }
skip() { echo "  [–] $1"; SKIP=$((SKIP + 1)); }

# 200 OK, or soft miss for optional integrations
soft() {
  local label="$1"
  if [[ "${_CURL_CODE}" == "200" ]]; then pass "$label"
  elif [[ "${_CURL_CODE}" =~ ^(404|501|502|503)$ ]]; then skip "$label (${_CURL_CODE})"
  elif [[ "${_CURL_CODE}" == "200" ]]; then pass "$label"
  else
    # configured:false still often returns 200
    if echo "${_CURL_BODY}" | grep -qi 'configured.:false\|not configured\|unreachable'; then
      skip "$label (unconfigured)"
    else
      fail "$label"
    fi
  fi
}

echo "Console actions smoke → ${BASE}"
curl_api GET /api/v1/health
[[ "${_CURL_CODE}" == "200" ]] && pass "health" || fail "health"

curl_api GET "/api/v1/vms?namespace=all"
[[ "${_CURL_CODE}" == "200" ]] && pass "list vms" || fail "list vms"
if [[ -z "$VM" ]]; then
  VM=$(python3 -c "import json
d=json.load(open('/tmp/veyron-smoke.json'))
items=d.get('data') if isinstance(d,dict) else d
if not isinstance(items,list): items=[]
print(next((x.get('name') for x in items if isinstance(x,dict) and x.get('name')),''))" 2>/dev/null || true)
fi
if [[ -z "$VM" ]]; then
  skip "no VM found — skipping lifecycle"
else
  echo "  Using VM ${NS}/${VM}"
  curl_api POST "/api/v1/vms/${NS}/${VM}/pause" '{}'
  [[ "${_CURL_CODE}" =~ ^(200|202|409)$ ]] && pass "pause (${_CURL_CODE})" || fail "pause"
  curl_api POST "/api/v1/vms/${NS}/${VM}/unpause" '{}'
  [[ "${_CURL_CODE}" =~ ^(200|202|409)$ ]] && pass "unpause (${_CURL_CODE})" || fail "unpause"

  curl_api PUT "/api/v1/vms/${NS}/${VM}/expose" '{"enabled":true,"service_type":"ClusterIP","ports":[{"port":22,"target_port":22}]}'
  [[ "${_CURL_CODE}" =~ ^(200|201)$ ]] && pass "expose put" || fail "expose put"
  curl_api GET "/api/v1/vms/${NS}/${VM}/expose"
  [[ "${_CURL_CODE}" == "200" ]] && pass "expose get" || fail "expose get"
  curl_api DELETE "/api/v1/vms/${NS}/${VM}/expose"
  [[ "${_CURL_CODE}" =~ ^(200|204)$ ]] && pass "expose delete" || fail "expose delete"

  curl_api GET "/api/v1/vms/${NS}/${VM}/network/internet"
  soft "internet get"
  curl_api GET "/api/v1/vms/${NS}/${VM}/drift"
  soft "drift get"
  curl_api POST "/api/v1/vms/${NS}/${VM}/hotplug" '{}'
  [[ "${_CURL_CODE}" == "400" ]] && pass "hotplug empty → 400" || fail "hotplug validation"
  curl_api PUT "/api/v1/vms/${NS}/${VM}/run-strategy" '{"strategy":"Always"}'
  [[ "${_CURL_CODE}" =~ ^(200|202)$ ]] && pass "run-strategy" || fail "run-strategy"
  curl_api POST "/api/v1/vms/bulk" "{\"action\":\"start\",\"namespace\":\"${NS}\",\"names\":[\"${VM}\"]}"
  [[ "${_CURL_CODE}" =~ ^(200|202)$ ]] && pass "bulk start" || fail "bulk start"
  curl_api GET "/api/v1/vms/${NS}/${VM}/guest/status"
  soft "guest status"
fi

# Product surfaces
curl_api GET "/api/v1/gpus"; soft "gpus"
curl_api GET "/api/v1/alerts?namespace=all"; soft "alerts"
curl_api GET "/api/v1/soc/detections"; soft "soc detections"
curl_api GET "/api/v1/atlas/status"; soft "atlas status"
curl_api GET "/api/v1/migrations"; soft "migrations"
curl_api GET "/api/v1/capacity/headroom"; soft "capacity"
curl_api GET "/api/v1/topology/map"; soft "topology"
curl_api GET "/api/v1/velero/status"; soft "velero"
curl_api GET "/api/v1/packetwolf/status"; soft "packetwolf"
curl_api GET "/api/v1/crds/templates"; soft "crd templates"
curl_api GET "/api/v1/images/datasources"; soft "datasources"
curl_api GET "/api/v1/snapshots?namespace=all"; soft "snapshots"
curl_api GET "/api/v1/backups?namespace=all"; soft "backups"
curl_api POST "/api/v1/self-healing/run?heal=false" '{}'; soft "self-healing dry-run"

echo
echo "PASS=${PASS} FAIL=${FAIL} SKIP=${SKIP}"
[[ "$FAIL" -eq 0 ]]
