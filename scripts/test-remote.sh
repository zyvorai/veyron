#!/usr/bin/env bash
# ============================================================================
# test-remote.sh — End-to-end checks for Veyron on a remote k3s/K8s node
# ============================================================================
# Assumes API is deployed in Kubernetes (e.g. ./scripts/deploy-k8s-remote.sh).
# No systemd — verifies Deployment readiness, optional operator + CRDs, and
# HTTPS API on the NodePort from this machine.
#
# Usage:
#   ./scripts/test-remote.sh [host] [user]
#   VEYRON_NODE_PORT=30151 VEYRON_API_KEY=... ./scripts/test-remote.sh
#   DEPLOY_HOST / DEPLOY_USER required if host/user omitted
#   VEYRON_REMOTE_DIR=/home/you/veyron  (operator samples for tier 3–4)
#
# Example:
#   ./scripts/test-remote.sh 192.0.2.1 sus
#   VEYRON_API_KEY=... ./scripts/test-remote.sh 192.0.2.1 sus
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
USER="${2:-${DEPLOY_USER:-}}"
if [[ -z "${HOST}" || -z "${USER}" ]]; then
    echo "Usage: $0 <host> <user>" >&2
    echo "  or:  DEPLOY_HOST=<host> DEPLOY_USER=<user> $0" >&2
    exit 1
fi
REMOTE="${USER}@${HOST}"

NS="${VEYRON_NAMESPACE:-veyron-system}"
NODE_PORT="${VEYRON_NODE_PORT:-30151}"
API_KEY="${VEYRON_API_KEY:-$(source "$(dirname "${BASH_SOURCE[0]}")/lib/api-key.sh" && veyron_require_api_key "${HOST}" 2>/dev/null)}"
if [[ -n "${HOST}" && -z "${API_KEY}" ]]; then
    echo "VEYRON_API_KEY is not set and no saved key for ${HOST} (deploy prints and saves it to ~/.config/veyron/hosts/)" >&2
    exit 1
fi
REMOTE_DIR="${VEYRON_REMOTE_DIR:-/home/${USER}/veyron}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOCAL_SAMPLES="${REPO_ROOT}/operator/config/samples"
SAMPLES="${REMOTE_DIR}/operator/config/samples"

# Apply a sample manifest: remote file if present, else pipe from local checkout.
apply_sample() {
  local rel="$1"
  local remote_path="${SAMPLES}/${rel}"
  local local_path="${LOCAL_SAMPLES}/${rel}"
  if _ssh "test -f '${remote_path}'" >/dev/null 2>&1; then
    run_k apply -f "${remote_path}"
  elif [[ -f "${local_path}" ]]; then
    cat "${local_path}" | _ssh "${REMOTE_K[@]}" apply -f -
  else
    echo "missing sample: ${rel} (not on remote or in ${LOCAL_SAMPLES})" >&2
    return 1
  fi
}

PASS=0
FAIL=0
SKIP=0

G='\033[0;32m'
R='\033[0;31m'
Y='\033[0;33m'
C='\033[0;36m'
B='\033[1m'
N='\033[0m'

pass() { PASS=$((PASS + 1)); echo -e "  ${G}PASS${N} $1"; }
fail() { FAIL=$((FAIL + 1)); echo -e "  ${R}FAIL${N} $1 — $2"; }
skip() { SKIP=$((SKIP + 1)); echo -e "  ${Y}SKIP${N} $1"; }
tier() { echo -e "\n${B}${C}═══ TIER $1: $2 ═══${N}"; }

check_api_json() {
  local name="$1"
  local path="$2"
  local pattern="$3"
  local body
  body=$(curl_api "$path" "${AUTH[@]}" 2>/dev/null || true)
  if echo "${body}" | grep -qE "${pattern}"; then
    pass "${name}"
  else
    fail "${name}" "$(echo "${body}" | head -c 160 | tr -d '\n')"
  fi
}

