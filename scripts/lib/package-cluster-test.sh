#!/usr/bin/env bash
# Verify Kubernetes cluster prerequisites for VMRogue / v9s client bundles.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
[[ -f "${ROOT}/cluster/env.sh" ]] && source "${ROOT}/cluster/env.sh"

PRODUCT="${PRODUCT:-Kubernetes client}"
APP_PORT="${APP_PORT:-5151}"
ENV_PREFIX="${ENV_PREFIX:-VMROGUE}"
KUBECONFIG="${KUBECONFIG:-${HOME}/.kube/config}"

PASS=0 WARN=0 FAIL=0
ok()   { echo "  OK: $*"; PASS=$((PASS + 1)); }
warn() { echo "  WARN: $*"; WARN=$((WARN + 1)); }
fail() { echo "  FAIL: $*"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP: $*"; }

echo "== ${PRODUCT} cluster test =="
echo "  KUBECONFIG=${KUBECONFIG}"

if ! command -v kubectl >/dev/null 2>&1; then
  fail "kubectl not in PATH (run ./install-client-deps.sh)"
  echo "Summary: ${PASS} ok, ${WARN} warn, ${FAIL} fail"
  exit 1
fi

export KUBECONFIG
if ! kubectl cluster-info >/dev/null 2>&1; then
  fail "Cannot reach cluster (check KUBECONFIG and network)"
  echo "Summary: ${PASS} ok, ${WARN} warn, ${FAIL} fail"
  exit 1
fi
ok "kubectl reaches API server"

if kubectl get crd virtualmachines.kubevirt.io >/dev/null 2>&1; then
  ok "KubeVirt CRD (virtualmachines.kubevirt.io)"
  phase=$(kubectl get kubevirt kubevirt -n kubevirt -o jsonpath='{.status.phase}' 2>/dev/null || true)
  if [[ "${phase}" == "Deployed" ]]; then
    ok "KubeVirt operator phase=Deployed"
  else
    warn "KubeVirt phase=${phase:-unknown} (run ./install-cluster.sh or wait)"
  fi
else
  fail "KubeVirt not installed — run: ./install-cluster.sh"
fi

if kubectl get crd datavolumes.cdi.kubevirt.io >/dev/null 2>&1; then
  ok "CDI DataVolume CRD"
  if kubectl get cdi cdi -o jsonpath='{.status.conditions[?(@.type=="Available")].status}' 2>/dev/null | grep -qi true; then
    ok "CDI Available"
  else
    warn "CDI not yet Available (import disks may fail until ready)"
  fi
else
  warn "CDI not installed (optional for basic VMs; required for DataVolume/import) — set ${ENV_PREFIX}_SKIP_CDI=1 or run ./install-cluster.sh"
fi

if kubectl get crd ciliumnetworkpolicies.cilium.io >/dev/null 2>&1; then
  ok "Cilium CRDs present"
  if [[ -f "${ROOT}/cluster/bootstrap/cilium-app-egress.yaml" ]] || \
     [[ -f "${ROOT}/cluster/bootstrap/cilium-vmrogue-egress.yaml" ]] || \
     [[ -f "${ROOT}/cluster/bootstrap/v9s-cilium-egress.yaml" ]]; then
    if [[ -x "${ROOT}/apply-cluster-network.sh" ]]; then
      skip "Run ./apply-cluster-network.sh after deploying ${PRODUCT} in-cluster"
    fi
  fi
  if kubectl get crd ciliumclusterwidenetworkpolicies.cilium.io >/dev/null 2>&1; then
    if kubectl get ciliumclusterwidenetworkpolicy allow-kubevirt-virt-launcher-egress >/dev/null 2>&1; then
      ok "Cilium CCNP for virt-launcher egress"
    else
      warn "CCNP allow-kubevirt-virt-launcher-egress missing — VM guests may have no internet (run ./apply-cluster-network.sh)"
    fi
  fi
else
  skip "Cilium not detected (non-Cilium CNI — skip Cilium bootstrap)"
fi

if kubectl get deployment metrics-server -n kube-system >/dev/null 2>&1; then
  ok "metrics-server"
else
  skip "metrics-server not installed (optional; install-cluster.sh installs by default)"
fi

if [[ -n "${APP_NAMESPACE:-}" ]] && kubectl get namespace "${APP_NAMESPACE}" >/dev/null 2>&1; then
  ok "namespace ${APP_NAMESPACE} exists"
else
  skip "namespace ${APP_NAMESPACE:-<not set>} not found (create when you deploy ${PRODUCT} in-cluster)"
fi

echo ""
echo "Summary: ${PASS} ok, ${WARN} warn, ${FAIL} fail"
[[ "${FAIL}" -eq 0 ]]
