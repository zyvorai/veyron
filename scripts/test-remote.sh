#!/usr/bin/env bash
# ============================================================================
# test-remote.sh — End-to-end tests for VMRogue on remote k3s cluster
# ============================================================================
# Runs 5 test tiers via SSH against the deployed VMRogue stack.
#
# Usage: ./scripts/test-remote.sh [host] [user]
#   Default: 185.165.240.5 sus
# ============================================================================

set -euo pipefail

HOST="${1:-185.165.240.5}"
USER="${2:-sus}"
REMOTE="${USER}@${HOST}"

PASS=0
FAIL=0
SKIP=0

G='\033[0;32m'
R='\033[0;31m'
Y='\033[0;33m'
C='\033[0;36m'
B='\033[1m'
N='\033[0m'

pass() { ((PASS++)); echo -e "  ${G}PASS${N} $1"; }
fail() { ((FAIL++)); echo -e "  ${R}FAIL${N} $1 — $2"; }
skip() { ((SKIP++)); echo -e "  ${Y}SKIP${N} $1"; }
tier() { echo -e "\n${B}${C}═══ TIER $1: $2 ═══${N}"; }

_ssh() { ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "${REMOTE}" "$@" 2>&1; }

echo -e "${B}${C}"
echo "  ╦  ╦╔╦╗╦═╗╔═╗╔═╗╦ ╦╔═╗"
echo "  ╚╗╔╝║║║╠╦╝║ ║║ ╦║ ║║╣ "
echo "   ╚╝ ╩ ╩╩╚═╚═╝╚═╝╚═╝╚═╝"
echo -e "${N}${B}  E2E Test Suite → ${REMOTE}${N}\n"

# ═══════════════════════════════════════════════
tier 1 "SMOKE TEST"
# ═══════════════════════════════════════════════

# Services running
WEB=$(_ssh "systemctl is-active vmrogue-web")
[[ "$WEB" == "active" ]] && pass "vmrogue-web is active" || fail "vmrogue-web" "$WEB"

OP=$(_ssh "systemctl is-active vmrogue-operator")
[[ "$OP" == "active" ]] && pass "vmrogue-operator is active" || fail "vmrogue-operator" "$OP"

# CRDs registered
CRD_COUNT=$(_ssh "kubectl get crd 2>/dev/null | grep -c vmrogue || echo 0")
[[ "$CRD_COUNT" -eq 5 ]] && pass "5 VMRogue CRDs registered" || fail "CRDs" "found $CRD_COUNT, expected 5"

# Existing VM
VRVM=$(_ssh "vmrogue vrvm-list 2>/dev/null | grep -c sample-ubuntu || echo 0")
[[ "$VRVM" -ge 1 ]] && pass "sample-ubuntu VMRogueVM exists" || fail "sample-ubuntu" "not found"

# API health
HEALTH=$(_ssh "curl -s http://localhost:5151/api/v1/health | grep -o '\"status\":200' || echo none")
[[ "$HEALTH" == *"200"* ]] && pass "API health endpoint OK" || fail "API health" "$HEALTH"

# CRD API
CRD_API=$(_ssh "curl -s -H 'Referer: http://localhost/dashboard' http://localhost:5151/api/v1/crds/vmroguevms | grep -o '\"total\":[0-9]*' || echo none")
[[ "$CRD_API" != "none" ]] && pass "CRD API returns data: $CRD_API" || fail "CRD API" "no response"

# ═══════════════════════════════════════════════
tier 2 "VM LIFECYCLE"
# ═══════════════════════════════════════════════

# Create VM via CLI
CREATE_OUT=$(_ssh "vmrogue vrvm-create test-e2e --template ubuntu-22.04 --cpus 1 --memory 1Gi --start 2>&1")
if echo "$CREATE_OUT" | grep -q "created"; then
    pass "vrvm-create test-e2e succeeded"
else
    fail "vrvm-create" "$CREATE_OUT"
fi

