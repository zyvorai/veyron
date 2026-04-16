#!/usr/bin/env bash
# ============================================================================
# deploy-all-remote.sh — Full VMRogue + Operator K8s deployment to remote host
# ============================================================================
# Works with any combination of:
#   Build:   podman / docker / nerdctl
#   Runtime: k3s (containerd) / k8s (docker/containerd) / kind / microk8s
#
# Flow:
#   1. Rsync source to remote ~/.deployment/vmrogue
#   2. Build VMRogue + Operator container images
#   3. Import images into K8s container runtime
#   4. Install CRDs + NATS + RBAC
#   5. Deploy VMRogue API + Operator pods (API key: CHANGE_ME)
#   6. Clean up source
#   7. Verify
#
# Usage:
#   ./scripts/deploy-all-remote.sh [host] [user]
#   ./scripts/deploy-all-remote.sh [host] [user] --quick   # skip image builds
# ============================================================================

set -euo pipefail

QUICK=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --quick) QUICK=true ;;
        --help|-h)
            echo "Usage: $0 [host] [user] [--quick]"
            echo "  --quick  Skip image builds (redeploy manifests only)"
            exit 0
            ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-HOST}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
REMOTE="${USER}@${HOST}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
API_KEY="CHANGE_ME"
VMROGUE_IMAGE="docker.io/library/vmrogue:latest"
OPERATOR_IMAGE="docker.io/library/vmrogue-operator:latest"
NAMESPACE="vmrogue-system"

[ -f "$REPO_DIR/Cargo.toml" ] || { echo "❌ Not in vmrogue repo"; exit 1; }

# ── Resolve remote paths and tools ──
REMOTE_HOME=$(ssh "${REMOTE}" "echo \$HOME")
DEPLOY_DIR="${REMOTE_HOME}/.deployment/vmrogue"

SUDO=""
[ "$USER" != "root" ] && SUDO="sudo"

# Detect build tool
CTR_BUILD=$(ssh "${REMOTE}" "
    if command -v podman &>/dev/null; then echo podman
    elif command -v docker &>/dev/null; then echo docker
    elif command -v nerdctl &>/dev/null; then echo nerdctl
    else echo none; fi
")

# Detect K8s runtime for image import
K8S_RUNTIME=$(ssh "${REMOTE}" "
    if [ -x /usr/local/bin/k3s ] || command -v k3s &>/dev/null 2>&1; then echo k3s
    elif [ -x /usr/local/bin/rke2 ] || command -v rke2 &>/dev/null 2>&1; then echo rke2
    elif command -v microk8s &>/dev/null; then echo microk8s
    elif command -v kind &>/dev/null; then echo kind
    elif command -v minikube &>/dev/null; then echo minikube
    elif command -v docker &>/dev/null && docker info 2>/dev/null | grep -q 'kubernetes'; then echo docker-desktop
    elif ${SUDO:-} ctr version &>/dev/null 2>&1; then echo containerd
    elif ${SUDO:-} crictl version &>/dev/null 2>&1; then echo cri
    else echo generic; fi
")

echo ""
echo "  🔥 VMRogue Full K8s Deploy"
echo "  📡 Target:    ${REMOTE}"
echo "  🐳 Builder:   ${CTR_BUILD}"
echo "  ☸️  Runtime:   ${K8S_RUNTIME}"
echo ""

if [ "$CTR_BUILD" = "none" ] && ! $QUICK; then
    echo "❌ No container build tool found on ${HOST} (need podman, docker, or nerdctl)"
    exit 1
fi

# ── Step 1: Rsync source ──
echo "📦 [1/7] Syncing source to ${HOST}:${DEPLOY_DIR}"
ssh "${REMOTE}" "mkdir -p ${DEPLOY_DIR}"
rsync -avz --delete \
    --exclude='target/' --exclude='.git' --exclude='operator/bin/' \
    --exclude='*.qcow2' --exclude='*.vmdk' --exclude='*.iso' \
    -e "ssh -o StrictHostKeyChecking=no" \
    "$REPO_DIR/" "${REMOTE}:${DEPLOY_DIR}/" 2>&1 | tail -3
echo "✅ Source synced"

# ── Step 2: Build images ──
echo ""
if $QUICK; then
    echo "⏭️  [2/7] Skipping image builds (--quick)"
else
    echo "🐳 [2/7] Building container images on remote"

    echo "  📦 VMRogue API image..."
    ssh "${REMOTE}" "cd ${DEPLOY_DIR} && ${CTR_BUILD} build --no-cache -t ${VMROGUE_IMAGE} ." 2>&1 | tail -3

    echo "  📦 Operator image..."
    ssh "${REMOTE}" "cd ${DEPLOY_DIR}/operator && ${CTR_BUILD} build -t ${OPERATOR_IMAGE} ." 2>&1 | tail -3

    echo "✅ Both images built"
fi

# ── Step 3: Import images into K8s runtime ──
echo ""
if $QUICK; then
    echo "⏭️  [3/7] Skipping image import (--quick)"
else
    echo "📥 [3/7] Importing images into ${K8S_RUNTIME} runtime"

    case "${K8S_RUNTIME}" in
        k3s)
            K3S_BIN=$(ssh "${REMOTE}" "which k3s 2>/dev/null || echo /usr/local/bin/k3s")
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
            " 2>&1
            ;;
        rke2)
            RKE2_BIN=$(ssh "${REMOTE}" "which rke2 2>/dev/null || echo /usr/local/bin/rke2")
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
            " 2>&1
            ;;
        microk8s)
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} > /tmp/vmrogue.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/vmrogue-operator.tar
                microk8s ctr image import /tmp/vmrogue.tar
                microk8s ctr image import /tmp/vmrogue-operator.tar
                rm -f /tmp/vmrogue.tar /tmp/vmrogue-operator.tar
            " 2>&1
            ;;
        kind)
            ssh "${REMOTE}" "
                kind load docker-image ${VMROGUE_IMAGE}
                kind load docker-image ${OPERATOR_IMAGE}
            " 2>&1
            ;;
        minikube)
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} > /tmp/vmrogue.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/vmrogue-operator.tar
                minikube image load /tmp/vmrogue.tar
                minikube image load /tmp/vmrogue-operator.tar
                rm -f /tmp/vmrogue.tar /tmp/vmrogue-operator.tar
            " 2>&1
            ;;
        containerd)
            # Standard kubeadm K8s with containerd
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ctr -n k8s.io images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ctr -n k8s.io images import -
            " 2>&1
            ;;
        cri)
            # CRI-O or other CRI runtime — import via tar
            ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} > /tmp/vmrogue.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/vmrogue-operator.tar
                ${SUDO} crictl pull /tmp/vmrogue.tar 2>/dev/null || \
                    ${SUDO} ctr -n k8s.io images import /tmp/vmrogue.tar
                ${SUDO} crictl pull /tmp/vmrogue-operator.tar 2>/dev/null || \
                    ${SUDO} ctr -n k8s.io images import /tmp/vmrogue-operator.tar
                rm -f /tmp/vmrogue.tar /tmp/vmrogue-operator.tar
            " 2>&1
            ;;
        docker-desktop)
            echo "  Images already in Docker — K8s shares the Docker daemon"
            ;;
        generic)
            echo "  ⚠️  Unknown runtime — images built but import skipped"
            echo "  Ensure imagePullPolicy is set to Never or IfNotPresent"
            ;;
    esac

    echo "✅ Images imported"
