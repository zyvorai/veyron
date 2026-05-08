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
#   4. Ensure CDI (if DataVolume CRD missing) + install CRDs + NATS + RBAC
#   5. Deploy VMRogue API + Operator pods (API key: CHANGE_ME)
#   6. Clean up source
#   7. Verify
#
# CDI: uses scripts/ensure-cdi-remote.sh (VMROGUE_SKIP_CDI=1, VMROGUE_CDI_VERSION=…)
#
# Usage:
#   ./scripts/deploy-all-remote.sh [host] [user]
#   ./scripts/deploy-all-remote.sh [host] [user] --quick   # skip image builds
# ============================================================================

set -euo pipefail

timestamp() {
    date +"%H:%M:%S"
}

now_epoch() {
    date +%s
}

format_duration() {
    local total="$1"
    local mins=$((total / 60))
    local secs=$((total % 60))
    printf "%02dm %02ds" "${mins}" "${secs}"
}

log_step() {
    echo "[$(timestamp)] $*"
}

STEP_STARTED_AT=0
STEP_NAME=""
RUN_STARTED_AT="$(now_epoch)"

start_phase() {
    STEP_NAME="$1"
    STEP_STARTED_AT="$(now_epoch)"
    log_step "▶ ${STEP_NAME}"
}

end_phase() {
    local ended_at
    local elapsed
    ended_at="$(now_epoch)"
    elapsed=$((ended_at - STEP_STARTED_AT))
    log_step "✔ ${STEP_NAME} finished in $(format_duration "${elapsed}")"
}

stream_remote() {
    local label="$1"
    local cmd="$2"
    ssh "${REMOTE}" "${cmd}" 2>&1 | sed -e "s/^/  [${label}] /"
}

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

# Detect build tool (single-quoted remote script — avoids nested-quote parse issues)
CTR_BUILD=$(ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; elif command -v nerdctl >/dev/null 2>&1; then echo nerdctl; else echo none; fi')

# Detect K8s runtime for image import (heredoc + explicit bash — robust across shells)
K8S_RUNTIME=$(ssh "${REMOTE}" env "SUDO=${SUDO:-}" bash -s <<'REMOTE_K8S_RUNTIME'
if [ -x /usr/local/bin/k3s ] || command -v k3s >/dev/null 2>&1; then echo k3s
elif [ -x /usr/local/bin/rke2 ] || command -v rke2 >/dev/null 2>&1; then echo rke2
elif command -v microk8s >/dev/null 2>&1; then echo microk8s
elif command -v kind >/dev/null 2>&1; then echo kind
elif command -v minikube >/dev/null 2>&1; then echo minikube
elif command -v docker >/dev/null 2>&1 && docker info 2>/dev/null | grep -q kubernetes; then echo docker-desktop
elif [ -n "${SUDO}" ] && ${SUDO} ctr version >/dev/null 2>&1; then echo containerd
elif [ -n "${SUDO}" ] && ${SUDO} crictl version >/dev/null 2>&1; then echo cri
else echo generic; fi
REMOTE_K8S_RUNTIME
)

# Cluster flavor (kubeadm vs k3s vs generic kubectl) — separate from import path above
CLUSTER_FLAVOR=$(ssh "${REMOTE}" 'if [ -x /usr/local/bin/k3s ] || command -v k3s >/dev/null 2>&1; then echo k3s; elif [ -x /usr/local/bin/rke2 ] || command -v rke2 >/dev/null 2>&1; then echo rke2; elif command -v kubeadm >/dev/null 2>&1; then echo kubeadm; elif command -v kubectl >/dev/null 2>&1; then echo kubernetes; else echo unknown; fi')
case "${CLUSTER_FLAVOR}" in
    k3s)
        K8S_CMD="sudo $(ssh "${REMOTE}" 'which k3s 2>/dev/null || echo /usr/local/bin/k3s') kubectl"
        ;;
    rke2)
        K8S_CMD="sudo $(ssh "${REMOTE}" 'which rke2 2>/dev/null || echo /usr/local/bin/rke2') kubectl"
        ;;
    *)
        K8S_CMD="kubectl"
        ;;
esac

