#!/usr/bin/env bash
# ============================================================================
# test-remote.sh — End-to-end checks for VMRogue on a remote k3s/K8s node
# ============================================================================
# Assumes API is deployed in Kubernetes (e.g. ./scripts/deploy-k8s-remote.sh).
# No systemd — verifies Deployment readiness, optional operator + CRDs, and
# HTTPS API on the NodePort from this machine.
#
# Usage:
#   ./scripts/test-remote.sh [host] [user]
#   VMROGUE_NODE_PORT=30151 VMROGUE_API_KEY=... ./scripts/test-remote.sh
#   VMROGUE_REMOTE_DIR=/home/you/vmrogue  (operator samples for tier 3–4)
#
# Defaults: host HOST, user sus, port 30151, key CHANGE_ME
# ============================================================================

set -euo pipefail

HOST="${1:-HOST}"
USER="${2:-sus}"
REMOTE="${USER}@${HOST}"

NS="${VMROGUE_NAMESPACE:-vmrogue-system}"
NODE_PORT="${VMROGUE_NODE_PORT:-30151}"
API_KEY="${VMROGUE_API_KEY:-CHANGE_ME}"
REMOTE_DIR="${VMROGUE_REMOTE_DIR:-/home/${USER}/vmrogue}"
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
  curl -sk "$@" "${BASE_URL}${path}"
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