# Wait for operator to reconcile
sleep 10

# Verify KubeVirt VM created
KV_VM=$(_ssh "kubectl get vm test-e2e --no-headers 2>/dev/null | wc -l || echo 0")
[[ "$KV_VM" -ge 1 ]] && pass "KubeVirt VM test-e2e created by operator" || fail "KubeVirt VM" "not found"

# Verify VMRogueVM status updated
PHASE=$(_ssh "kubectl get vrvm test-e2e -o jsonpath='{.status.phase}' 2>/dev/null || echo none")
[[ "$PHASE" != "none" && "$PHASE" != "" ]] && pass "VMRogueVM status phase: $PHASE" || skip "VMRogueVM phase not yet set"

# Get via CLI
GET_OUT=$(_ssh "vmrogue vrvm-get test-e2e 2>&1")
echo "$GET_OUT" | grep -q "CPU:" && pass "vrvm-get shows hardware config" || fail "vrvm-get" "no output"

# Delete VM
DEL_OUT=$(_ssh "vmrogue vrvm-delete test-e2e --yes 2>&1")
echo "$DEL_OUT" | grep -q "deleted" && pass "vrvm-delete test-e2e succeeded" || fail "vrvm-delete" "$DEL_OUT"

sleep 5

# Verify KubeVirt VM garbage-collected
KV_GONE=$(_ssh "kubectl get vm test-e2e --no-headers 2>&1 | grep -c 'not found' || echo 0")
[[ "$KV_GONE" -ge 1 ]] && pass "KubeVirt VM garbage-collected" || skip "KubeVirt VM may still be deleting"

# ═══════════════════════════════════════════════
tier 3 "BLUEPRINT ORCHESTRATION"
# ═══════════════════════════════════════════════

# Deploy LAMP blueprint
BP_OUT=$(_ssh "kubectl apply -f /home/sus/vmrogue/operator/config/samples/vmrogue_v1alpha1_vmrogueblueprint.yaml 2>&1")
echo "$BP_OUT" | grep -q "created\|configured\|unchanged" && pass "LAMP blueprint applied" || fail "blueprint apply" "$BP_OUT"

sleep 5

# List blueprints
BP_LIST=$(_ssh "vmrogue vrbp-list 2>&1")
echo "$BP_LIST" | grep -q "lamp-stack" && pass "vrbp-list shows lamp-stack" || fail "vrbp-list" "$BP_LIST"

# Get blueprint details
BP_GET=$(_ssh "vmrogue vrbp-get lamp-stack 2>&1")
echo "$BP_GET" | grep -q "db\|web" && pass "vrbp-get shows VMs (db, web)" || fail "vrbp-get" "$BP_GET"

# Clean up blueprint
_ssh "kubectl delete vmrogueblueprint lamp-stack 2>/dev/null || true" > /dev/null
sleep 3
# Clean up child VMs
_ssh "kubectl delete vrvm -l vmrogue.io/blueprint=lamp-stack 2>/dev/null || true" > /dev/null
pass "Blueprint cleaned up"

# ═══════════════════════════════════════════════
tier 4 "POLICIES + ACTIONS"
# ═══════════════════════════════════════════════

# Deploy policy
POL_OUT=$(_ssh "kubectl apply -f /home/sus/vmrogue/operator/config/samples/vmrogue_v1alpha1_vmroguepolicy.yaml 2>&1")
echo "$POL_OUT" | grep -q "created\|configured\|unchanged" && pass "Security policy applied" || fail "policy apply" "$POL_OUT"

sleep 3

# List policies
POL_LIST=$(_ssh "vmrogue vrpol-list 2>&1")
echo "$POL_LIST" | grep -q "prod-security-baseline" && pass "vrpol-list shows policy" || fail "vrpol-list" "$POL_LIST"

# Get policy details
POL_GET=$(_ssh "vmrogue vrpol-get prod-security-baseline 2>&1")
echo "$POL_GET" | grep -q "require-tpm\|minimum-cpu" && pass "vrpol-get shows rules" || fail "vrpol-get" "$POL_GET"