fi

# ── Step 4: Install CRDs + NATS + RBAC ──
echo ""
echo "📋 [4/7] Installing CRDs, NATS, and RBAC"
ssh "${REMOTE}" "
    kubectl create namespace ${NAMESPACE} --dry-run=client -o yaml | kubectl apply -f -
    for f in ${DEPLOY_DIR}/operator/config/crd/bases/*.yaml; do
        kubectl apply -f \"\$f\"
    done
    kubectl apply -f ${DEPLOY_DIR}/operator/config/rbac/service_account.yaml
    kubectl apply -f ${DEPLOY_DIR}/operator/config/rbac/role.yaml
    kubectl apply -f ${DEPLOY_DIR}/operator/config/nats/nats-deployment.yaml
" 2>&1
echo "✅ CRDs + NATS + RBAC deployed"

# ── Step 5: Deploy VMRogue API + Operator ──
echo ""
echo "🚀 [5/7] Deploying VMRogue API + Operator"

# Tear down existing deployments for clean rollout
ssh "${REMOTE}" "
    kubectl -n ${NAMESPACE} delete deployment vmrogue-api --ignore-not-found 2>/dev/null
    kubectl -n ${NAMESPACE} delete deployment vmrogue-operator --ignore-not-found 2>/dev/null
    kubectl -n ${NAMESPACE} delete secret vmrogue-api-key --ignore-not-found 2>/dev/null
    sleep 2
" 2>&1

# Create API key secret
ssh "${REMOTE}" "
    kubectl -n ${NAMESPACE} create secret generic vmrogue-api-key \
        --from-literal=api-key='${API_KEY}'
" 2>&1

# Apply VMRogue API deployment
ssh "${REMOTE}" "kubectl apply -f ${DEPLOY_DIR}/deploy/k8s.yaml" 2>&1

# Apply Operator deployment
ssh "${REMOTE}" "kubectl apply -f ${DEPLOY_DIR}/operator/config/manager/manager.yaml" 2>&1

echo "  ⏳ Waiting for rollouts..."
ssh "${REMOTE}" "
    kubectl -n ${NAMESPACE} rollout status deployment/vmrogue-api --timeout=120s
    kubectl -n ${NAMESPACE} rollout status deployment/vmrogue-operator --timeout=120s
" 2>&1 || true
echo "✅ Deployed"

# ── Step 6: Clean up source ──
echo ""
echo "🧹 [6/7] Cleaning up build directory"
ssh "${REMOTE}" "rm -rf ${DEPLOY_DIR}"
echo "✅ Source removed"

# ── Step 7: Verify ──
echo ""
echo "🔍 [7/7] Verifying"
sleep 3
ssh "${REMOTE}" "
    echo ''
    echo '  Pods:'
    kubectl -n ${NAMESPACE} get pods -o wide
    echo ''
    echo '  Services:'
    kubectl -n ${NAMESPACE} get svc
" 2>&1

NODE_PORT=$(ssh "${REMOTE}" "kubectl -n ${NAMESPACE} get svc vmrogue-api -o jsonpath='{.spec.ports[0].nodePort}' 2>/dev/null" || echo "30151")

echo ""
echo "════════════════════════════════════════"
echo "  ✅ Deployment complete"
echo "════════════════════════════════════════"
echo "  🌐 Dashboard:  https://${HOST}:${NODE_PORT}/dashboard"
echo "  🔑 API Key:    ${API_KEY}"
echo "  📋 API Logs:   kubectl -n ${NAMESPACE} logs -l app.kubernetes.io/component=api -f"
echo "  📋 Op Logs:    kubectl -n ${NAMESPACE} logs -l app.kubernetes.io/component=operator -f"
echo "════════════════════════════════════════"
echo ""