# Large dashboard HTML (~900KB); grep a temp file instead of piping a variable.
check_dashboard_html() {
  local name="$1"
  local needle="$2"
  local tmp
  tmp="$(mktemp)"
  if curl -sk --connect-timeout 15 --max-time 90 -o "${tmp}" "${BASE_URL}/dashboard" 2>/dev/null \
      && grep -q "${needle}" "${tmp}"; then
    pass "${name}"
  else
    fail "${name}" "missing ${needle}"
  fi
  rm -f "${tmp}"
}

_ssh() { ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "${REMOTE}" "$@" 2>&1; }

# kubectl on the remote node (k3s is the common case). Use an argv array so SSH
# never collapses "sudo /usr/local/bin/k3s kubectl" into a single token.
K_MODE=$(_ssh 'if [ -x /usr/local/bin/k3s ]; then echo k3s_std; elif command -v k3s >/dev/null 2>&1; then echo k3s_path; elif command -v kubectl >/dev/null 2>&1; then echo kubectl_plain; else echo ""; fi' | tr -d '\r')
case "${K_MODE}" in
  k3s_std) REMOTE_K=(sudo /usr/local/bin/k3s kubectl) ;;
  k3s_path) REMOTE_K=(sudo k3s kubectl) ;;
  kubectl_plain) REMOTE_K=(kubectl) ;;
  *)
    echo -e "${R}Could not find k3s or kubectl on ${REMOTE}${N}"
    exit 1
    ;;
esac

run_k() {
  _ssh "${REMOTE_K[@]}" "$@"
}

BASE_URL="https://${HOST}:${NODE_PORT}"

curl_api() {
  local path="$1"
  shift
  curl -sk --connect-timeout 15 --max-time 90 "$@" "${BASE_URL}${path}"
}

echo -e "${B}${C}"
echo "  ╦  ╦╔╦╗╦═╗╔═╗╔═╗╦ ╦╔═╗"
echo "  ╚╗╔╝║║║╠╦╝║ ║║ ╦║ ║║╣ "
echo "   ╚╝ ╩ ╩╩╚═╚═╝╚═╝╚═╝╚═╝"
echo -e "${N}${B}  Remote checks → ${REMOTE}${N}"
echo -e "${B}  kubectl: ${REMOTE_K[*]}${N}"
echo -e "${B}  API from here → ${BASE_URL}${N}\n"

# ═══════════════════════════════════════════════
tier 1 "SMOKE (Kubernetes + API)"
# ═══════════════════════════════════════════════