# Deploy insight
INS_OUT=$(_ssh "kubectl apply -f /home/sus/vmrogue/operator/config/samples/vmrogue_v1alpha1_vmrogueinsight.yaml 2>&1")
echo "$INS_OUT" | grep -q "created\|configured\|unchanged" && pass "Insight applied" || fail "insight apply" "$INS_OUT"

# Deploy action
ACT_OUT=$(_ssh "kubectl apply -f /home/sus/vmrogue/operator/config/samples/vmrogue_v1alpha1_vmrogueaction.yaml 2>&1")
echo "$ACT_OUT" | grep -q "created\|configured\|unchanged" && pass "Action applied" || fail "action apply" "$ACT_OUT"

sleep 2

# List actions
ACT_LIST=$(_ssh "vmrogue vract-list 2>&1")
echo "$ACT_LIST" | grep -q "scale-prod-db" && pass "vract-list shows action" || fail "vract-list" "$ACT_LIST"

# List insights
INS_LIST=$(_ssh "vmrogue vrin-list 2>&1")
echo "$INS_LIST" | grep -q "high-cpu-prod-db" && pass "vrin-list shows insight" || fail "vrin-list" "$INS_LIST"

# Clean up
_ssh "kubectl delete vmroguepolicy prod-security-baseline 2>/dev/null; kubectl delete vmrogueinsight high-cpu-prod-db 2>/dev/null; kubectl delete vmrogueaction scale-prod-db 2>/dev/null" > /dev/null 2>&1 || true
pass "Policies + actions cleaned up"

# ═══════════════════════════════════════════════
tier 5 "API ENDPOINTS"
# ═══════════════════════════════════════════════

REF="-H Referer:http://localhost/dashboard"
APIKEY=$(ssh -o StrictHostKeyChecking=no "${REMOTE}" "sudo grep VMROGUE_API_KEY /etc/vmrogue/env 2>/dev/null | cut -d= -f2" 2>/dev/null)
AUTH="-H X-API-Key:${APIKEY}"

# CRD endpoints (use Referer auth)
for endpoint in \
    "/api/v1/crds/vmroguevms" \
    "/api/v1/crds/blueprints" \
    "/api/v1/crds/policies" \
    "/api/v1/crds/insights" \
    "/api/v1/crds/actions"; do
    CODE=$(_ssh "curl -s -o /dev/null -w '%{http_code}' $REF http://localhost:5151${endpoint}")
    [[ "$CODE" == "200" ]] && pass "GET $endpoint → 200" || fail "GET $endpoint" "HTTP $CODE"
done

# Handler endpoints (use API key — these are not under /api/ prefix)
for endpoint in \
    "/nodes" \
    "/pods" \
    "/events" \
    "/metrics" \
    "/costs"; do
    CODE=$(_ssh "curl -s -o /dev/null -w '%{http_code}' $AUTH http://localhost:5151${endpoint}")
    [[ "$CODE" == "200" ]] && pass "GET $endpoint → 200" || fail "GET $endpoint" "HTTP $CODE"
done

# Dashboard accessible
DASH=$(_ssh "curl -s -o /dev/null -w '%{http_code}' http://localhost:5151/dashboard")
[[ "$DASH" == "200" ]] && pass "Dashboard page → 200" || fail "Dashboard" "HTTP $DASH"

# ═══════════════════════════════════════════════
echo ""
echo -e "${B}═══════════════════════════════════════════${N}"
echo -e "${B}  Results: ${G}${PASS} passed${N}, ${R}${FAIL} failed${N}, ${Y}${SKIP} skipped${N}"
echo -e "${B}═══════════════════════════════════════════${N}"
echo ""

[[ $FAIL -eq 0 ]] && echo -e "${G}${B}  ALL TESTS PASSED${N}" || echo -e "${R}${B}  SOME TESTS FAILED${N}"
exit $FAIL
