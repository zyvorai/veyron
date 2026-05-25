#!/usr/bin/env bash
# Apply Cilium egress bootstrap manifests shipped in this bundle (after Cilium + in-cluster deploy).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
[[ -f "${ROOT}/cluster/env.sh" ]] && source "${ROOT}/cluster/env.sh"

APP_NAMESPACE="${APP_NAMESPACE:-vmrogue-system}"
BOOT="${ROOT}/cluster/bootstrap"

command -v kubectl >/dev/null 2>&1 || { echo "kubectl required"; exit 1; }
export KUBECONFIG="${KUBECONFIG:-${HOME}/.kube/config}"

echo "== ${PRODUCT:-Client} cluster network bootstrap =="
echo "  namespace: ${APP_NAMESPACE}"

if [[ "${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}${V9S_SKIP_CLUSTER_ADDONS:-}${V9S_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}" =~ ^(1|true|yes|TRUE|Yes)$ ]]; then
  echo "  SKIP: egress bootstrap disabled by env"
  exit 0
fi

if ! kubectl get crd ciliumnetworkpolicies.cilium.io >/dev/null 2>&1; then
  echo "  SKIP: no Cilium CRD (non-Cilium cluster)"
  exit 0
fi

_apply() {
  local f="$1"
  [[ -f "${f}" ]] || return 0
  echo "  Applying $(basename "${f}")…"
  if grep -q '__V9S_APP_NAMESPACE__' "${f}" 2>/dev/null; then
    sed "s|__V9S_APP_NAMESPACE__|${APP_NAMESPACE}|g" "${f}" | kubectl apply -f -
  else
    kubectl apply -f "${f}"
  fi
}

for y in \
  "${BOOT}/cilium-app-egress.yaml" \
  "${BOOT}/cilium-vmrogue-egress.yaml" \
  "${BOOT}/v9s-cilium-egress.yaml" \
  "${BOOT}/kubevirt-cilium-egress.yaml"; do
  _apply "${y}"
done

if kubectl get crd ciliumclusterwidenetworkpolicies.cilium.io >/dev/null 2>&1; then
  _apply "${BOOT}/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml"
  _apply "${BOOT}/kubevirt-virt-launcher-clusterwide-egress.yaml"
fi

echo "  Done. Per-VM internet: use ${PRODUCT:-product} API/UI after VMs are running."
echo "  Test: ./test-cluster.sh"
