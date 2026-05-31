#!/usr/bin/env bash
# ============================================================================
# ensure-cdi-remote.sh — Install CDI on a remote cluster when DataVolume CRD is missing
# ============================================================================
# Invoked over SSH; requires outbound HTTPS to GitHub for release manifests.
# Works on k3s and on generic Kubernetes nodes where kubectl reaches the cluster API.
#
# Usage:
#   ./scripts/ensure-cdi-remote.sh <user@host> [cdi-version]
#
# Environment:
#   VMROGUE_SKIP_CDI=1       — exit 0 without doing anything
#   VMROGUE_CDI_VERSION=v1.65.0 — release tag (must match your KubeVirt line)
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/deploy-ssh.sh
source "${SCRIPT_DIR}/lib/deploy-ssh.sh"

REMOTE="${1:?Usage: $0 <user@host> [cdi-version]}"
CDI_VERSION="${2:-${VMROGUE_CDI_VERSION:-v1.65.0}}"

if [[ "${VMROGUE_SKIP_CDI:-0}" == "1" ]]; then
  echo "  [skip] CDI (VMROGUE_SKIP_CDI=1)"
  exit 0
fi

_ssh() {
  deploy_ssh "${REMOTE}" "$@"
}

K_MODE=$(_ssh 'if [ -x /usr/local/bin/k3s ]; then echo k3s_std; elif command -v k3s >/dev/null 2>&1; then echo k3s_path; elif command -v kubectl >/dev/null 2>&1; then echo kubectl_plain; else echo ""; fi' | tr -d '\r')
case "${K_MODE}" in
  k3s_std) REMOTE_K=(sudo /usr/local/bin/k3s kubectl) ;;
  k3s_path) REMOTE_K=(sudo k3s kubectl) ;;
  kubectl_plain) REMOTE_K=(kubectl) ;;
  *)
    echo "  [✗] Could not find k3s or kubectl on ${REMOTE}" >&2
    exit 1
    ;;
esac

if _ssh "${REMOTE_K[@]}" get crd datavolumes.cdi.kubevirt.io >/dev/null 2>&1; then
  echo "  [✓] CDI already present (datavolumes.cdi.kubevirt.io CRD)"
  exit 0
fi

echo "  […] Installing CDI ${CDI_VERSION} (operator + cdi CR)…"
_ssh "${REMOTE_K[@]}" apply -f "https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-operator.yaml"
_ssh "${REMOTE_K[@]}" apply -f "https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-cr.yaml"

# Wait until the CDI custom resource exists (cdi-cr can take a few seconds).
for _ in $(seq 1 60); do
  if _ssh "${REMOTE_K[@]}" get cdi cdi >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

if ! _ssh "${REMOTE_K[@]}" get cdi cdi >/dev/null 2>&1; then
  echo "  [✗] CDI custom resource 'cdi' did not appear — check operator logs in namespace cdi" >&2
  exit 1
fi

if ! _ssh "${REMOTE_K[@]}" wait cdi cdi --for=condition=Available --timeout=600s 2>/dev/null; then
  _ssh "${REMOTE_K[@]}" wait cdi cdi --for=jsonpath='{.status.phase}'=Deployed --timeout=600s
fi

if ! _ssh "${REMOTE_K[@]}" get crd datavolumes.cdi.kubevirt.io >/dev/null 2>&1; then
  echo "  [✗] CDI installed but DataVolume CRD still missing" >&2
  exit 1
fi

echo "  [✓] CDI ready (DataVolume CRD registered)"
exit 0