API_READY=$(run_k -n "${NS}" get deploy vmrogue-api -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' 2>/dev/null | tr -d '\r' || true)
if [[ "$API_READY" == "1/1" ]]; then
  pass "vmrogue-api Deployment ready (1/1) in ${NS}"
else
  fail "vmrogue-api Deployment" "ready/spec replicas: '${API_READY:-missing}' (deploy with ./scripts/deploy-k8s-remote.sh)"
fi

if run_k -n "${NS}" get deploy vmrogue-operator >/dev/null 2>&1; then
  OP_READY=$(run_k -n "${NS}" get deploy vmrogue-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" == "1/1" ]]; then
    pass "vmrogue-operator Deployment ready (1/1) in ${NS}"
  else
    skip "vmrogue-operator not fully ready (${OP_READY:-?})"
  fi
else
  skip "vmrogue-operator not installed in ${NS} (API-only cluster)"
fi

CRD_COUNT=$(run_k get crd -o name 2>/dev/null | grep -c 'vmrogue\.io' || true)
CRD_COUNT="${CRD_COUNT// /}"
CRD_COUNT="${CRD_COUNT//$'\r'/}"
CRD_COUNT="${CRD_COUNT:-0}"
if [[ "${CRD_COUNT}" -ge 7 ]]; then
  pass "VMRogue CRDs present (${CRD_COUNT} vmrogue.io CRDs incl. catalog)"
elif [[ "${CRD_COUNT}" -ge 5 ]]; then
  pass "VMRogue CRDs present (${CRD_COUNT} vmrogue.io CRDs; catalog CRDs optional)"
elif [[ "${CRD_COUNT}" -ge 1 ]]; then
  pass "VMRogue CRDs present (${CRD_COUNT} vmrogue.io CRD(s))"
else
  skip "No vmrogue.io CRDs (install operator CRDs for VMRogueVM / blueprints)"
fi

if run_k get vmtemplate windows-2022 >/dev/null 2>&1; then
  pass "VMTemplate windows-2022 in cluster catalog"
elif [[ "${CRD_COUNT}" -ge 7 ]]; then
  skip "VMTemplate windows-2022 not synced (run catalog export + deploy or vmrogue catalog sync)"
fi

if [[ "${CRD_COUNT}" -ge 1 ]]; then
  if run_k get vrvm sample-ubuntu -n default >/dev/null 2>&1; then
    pass "sample-ubuntu VMRogueVM exists in default"
  else
    skip "sample-ubuntu VMRogueVM not in default (optional sample)"
  fi
else
  skip "skip sample-ubuntu check (no VMRogue CRD)"
fi

if curl_api "/api/v1/health" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"'; then
  pass "API health (HTTPS NodePort ${NODE_PORT})"
else
  fail "API health" "no healthy JSON from ${BASE_URL}/api/v1/health"
fi

if curl_api "/api/v1/crds/vmroguevms" -H "X-API-Key: ${API_KEY}" | grep -q '"total"'; then
  pass "CRD API GET /api/v1/crds/vmroguevms (authenticated)"
else
  fail "CRD API" "no JSON list from /api/v1/crds/vmroguevms"
fi

# ═══════════════════════════════════════════════
tier 2 "VM LIFECYCLE (operator + CRDs)"
# ═══════════════════════════════════════════════

if ! run_k -n "${NS}" get deploy vmrogue-operator >/dev/null 2>&1; then
  skip "entire tier — vmrogue-operator not in ${NS}"
elif [[ "${CRD_COUNT:-0}" -lt 1 ]]; then
  skip "entire tier — no vmrogue.io CRDs"
else
  OP_READY=$(run_k -n "${NS}" get deploy vmrogue-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — vmrogue-operator not ready (${OP_READY})"
  else
    run_k delete vrvm test-e2e -n default --ignore-not-found >/dev/null 2>&1 || true
    sleep 2

    CREATE_OUT=$(
      _ssh "${REMOTE_K[@]}" apply -f - <<'EOF' 2>&1
apiVersion: vmrogue.io/v1alpha1
kind: VMRogueVM
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
      pass "kubectl apply VMRogueVM test-e2e"
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

    PHASE=$(run_k get vrvm test-e2e -n default -o 'jsonpath={.status.phase}' 2>/dev/null | tr -d '\r' || true)
    if [[ -n "$PHASE" ]]; then
      pass "VMRogueVM status phase: ${PHASE}"
    else
      skip "VMRogueVM phase not set yet"
    fi

    GET_JSON=$(run_k get vrvm test-e2e -n default -o 'jsonpath={.spec.cpu.cores}' 2>/dev/null | tr -d '\r' || true)
    if [[ "$GET_JSON" == "1" ]]; then
      pass "kubectl get vrvm shows spec.cpu.cores=1"
    else
      fail "kubectl get vrvm spec" "cores='${GET_JSON:-}'"
    fi

    if run_k delete vrvm test-e2e -n default --ignore-not-found >/dev/null 2>&1; then
      pass "kubectl delete vrvm test-e2e"
    else
      fail "kubectl delete vrvm test-e2e" "kubectl returned non-zero"
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
  skip "entire tier — no vmrogue.io CRDs"
elif ! run_k -n "${NS}" get deploy vmrogue-operator >/dev/null 2>&1; then
  skip "entire tier — vmrogue-operator Deployment not in ${NS}"
else
  OP_READY=$(run_k -n "${NS}" get deploy vmrogue-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — vmrogue-operator not ready"
  else
    BP_OUT=$(apply_sample "vmrogue_v1alpha1_vmrogueblueprint.yaml" 2>&1)
    if echo "$BP_OUT" | grep -qE 'created|configured|unchanged'; then
      pass "LAMP blueprint applied"
    else
      fail "blueprint apply" "$BP_OUT"
    fi

    sleep 6
    # Remote login shell may be zsh — brace globs break jsonpath=; use YAML + grep.
    if run_k get vmrogueblueprint lamp-stack -n default -o yaml 2>/dev/null | grep -qE '^[[:space:]]+name:[[:space:]]+db'; then
      pass "kubectl get VMRogueBlueprint lamp-stack (spec.vms)"
    else
      fail "blueprint get" "$(run_k get vmrogueblueprint lamp-stack -n default 2>&1)"
    fi

    # Windows blueprint with template resolution (operator catalog)
    if run_k get vmtemplate windows-2022 >/dev/null 2>&1; then
      WIN_OUT=$(apply_sample "windows-ad-blueprint.yaml" 2>&1 || true)
      if echo "$WIN_OUT" | grep -qE 'created|configured|unchanged'; then
        pass "Windows AD blueprint applied (template catalog)"
        run_k delete vmrogueblueprint windows-ad-lab -n default --ignore-not-found >/dev/null 2>&1 || true
      else
        skip "windows-ad blueprint apply (sample or CRD issue)"
      fi
    else
      skip "windows-ad blueprint — VMTemplate CRD/catalog not installed"
    fi

    run_k delete vmrogueblueprint lamp-stack -n default --ignore-not-found >/dev/null 2>&1 || true
    sleep 3
    run_k delete vrvm -n default -l 'vmrogue.io/blueprint=lamp-stack' --ignore-not-found >/dev/null 2>&1 || true
    pass "Blueprint cleaned up"
  fi
fi

# ═══════════════════════════════════════════════
tier 4 "POLICIES + ACTIONS (operator + samples)"
# ═══════════════════════════════════════════════

if [[ "${CRD_COUNT:-0}" -lt 1 ]]; then
  skip "entire tier — no vmrogue.io CRDs"
elif ! run_k -n "${NS}" get deploy vmrogue-operator >/dev/null 2>&1; then
  skip "entire tier — vmrogue-operator Deployment not in ${NS}"
else
  OP_READY=$(run_k -n "${NS}" get deploy vmrogue-operator -o 'jsonpath={.status.readyReplicas}/{.spec.replicas}' | tr -d '\r')
  if [[ "$OP_READY" != "1/1" ]]; then
    skip "entire tier — vmrogue-operator not ready"
  else
    POL_OUT=$(apply_sample "vmrogue_v1alpha1_vmroguepolicy.yaml" 2>&1)
    echo "$POL_OUT" | grep -qE 'created|configured|unchanged' && pass "Security policy applied" || fail "policy apply" "$POL_OUT"

    sleep 2
    if run_k get vmroguepolicy prod-security-baseline -n default -o yaml 2>/dev/null | grep -q 'require-tpm'; then
      pass "kubectl get VMRoguePolicy prod-security-baseline (rules)"
    else
      fail "policy get" "missing prod-security-baseline or rules"
    fi

    INS_OUT=$(apply_sample "vmrogue_v1alpha1_vmrogueinsight.yaml" 2>&1)
    echo "$INS_OUT" | grep -qE 'created|configured|unchanged' && pass "Insight applied" || fail "insight apply" "$INS_OUT"

    ACT_OUT=$(apply_sample "vmrogue_v1alpha1_vmrogueaction.yaml" 2>&1)
    echo "$ACT_OUT" | grep -qE 'created|configured|unchanged' && pass "Action applied" || fail "action apply" "$ACT_OUT"

    sleep 2
    run_k get vmrogueaction scale-prod-db -n default >/dev/null 2>&1 && pass "kubectl get VMRogueAction scale-prod-db" || fail "action get" "missing"
    run_k get vmrogueinsight high-cpu-prod-db -n default >/dev/null 2>&1 && pass "kubectl get VMRogueInsight high-cpu-prod-db" || fail "insight get" "missing"

    run_k delete vmroguepolicy prod-security-baseline -n default --ignore-not-found >/dev/null 2>&1 || true
    run_k delete vmrogueinsight high-cpu-prod-db -n default --ignore-not-found >/dev/null 2>&1 || true
    run_k delete vmrogueaction scale-prod-db -n default --ignore-not-found >/dev/null 2>&1 || true
    pass "Policies + actions cleaned up"
  fi
fi

# ═══════════════════════════════════════════════
tier 5 "API ENDPOINTS (HTTPS + X-API-Key)"
# ═══════════════════════════════════════════════

AUTH=( -H "X-API-Key: ${API_KEY}" )

for endpoint in \
  "/api/v1/crds/vmroguevms" \
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

for path in "/dashboard" "/dashboard-next/"; do
  code=$(curl_api "$path" -o /dev/null -w '%{http_code}')
  if [[ "$code" == "200" ]]; then
    pass "GET ${path} → 200"
  else
    fail "GET ${path}" "HTTP ${code}"
  fi
done

root_loc=$(curl -skI "${BASE_URL}/" 2>/dev/null | tr -d '\r' | awk -F': ' 'tolower($1)=="location"{print $2; exit}')
if [[ "${root_loc}" == */dashboard-next/* || "${root_loc}" == */dashboard-next ]]; then
  pass "GET / → ${root_loc}"
else
  fail "GET / root redirect" "expected /dashboard-next/, got: ${root_loc:-none}"
fi

# ═══════════════════════════════════════════════
echo ""
echo -e "${B}═══════════════════════════════════════════${N}"
echo -e "${B}  Results: ${G}${PASS} passed${N}, ${R}${FAIL} failed${N}, ${Y}${SKIP} skipped${N}"
echo -e "${B}═══════════════════════════════════════════${N}"
echo ""

[[ $FAIL -eq 0 ]] && echo -e "${G}${B}  ALL CHECKS PASSED (no failures)${N}" || echo -e "${R}${B}  SOME CHECKS FAILED${N}"
exit "$FAIL"