echo ""
echo "  🔥 VMRogue Full K8s Deploy"
echo "  📡 Target:    ${REMOTE}"
echo "  🐳 Builder:   ${CTR_BUILD}"
echo "  ☸️  Runtime:   ${K8S_RUNTIME}"
echo "  🧭 Cluster:   ${CLUSTER_FLAVOR}"
echo "  📎 kubectl:   ${K8S_CMD}"
echo "  🕒 Started:   $(timestamp)"
echo ""

if [ "$CTR_BUILD" = "none" ] && ! $QUICK; then
    echo "❌ No container build tool found on ${HOST} (need podman, docker, or nerdctl)"
    exit 1
fi

# ── Step 0: Remote diagnostics ──
start_phase "🩺 [0/7] Remote system and cluster diagnostics"
stream_remote "sysinfo" "set -o pipefail; uname -a; echo ''; cat /etc/os-release 2>/dev/null | sed -n '1,6p'; echo ''; echo \"CPU: \$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo unknown) cores\"; echo \"Memory:\"; free -h 2>/dev/null || vm_stat 2>/dev/null || true; echo ''; echo 'Disk:'; df -h / 2>/dev/null || true"
stream_remote "k8s-info" "set -o pipefail; printf '%s\n' \"Cluster flavor: ${CLUSTER_FLAVOR}\" \"Runtime import path: ${K8S_RUNTIME}\" \"kubectl command: ${K8S_CMD}\"; if command -v kubectl >/dev/null 2>&1 || [ \"${CLUSTER_FLAVOR}\" = \"k3s\" ] || [ \"${CLUSTER_FLAVOR}\" = \"rke2\" ]; then printf '%s\n' ''; ${K8S_CMD} version --short 2>/dev/null || ${K8S_CMD} version 2>/dev/null || true; printf '%s\n' '' 'Nodes:'; ${K8S_CMD} get nodes -o wide 2>/dev/null || true; printf '%s\n' '' 'All namespaces:'; ${K8S_CMD} get ns 2>/dev/null || true; printf '%s\n' '' 'All pods cluster-wide:'; ${K8S_CMD} get pods -A -o wide 2>/dev/null || true; else printf '%s\n' 'kubectl not found on remote host'; fi"
end_phase

# ── Step 1: Rsync source ──
start_phase "📦 [1/7] Syncing source to ${HOST}:${DEPLOY_DIR}"
ssh "${REMOTE}" "mkdir -p ${DEPLOY_DIR}"
rsync -avz --delete \
    --exclude='target/' --exclude='.git' --exclude='operator/bin/' \
    --exclude='*.qcow2' --exclude='*.vmdk' --exclude='*.iso' \
    -e "ssh -o StrictHostKeyChecking=no" \
    "$REPO_DIR/" "${REMOTE}:${DEPLOY_DIR}/" 2>&1 | sed -e 's/^/  [rsync] /'
end_phase

# ── Step 2: Build images ──
echo ""
if $QUICK; then
    log_step "⏭️  [2/7] Skipping image builds (--quick)"
else
    start_phase "🐳 [2/7] Building container images on remote"

    API_BUILD_CMD="cd ${DEPLOY_DIR} && ${CTR_BUILD} build --no-cache -t ${VMROGUE_IMAGE} ."
    OP_BUILD_CMD="cd ${DEPLOY_DIR}/operator && ${CTR_BUILD} build -t ${OPERATOR_IMAGE} ."
    if [ "${CTR_BUILD}" = "docker" ]; then
        API_BUILD_CMD="cd ${DEPLOY_DIR} && DOCKER_BUILDKIT=1 docker build --progress=plain --no-cache -t ${VMROGUE_IMAGE} ."
        OP_BUILD_CMD="cd ${DEPLOY_DIR}/operator && DOCKER_BUILDKIT=1 docker build --progress=plain -t ${OPERATOR_IMAGE} ."
    fi

    log_step "  📦 VMRogue API image build started"
    stream_remote "api-build" "${API_BUILD_CMD}"
    log_step "  ✅ VMRogue API image build complete"

    log_step "  📦 Operator image build started"
    stream_remote "operator-build" "${OP_BUILD_CMD}"
    log_step "  ✅ Operator image build complete"

    end_phase
fi

# ── Step 3: Import images into K8s runtime ──
echo ""
if $QUICK; then
    log_step "⏭️  [3/7] Skipping image import (--quick)"
else
    start_phase "📥 [3/7] Importing images into ${K8S_RUNTIME} runtime"

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

    end_phase
