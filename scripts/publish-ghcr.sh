#!/usr/bin/env bash
# Publish Docker image + Helm charts to ghcr.io/zyvorai from the remote build server.
# The remote already has the built image from the last deploy — no rebuild needed.
#
# Usage:
#   GHCR_TOKEN=<pat> ./scripts/publish-ghcr.sh [host] [user] [version]
#
# Example:
#   GHCR_TOKEN=ghp_xxx ./scripts/publish-ghcr.sh HOST sus 0.3.0

set -euo pipefail

HOST="${1:-HOST}"
RUSER="${2:-sus}"
VERSION="${3:-0.3.0}"
REMOTE="${RUSER}@${HOST}"
ORG="ssahani"

if [[ -z "${GHCR_TOKEN:-}" ]]; then
  echo "Error: set GHCR_TOKEN before running" >&2
  echo "  export GHCR_TOKEN='ghp_...'" >&2
  exit 1
fi

echo "==> Publishing veyron v${VERSION} to ghcr.io/${ORG}"

# ── 1. Push Docker image ────────────────────────────────────────────────────
echo "==> [1/2] Pushing Docker image to ghcr.io/${ORG}/veyron"
ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail
echo "${GHCR_TOKEN}" | podman login ghcr.io -u ${ORG} --password-stdin
podman tag docker.io/library/veyron:latest ghcr.io/${ORG}/veyron:${VERSION}
podman tag docker.io/library/veyron:latest ghcr.io/${ORG}/veyron:latest
podman push ghcr.io/${ORG}/veyron:${VERSION}
podman push ghcr.io/${ORG}/veyron:latest
echo "Image pushed: ghcr.io/${ORG}/veyron:${VERSION}"
ENDSSH

# ── 2. Push Helm charts ─────────────────────────────────────────────────────
echo "==> [2/2] Packaging and pushing Helm charts to oci://ghcr.io/${ORG}/charts"

# Sync charts to remote
CHARTS_DIR="/tmp/veyron-charts-${VERSION}"
ssh "$REMOTE" "mkdir -p ${CHARTS_DIR}"
rsync -az --delete charts/ "${REMOTE}:${CHARTS_DIR}/"

ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail

# Install helm if missing
if ! command -v helm &>/dev/null; then
  curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
fi

echo "${GHCR_TOKEN}" | helm registry login ghcr.io -u ${ORG} --password-stdin

mkdir -p /tmp/helm-packages
for chart in ${CHARTS_DIR}/veyron ${CHARTS_DIR}/veyron-operator ${CHARTS_DIR}/veyron-monitoring; do
  helm package "\$chart" -d /tmp/helm-packages
done

for tgz in /tmp/helm-packages/*.tgz; do
  echo "Pushing \$tgz"
  helm push "\$tgz" oci://ghcr.io/${ORG}/charts
done

rm -rf /tmp/helm-packages ${CHARTS_DIR}
echo "Charts pushed to oci://ghcr.io/${ORG}/charts"
ENDSSH

echo ""
echo "✓ Done. Customer install:"
echo "  helm install veyron oci://ghcr.io/${ORG}/charts/veyron --version ${VERSION} \\"
echo "    --namespace veyron-system --create-namespace"