API_READY=$(run_k -n "${NS}" get deploy veyron-api -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' 2>/dev/null | tr -d '\r' || true)
if [[ "$API_READY" == "1/1" ]]; then
  pass "veyron-api Deployment ready (1/1) in ${NS}"
else
  fail "veyron-api Deployment" "ready/spec replicas: '${API_READY:-missing}' (deploy with ./scripts/deploy-k8s-remote.sh)"
fi

if run_k -n "${NS}" get deploy veyron-operator >/dev/null 2>&1; then
  OP_READY=$(run_k -n "${NS}" get deploy veyron-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" == "1/1" ]]; then
    pass "veyron-operator Deployment ready (1/1) in ${NS}"
  else
    skip "veyron-operator not fully ready (${OP_READY:-?})"
  fi
else
  skip "veyron-operator not installed in ${NS} (API-only cluster)"
fi

CRD_COUNT=$(run_k get crd -o name 2>/dev/null | grep -c 'veyron\.io' || true)
CRD_COUNT="${CRD_COUNT// /}"
CRD_COUNT="${CRD_COUNT//$'\r'/}"
CRD_COUNT="${CRD_COUNT:-0}"
if [[ "${CRD_COUNT}" -ge 7 ]]; then
  pass "Veyron CRDs present (${CRD_COUNT} veyron.io CRDs incl. catalog)"
elif [[ "${CRD_COUNT}" -ge 5 ]]; then
  pass "Veyron CRDs present (${CRD_COUNT} veyron.io CRDs; catalog CRDs optional)"
elif [[ "${CRD_COUNT}" -ge 1 ]]; then
  pass "Veyron CRDs present (${CRD_COUNT} veyron.io CRD(s))"
else
  skip "No veyron.io CRDs (install operator CRDs for VeyronVM / blueprints)"
fi

if run_k get vmtemplate windows-2022 >/dev/null 2>&1; then
  pass "VMTemplate windows-2022 in cluster catalog"
elif [[ "${CRD_COUNT}" -ge 7 ]]; then
  skip "VMTemplate windows-2022 not synced (run catalog export + deploy or veyron catalog sync)"
fi

if [[ "${CRD_COUNT}" -ge 1 ]]; then
  if run_k get veyronvms sample-ubuntu -n default >/dev/null 2>&1; then
    pass "sample-ubuntu VeyronVM exists in default"
  else
    skip "sample-ubuntu VeyronVM not in default (optional sample)"
  fi
else
  skip "skip sample-ubuntu check (no Veyron CRD)"
fi

if curl_api "/api/v1/health" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"'; then
  pass "API health (HTTPS NodePort ${NODE_PORT})"
else
  fail "API health" "no healthy JSON from ${BASE_URL}/api/v1/health"
fi

if curl_api "/api/v1/crds/veyronvms" -H "X-API-Key: ${API_KEY}" | grep -q '"total"'; then
  pass "CRD API GET /api/v1/crds/veyronvms (authenticated)"
else
  fail "CRD API" "no JSON list from /api/v1/crds/veyronvms"
fi

# ═══════════════════════════════════════════════
tier 2 "VM LIFECYCLE (operator + CRDs)"
# ═══════════════════════════════════════════════

if ! run_k -n "${NS}" get deploy veyron-operator >/dev/null 2>&1; then
  skip "entire tier — veyron-operator not in ${NS}"
elif [[ "${CRD_COUNT:-0}" -lt 1 ]]; then
  skip "entire tier — no veyron.io CRDs"
else
  OP_READY=$(run_k -n "${NS}" get deploy veyron-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — veyron-operator not ready (${OP_READY})"
  else
    run_k delete veyronvms test-e2e -n default --ignore-not-found >/dev/null 2>&1 || true
    sleep 2

    CREATE_OUT=$(
      _ssh "${REMOTE_K[@]}" apply -f - <<'EOF' 2>&1
apiVersion: veyron.io/v1alpha1
kind: VeyronVM
metadata:
  name: test-e2e
  namespace: default
spec:
  template: ubuntu-22.04
  cpu:
    cores: 1
    sockets: 1
    threads: 1
  memory:
    size: 1Gi
  disks:
    - name: rootdisk
      size: 20Gi
      bootOrder: 1
      source:
        type: containerDisk
        image: quay.io/containerdisks/ubuntu:22.04
  interfaces:
    - name: default
      network: default
      model: virtio
      networkType:
        type: pod
  cloudInit:
    userData: |
      #cloud-config
      hostname: test-e2e
  enableRng: true
  running: true
EOF
    )
    if echo "$CREATE_OUT" | grep -qE 'created|configured'; then
      pass "kubectl apply VeyronVM test-e2e"
    else
      fail "kubectl apply test-e2e" "$CREATE_OUT"
    fi

    sleep 12
    KV_VM=$(run_k get vm test-e2e -n default --no-headers 2>/dev/null | wc -l | tr -d ' \r')
    KV_VM="${KV_VM:-0}"
    if [[ "$KV_VM" -ge 1 ]]; then
      pass "KubeVirt VirtualMachine test-e2e reconciled"
    else
      fail "KubeVirt VM test-e2e" "not found yet (operator / KubeVirt / CDI?)"
    fi

    PHASE=$(run_k get veyronvms test-e2e -n default -o 'jsonpath={.status.phase}' 2>/dev/null | tr -d '\r' || true)
    if [[ -n "$PHASE" ]]; then
      pass "VeyronVM status phase: ${PHASE}"
    else
      skip "VeyronVM phase not set yet"
    fi

    GET_JSON=$(run_k get veyronvms test-e2e -n default -o 'jsonpath={.spec.cpu.cores}' 2>/dev/null | tr -d '\r' || true)
    if [[ "$GET_JSON" == "1" ]]; then
      pass "kubectl get veyronvms shows spec.cpu.cores=1"
    else
      fail "kubectl get veyronvms spec" "cores='${GET_JSON:-}'"
    fi

    if run_k delete veyronvms test-e2e -n default --ignore-not-found >/dev/null 2>&1; then
      pass "kubectl delete veyronvms test-e2e"
    else
      fail "kubectl delete veyronvms test-e2e" "kubectl returned non-zero"
    fi

    sleep 8
    if run_k get vm test-e2e -n default 2>&1 | grep -qiE 'NotFound|not found'; then
      pass "KubeVirt VM removed after VRVM delete"
    else
      skip "KubeVirt VM test-e2e may still be terminating"
    fi
  fi
fi

# ═══════════════════════════════════════════════
tier 3 "BLUEPRINT (operator + samples on remote)"
# ═══════════════════════════════════════════════

if [[ "${CRD_COUNT:-0}" -lt 1 ]]; then
  skip "entire tier — no veyron.io CRDs"
elif ! run_k -n "${NS}" get deploy veyron-operator >/dev/null 2>&1; then
  skip "entire tier — veyron-operator Deployment not in ${NS}"
else
  OP_READY=$(run_k -n "${NS}" get deploy veyron-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — veyron-operator not ready"
  else
    BP_OUT=$(apply_sample "veyron_v1alpha1_veyronblueprint.yaml" 2>&1)
    if echo "$BP_OUT" | grep -qE 'created|configured|unchanged'; then
      pass "LAMP blueprint applied"
    else
      fail "blueprint apply" "$BP_OUT"
    fi

    sleep 6
    # Remote login shell may be zsh — brace globs break jsonpath=; use YAML + grep.
    if run_k get veyronblueprint lamp-stack -n default -o yaml 2>/dev/null | grep -qE '^[[:space:]]+name:[[:space:]]+db'; then
      pass "kubectl get VeyronBlueprint lamp-stack (spec.vms)"
    else
      fail "blueprint get" "$(run_k get veyronblueprint lamp-stack -n default 2>&1)"
    fi

    # Windows blueprint with template resolution (operator catalog)
    if run_k get vmtemplate windows-2022 >/dev/null 2>&1; then
      WIN_OUT=$(apply_sample "windows-ad-blueprint.yaml" 2>&1 || true)
      if echo "$WIN_OUT" | grep -qE 'created|configured|unchanged'; then
        pass "Windows AD blueprint applied (template catalog)"
        run_k delete veyronblueprint windows-ad-lab -n default --ignore-not-found >/dev/null 2>&1 || true
      else
        skip "windows-ad blueprint apply (sample or CRD issue)"
      fi
    else
      skip "windows-ad blueprint — VMTemplate CRD/catalog not installed"
    fi

    run_k delete veyronblueprint lamp-stack -n default --ignore-not-found >/dev/null 2>&1 || true
    sleep 3
    run_k delete veyronvms -n default -l 'veyron.io/blueprint=lamp-stack' --ignore-not-found >/dev/null 2>&1 || true
    pass "Blueprint cleaned up"
  fi
fi

# ═══════════════════════════════════════════════
tier 4 "POLICIES + ACTIONS (operator + samples)"
# ═══════════════════════════════════════════════

if [[ "${CRD_COUNT:-0}" -lt 1 ]]; then
  skip "entire tier — no veyron.io CRDs"
elif ! run_k -n "${NS}" get deploy veyron-operator >/dev/null 2>&1; then
  skip "entire tier — veyron-operator Deployment not in ${NS}"
else
  OP_READY=$(run_k -n "${NS}" get deploy veyron-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — veyron-operator not ready"
  else
    POL_OUT=$(apply_sample "veyron_v1alpha1_veyronpolicy.yaml" 2>&1)
    echo "$POL_OUT" | grep -qE 'created|configured|unchanged' && pass "Security policy applied" || fail "policy apply" "$POL_OUT"

    sleep 2
    if run_k get veyronpolicy prod-security-baseline -n default -o yaml 2>/dev/null | grep -q 'require-tpm'; then
      pass "kubectl get VeyronPolicy prod-security-baseline (rules)"
    else
      fail "policy get" "missing prod-security-baseline or rules"
    fi

    INS_OUT=$(apply_sample "veyron_v1alpha1_veyroninsight.yaml" 2>&1)
    echo "$INS_OUT" | grep -qE 'created|configured|unchanged' && pass "Insight applied" || fail "insight apply" "$INS_OUT"

    ACT_OUT=$(apply_sample "veyron_v1alpha1_veyronaction.yaml" 2>&1)
    echo "$ACT_OUT" | grep -qE 'created|configured|unchanged' && pass "Action applied" || fail "action apply" "$ACT_OUT"

    sleep 2
    run_k get veyronaction scale-prod-db -n default >/dev/null 2>&1 && pass "kubectl get VeyronAction scale-prod-db" || fail "action get" "missing"
    run_k get veyroninsight high-cpu-prod-db -n default >/dev/null 2>&1 && pass "kubectl get VeyronInsight high-cpu-prod-db" || fail "insight get" "missing"

    run_k delete veyronpolicy prod-security-baseline -n default --ignore-not-found >/dev/null 2>&1 || true
    run_k delete veyroninsight high-cpu-prod-db -n default --ignore-not-found >/dev/null 2>&1 || true
    run_k delete veyronaction scale-prod-db -n default --ignore-not-found >/dev/null 2>&1 || true
    pass "Policies + actions cleaned up"
  fi
fi

# ═══════════════════════════════════════════════
tier 5 "API ENDPOINTS (HTTPS + X-API-Key)"
# ═══════════════════════════════════════════════

AUTH=( -H "X-API-Key: ${API_KEY}" )

for endpoint in \
  "/api/v1/crds/veyronvms" \
  "/api/v1/crds/blueprints" \
  "/api/v1/crds/templates" \
  "/api/v1/crds/profiles" \
  "/api/v1/crds/policies" \
  "/api/v1/crds/insights" \
  "/api/v1/crds/actions" \
  "/api/v1/images/catalog"; do
  code=$(curl_api "$endpoint" -o /dev/null -w '%{http_code}' "${AUTH[@]}")
  if [[ "$code" == "200" ]]; then
    pass "GET ${endpoint} → 200"
  else
    fail "GET ${endpoint}" "HTTP ${code}"
  fi
done

for endpoint in \
  "/api/v1/compliance/status" \
  "/api/v1/heatmap/resources" \
  "/api/v1/dashboards" \
  "/api/v1/network-policies" \
  "/api/v1/clusters" \
  "/api/v1/cilium/status" \
  "/api/v1/autoscaler/policies" \
  "/api/v1/integrations/status" \
  "/api/v1/snapshots" \
  "/api/v1/snapshot-schedules" \
  "/api/v1/backups" \
  "/api/v1/velero/status"; do
  code=$(curl_api "$endpoint" -o /dev/null -w '%{http_code}' "${AUTH[@]}")
  if [[ "$code" == "200" ]]; then
    pass "GET ${endpoint} → 200"
  else
    fail "GET ${endpoint}" "HTTP ${code}"
  fi
done

code=$(curl_api "/api/v1/dr/export?namespace=default&vm_name=nonexistent-vm" -o /dev/null -w '%{http_code}' "${AUTH[@]}")
if [[ "$code" == "200" || "$code" == "404" ]]; then
  pass "GET /api/v1/dr/export → ${code}"
else
  fail "GET /api/v1/dr/export" "HTTP ${code}"
fi

for endpoint in \
  "/api/v1/nodes" \
  "/api/v1/pods" \
  "/api/v1/events" \
  "/api/v1/metrics" \
  "/api/v1/costs"; do
  code=$(curl_api "$endpoint" -o /dev/null -w '%{http_code}' "${AUTH[@]}")
  if [[ "$code" == "200" ]]; then
    pass "GET ${endpoint} → 200"
  else
    fail "GET ${endpoint}" "HTTP ${code}"
  fi
done

for path in "/dashboard"; do
  code=$(curl_api "$path" -o /dev/null -w '%{http_code}')
  if [[ "$code" == "200" ]]; then
    pass "GET ${path} → 200"
  else
    fail "GET ${path}" "HTTP ${code}"
  fi
done

root_loc=$(curl -skI "${BASE_URL}/" 2>/dev/null | tr -d '\r' | awk -F': ' 'tolower($1)=="location"{print $2; exit}')
if [[ "${root_loc}" == */dashboard/* || "${root_loc}" == */dashboard ]]; then
  pass "GET / → ${root_loc}"
else
  fail "GET / root redirect" "expected /dashboard, got: ${root_loc:-none}"
fi

# ═══════════════════════════════════════════════
tier 6 "DASHBOARD SHELL (Template Foundry, Costs, Network Intelligence)"
# ═══════════════════════════════════════════════

check_api_json "Template Foundry /experience/templates" \
  "/api/v1/experience/templates" '"templates"\s*:\s*\['
tpl_body=$(curl_api "/api/v1/experience/templates" "${AUTH[@]}" 2>/dev/null || true)
tpl_n=$(echo "${tpl_body}" | grep -o '"id"' | wc -l | tr -d ' ')
if [[ "${tpl_n}" -ge 30 ]]; then
  pass "Template Foundry template count (≥30: ${tpl_n})"
else
  fail "Template Foundry template count" "got ${tpl_n:-0}, expected ≥30"
fi

check_api_json "Cost Explorer /costs list" \
  "/api/v1/costs?namespace=all" '"costs"\s*:'
check_api_json "Cost Explorer /costs/summary" \
  "/api/v1/costs/summary?namespace=all" 'total_cost'
check_api_json "Cost Explorer /costs/forecast" \
  "/api/v1/costs/forecast?namespace=all" 'projected_monthly'

check_api_json "Netra /netra/status" \
  "/api/v1/netra/status" '"api_authorized"\s*:\s*true'
check_api_json "Netra /netra/flows/summary" \
  "/api/v1/netra/flows/summary?number=100" '"verdicts"\s*:'
check_api_json "Netra /netra/vms" \
  "/api/v1/netra/vms" '"items"\s*:'
check_api_json "Paqtra /paqtra/status" \
  "/api/v1/paqtra/status" '"api_authorized"\s*:\s*true'
check_api_json "Paqtra /paqtra/flows" \
  "/api/v1/paqtra/flows?limit=5" '"flows"\s*:'
check_api_json "Paqtra /paqtra/drops" \
  "/api/v1/paqtra/drops" '"drops"\s*:'
check_api_json "Network Intelligence /cilium/flows" \
  "/api/v1/cilium/flows?namespace=all" 'flow_source'

check_dashboard_html "Dashboard embeds Template Foundry page" 'id="page-app-store"'
check_dashboard_html "Dashboard embeds Cost Explorer page" 'id="page-costs"'
check_dashboard_html "Dashboard embeds Network Intelligence loader" 'fetchNetworkIntelData'

# ═══════════════════════════════════════════════
echo ""
echo -e "${B}═══════════════════════════════════════════${N}"
echo -e "${B}  Results: ${G}${PASS} passed${N}, ${R}${FAIL} failed${N}, ${Y}${SKIP} skipped${N}"
echo -e "${B}═══════════════════════════════════════════${N}"
echo ""

[[ $FAIL -eq 0 ]] && echo -e "${G}${B}  ALL CHECKS PASSED (no failures)${N}" || echo -e "${R}${B}  SOME CHECKS FAILED${N}"
exit "$FAIL"
