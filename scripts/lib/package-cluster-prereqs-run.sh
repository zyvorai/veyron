#!/usr/bin/env bash
# Install cluster prerequisites: Cilium (when applicable), metrics-server, KubeVirt, CDI.
# Run from extracted tarball on a machine with kubectl + cluster-admin kubeconfig.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
[[ -f "${ROOT}/cluster/env.sh" ]] && source "${ROOT}/cluster/env.sh"

SCRIPT="${ROOT}/cluster/install-cluster-prereqs.sh"
[[ -x "${SCRIPT}" ]] || SCRIPT="${ROOT}/cluster/install-cluster-prereqs.sh"
[[ -f "${SCRIPT}" ]] || { echo "ERROR: missing ${SCRIPT}" >&2; exit 1; }

# Map VMROGUE_* / V9S_* skip and version flags (install script uses V9S_* names).
_map() {
  local name="$1"
  local v9s="V9S_${name}"
  local vmr="VMROGUE_${name}"
  if [[ -z "${!v9s:-}" && -n "${!vmr:-}" ]]; then
    export "${v9s}=${!vmr}"
  fi
}
for _v in SKIP_CILIUM SKIP_CDI SKIP_KUBEVIRT SKIP_METRICS_SERVER SKIP_MULTUS \
  CILIUM_CHART_VERSION KUBEVIRT_VERSION CDI_VERSION INSTALL_METRICS_SERVER \
  INSTALL_MULTUS INSTALL_SNAPSHOT_CONTROLLER INSTALL_METALLB INSTALL_PROMETHEUS; do
  _map "${_v}"
done

echo "== Cluster prerequisites (Cilium / KubeVirt / CDI) =="
echo "  See CLUSTER_SETUP.txt for all flags (V9S_* and VMROGUE_* are equivalent)."
echo "  Examples:"
echo "    V9S_SKIP_CILIUM=1 ./install-cluster.sh"
echo "    V9S_SKIP_CDI=1 ./install-cluster.sh"
echo "    VMROGUE_CDI_VERSION=v1.65.0 ./install-cluster.sh"
echo ""

exec bash "${SCRIPT}" "$@"