fi

# ── Step 4: CDI (if missing) + CRDs + NATS + RBAC ──
echo ""
start_phase "📋 [4/7] CDI (if missing), CRDs, NATS, and RBAC"
"${REPO_DIR}/scripts/ensure-cdi-remote.sh" "${REMOTE}" || {
  echo "❌ CDI ensure failed (set VMROGUE_SKIP_CDI=1 to skip on air-gapped clusters)"
  exit 1
}
ssh "${REMOTE}" "
    ${K8S_CMD} create namespace ${NAMESPACE} --dry-run=client -o yaml | ${K8S_CMD} apply -f -
    for f in ${DEPLOY_DIR}/operator/config/crd/bases/*.yaml; do
        ${K8S_CMD} apply -f \"\$f\"
    done
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/service_account.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/role.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/nats/nats-deployment.yaml
" 2>&1
end_phase

# ── Step 5: Deploy VMRogue API + Operator ──
echo ""
start_phase "🚀 [5/7] Deploying VMRogue API + Operator"

# Tear down existing workloads so the next apply creates fresh ReplicaSets/pods.
# --wait ensures objects are gone before apply (avoid races with stale pods).
ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} delete deployment vmrogue-api vmrogue-operator \
        --ignore-not-found --wait=true --timeout=180s 2>/dev/null || true
    ${K8S_CMD} -n ${NAMESPACE} delete secret vmrogue-api-key --ignore-not-found --wait=false 2>/dev/null || true
" 2>&1

# Create API key secret
ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} create secret generic vmrogue-api-key \
        --from-literal=api-key='${API_KEY}'
" 2>&1

# Apply VMRogue API deployment
ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s.yaml" 2>&1

# Apply Operator deployment
ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/manager/manager.yaml" 2>&1

# Same image tags + imagePullPolicy: Never: replacing the Deployment can still leave
# kubelet using a cached layer unless pods are recreated after image import.
log_step "  🔄 Recycling API + operator pods for new images..."
ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} rollout restart deployment/vmrogue-api deployment/vmrogue-operator 2>/dev/null || true
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/vmrogue-api --timeout=120s
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/vmrogue-operator --timeout=120s
" 2>&1 || true
end_phase

# ── Step 6: Clean up source ──
echo ""
start_phase "🧹 [6/7] Cleaning up build directory"
ssh "${REMOTE}" "rm -rf ${DEPLOY_DIR}"
end_phase

# ── Step 7: Verify ──
echo ""
start_phase "🔍 [7/7] Verifying"
sleep 3
ssh "${REMOTE}" "
    echo ''
    echo '  Pods:'
    ${K8S_CMD} -n ${NAMESPACE} get pods -o wide
    echo ''
    echo '  Services:'
    ${K8S_CMD} -n ${NAMESPACE} get svc
" 2>&1
end_phase

# Do not use ports[0]: API order may list http-redirect (30150) before https (30151).
NODE_PORT=$(ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || echo "30151")
HTTP_REDIRECT_PORT=$(ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"http-redirect\")].nodePort}' 2>/dev/null" || echo "")

echo ""
echo "════════════════════════════════════════"
echo "  ✅ Deployment complete"
echo "════════════════════════════════════════"
echo "  ⏱️  Total time: $(format_duration "$(( $(now_epoch) - RUN_STARTED_AT ))")"
echo "  🌐 Dashboard:  https://${HOST}:${NODE_PORT}/dashboard"
echo "  💚 Health:     https://${HOST}:${NODE_PORT}/api/v1/health"
if [ -n "${HTTP_REDIRECT_PORT}" ]; then
    echo "  ↪️  HTTP→HTTPS redirect also on node port ${HTTP_REDIRECT_PORT}"
fi
echo "  📓 Pod logs show :5151 (container); use NodePort ${NODE_PORT} in the browser"
echo "  🔑 API Key:    ${API_KEY}"
echo "  📋 API Logs:   ssh ${REMOTE} \"${K8S_CMD} -n ${NAMESPACE} logs -l app.kubernetes.io/component=api -f\""
echo "  📋 Op Logs:    ssh ${REMOTE} \"${K8S_CMD} -n ${NAMESPACE} logs -l app.kubernetes.io/component=operator -f\""
echo "════════════════════════════════════════"
echo ""
