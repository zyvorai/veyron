#!/usr/bin/env bash
# Veyron install-to-kubernetes.sh — build a container image from THIS bundle (no source
# tree, no cargo toolchain, no internet access needed to build) and deploy it to
# Kubernetes via the bundled Helm chart.
#
# Run this from inside the extracted bundle directory — it expects `Dockerfile`,
# `chart/`, and `veyron` as siblings of this script (all shipped in the bundle).
#
# Usage:
#   ./install-to-kubernetes.sh --registry <registry/repo> [--tag <tag>] [--push]
#                               [--namespace <ns>] [--release <name>] [--dry-run]
#                               [--set key=value ...]
#
# Examples:
#   # Push to your own registry, deploy to the "veyron" namespace:
#   ./install-to-kubernetes.sh --registry registry.example.com/veyron --push
#
#   # Local single-node cluster (kind/k3s/minikube) sharing the build daemon's image
#   # store — no registry/push needed:
#   ./install-to-kubernetes.sh --registry veyron --tag local
#
#   # Preview the Helm changes without applying them:
#   ./install-to-kubernetes.sh --registry registry.example.com/veyron --dry-run
#
# Requires on THIS machine: docker or podman (to build/push), helm 3.12+, a kubeconfig
# pointed at your target cluster. Nothing else from the Veyron source tree is needed.
#
# Cluster prerequisites (KubeVirt, CDI, Cilium) are a separate step — run
# ./install-cluster.sh first if you haven't already (see QUICKSTART.txt).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${HERE}"

REGISTRY=""
TAG="latest"
PUSH=false
NAMESPACE="veyron"
RELEASE="veyron"
DRY_RUN=false
EXTRA_SET=()

usage() { sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
    case "$1" in
        --registry) REGISTRY="$2"; shift 2 ;;
        --tag) TAG="$2"; shift 2 ;;
        --push) PUSH=true; shift ;;
        --namespace) NAMESPACE="$2"; shift 2 ;;
        --release) RELEASE="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --set) EXTRA_SET+=(--set "$2"); shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

[ -f Dockerfile ] || { echo "ERROR: Dockerfile not found — run this from inside the extracted bundle directory" >&2; exit 1; }
[ -d chart ] || { echo "ERROR: chart/ not found — run this from inside the extracted bundle directory" >&2; exit 1; }
[ -x veyron ] || { echo "ERROR: veyron binary not found next to this script" >&2; exit 1; }
if [ -z "${REGISTRY}" ]; then
    echo "ERROR: --registry is required (e.g. --registry registry.example.com/veyron," >&2
    echo "       or a bare name like veyron for a local single-node cluster)" >&2
    exit 1
fi

# virtctl/guestkit are optional in the bundle — the Dockerfile COPYs need them to exist;
# empty placeholders are harmless unless you use virtctl console passthrough or in-guest
# agent auto-install.
touch -a virtctl guestkit
chmod +x virtctl guestkit 2>/dev/null || true

BUILDER=""
if command -v docker &>/dev/null; then
    BUILDER=docker
elif command -v podman &>/dev/null; then
    BUILDER=podman
else
    echo "ERROR: neither docker nor podman found in PATH" >&2
    exit 1
fi
echo "Using ${BUILDER} to build the image"

IMAGE="${REGISTRY}:${TAG}"
echo "Building ${IMAGE} from this bundle (no source tree, no compile step)..."
"${BUILDER}" build -f Dockerfile -t "${IMAGE}" .

if $PUSH; then
    echo "Pushing ${IMAGE}..."
    "${BUILDER}" push "${IMAGE}"
else
    echo "Skipping push (--push not set) — the image only exists in ${BUILDER}'s local store."
    echo "That's enough for a local single-node cluster (kind/k3s/minikube) sharing the same"
    echo "image store; any other cluster needs --push to a registry it can reach."
fi

command -v helm &>/dev/null || { echo "ERROR: helm not found — install Helm 3.12+ (https://helm.sh/docs/intro/install/) then re-run" >&2; exit 1; }
command -v kubectl &>/dev/null || echo "WARNING: kubectl not found — helm will still use your kubeconfig directly, but you won't be able to check rollout status below."

HELM_ARGS=(upgrade --install "${RELEASE}" ./chart --namespace "${NAMESPACE}" --create-namespace
    --set "image.repository=${REGISTRY}" --set "image.tag=${TAG}")
HELM_ARGS+=("${EXTRA_SET[@]}")
$DRY_RUN && HELM_ARGS+=(--dry-run)

echo "Deploying: helm ${HELM_ARGS[*]}"
helm "${HELM_ARGS[@]}"

if ! $DRY_RUN; then
    cat <<EOF

Deployed. Check status:
  kubectl -n ${NAMESPACE} rollout status deployment/${RELEASE}

Retrieve admin credentials:
  kubectl -n ${NAMESPACE} get secret veyron-api-key -o jsonpath='{.data.api-key}' | base64 -d; echo
  kubectl -n ${NAMESPACE} get secret veyron-api-key -o jsonpath='{.data.admin-password}' | base64 -d; echo
See docs/sso.md for the full credential story.
EOF
fi
