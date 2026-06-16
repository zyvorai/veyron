#!/usr/bin/env bash
# Legacy entry point — VMRogue is deployed only on Kubernetes (k3s or other clusters).
# This script forwards to the supported remote deploy flow.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "VMRogue remote deployment is Kubernetes-only (no systemd on the node)."
  echo ""
  echo "Use:"
  echo "  ./scripts/deploy-k8s-remote.sh <host> [user]"
  echo ""
  echo "Optional: VEYRON_API_KEY, VMROGUE_NODE_PORT, VEYRON_NAMESPACE (see that script)."
  exit 0
fi

exec "${REPO_ROOT}/scripts/deploy-k8s-remote.sh" "$@"
