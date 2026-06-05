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
#   4. Ensure CDI (if DataVolume CRD missing) + install CRDs + NATS + RBAC + optional Cilium egress
#   5. Deploy VMRogue API + Operator pods (API key: Admin@321)
#   6. Clean up source
#   7. Verify
#
# CDI: uses scripts/ensure-cdi-remote.sh (VMROGUE_SKIP_CDI=1, VMROGUE_CDI_VERSION=…)
# Cilium: when CRD ciliumnetworkpolicies.cilium.io exists, applies deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml
#         so API/operator/NATS can reach kube-apiserver (default-egress + DNS-only CNPs).
#
# Usage:
#   ./scripts/deploy-all-remote.sh [host] [user]
#   ./scripts/deploy-all-remote.sh [host] [user] --quick   # skip image builds
#
# Preferred wrapper (SSH preflight): ./scripts/deploy-remote.sh [host] [user] [--quick]
# After deploy, smoke-test HTTPS API: ./scripts/verify-vmrogue-remote.sh <host> [node_port]
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/deploy-remote-ui.sh
source "${SCRIPT_DIR}/lib/deploy-remote-ui.sh"
# shellcheck source=lib/deploy-ssh.sh
source "${SCRIPT_DIR}/lib/deploy-ssh.sh"
DEPLOY_RUN_STARTED=${SECONDS}

stream_remote() {
    local label="$1"
    local cmd="$2"
    # Keep the session alive through long podman/cargo builds (avoid client timeout).
    deploy_ssh_stream "${REMOTE}" "${cmd}" 2>&1 | while IFS= read -r line || [[ -n "${line}" ]]; do
        deploy_stream_prefix "${label}"
        printf '%s\n' "${line}"
    done
}

stream_local_rsync() {
    while IFS= read -r line || [[ -n "${line}" ]]; do
        deploy_stream_prefix "rsync"
        printf '%s\n' "${line}"
    done
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

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-185.165.240.5}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
REMOTE="${USER}@${HOST}"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
API_KEY="Admin@321"
VMROGUE_IMAGE="docker.io/library/vmrogue:latest"
OPERATOR_IMAGE="docker.io/library/vmrogue-operator:latest"
NAMESPACE="vmrogue-system"

[ -f "$REPO_DIR/Cargo.toml" ] || { pkg_fail "Not in vmrogue repo (missing Cargo.toml)"; exit 1; }

# ── Resolve remote paths and tools ──
REMOTE_HOME=$(deploy_ssh "${REMOTE}" "echo \$HOME")
DEPLOY_DIR="${REMOTE_HOME}/.deployment/vmrogue"

SUDO=""
[ "$USER" != "root" ] && SUDO="sudo"

# Detect build tool (single-quoted remote script — avoids nested-quote parse issues)
CTR_BUILD=$(deploy_ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; elif command -v nerdctl >/dev/null 2>&1; then echo nerdctl; else echo none; fi')

# Detect K8s runtime for image import (heredoc + explicit bash — robust across shells)
K8S_RUNTIME=$(deploy_ssh "${REMOTE}" env "SUDO=${SUDO:-}" bash -s <<'REMOTE_K8S_RUNTIME'
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
CLUSTER_FLAVOR=$(deploy_ssh "${REMOTE}" 'if [ -x /usr/local/bin/k3s ] || command -v k3s >/dev/null 2>&1; then echo k3s; elif [ -x /usr/local/bin/rke2 ] || command -v rke2 >/dev/null 2>&1; then echo rke2; elif command -v kubeadm >/dev/null 2>&1; then echo kubeadm; elif command -v kubectl >/dev/null 2>&1; then echo kubernetes; else echo unknown; fi')
case "${CLUSTER_FLAVOR}" in
    k3s)
        K8S_CMD="sudo $(deploy_ssh "${REMOTE}" 'which k3s 2>/dev/null || echo /usr/local/bin/k3s') kubectl"
        ;;
    rke2)
        K8S_CMD="sudo $(deploy_ssh "${REMOTE}" 'which rke2 2>/dev/null || echo /usr/local/bin/rke2') kubectl"
        ;;
    *)
        K8S_CMD="kubectl"
        ;;
esac

deploy_main_banner "${REMOTE}" "${CTR_BUILD}" "${K8S_RUNTIME}" "${CLUSTER_FLAVOR}" "${K8S_CMD}" "${QUICK}"

if [ "$CTR_BUILD" = "none" ] && ! $QUICK; then
    deploy_build_fail "${HOST}"
fi

# ── Step 0: Remote diagnostics ──
deploy_phase_start "🩺 [0/7] Remote system and cluster diagnostics"
stream_remote "sysinfo" "set -o pipefail; uname -a; echo ''; cat /etc/os-release 2>/dev/null | sed -n '1,6p'; echo ''; echo \"CPU: \$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo unknown) cores\"; echo \"Memory:\"; free -h 2>/dev/null || vm_stat 2>/dev/null || true; echo ''; echo 'Disk:'; df -h / 2>/dev/null || true"
stream_remote "k8s-info" "set -o pipefail; printf '%s\n' \"Cluster flavor: ${CLUSTER_FLAVOR}\" \"Runtime import path: ${K8S_RUNTIME}\" \"kubectl command: ${K8S_CMD}\"; if command -v kubectl >/dev/null 2>&1 || [ \"${CLUSTER_FLAVOR}\" = \"k3s\" ] || [ \"${CLUSTER_FLAVOR}\" = \"rke2\" ]; then printf '%s\n' ''; ${K8S_CMD} version --short 2>/dev/null || ${K8S_CMD} version 2>/dev/null || true; printf '%s\n' '' 'Nodes:'; ${K8S_CMD} get nodes -o wide 2>/dev/null || true; printf '%s\n' '' 'All namespaces:'; ${K8S_CMD} get ns 2>/dev/null || true; printf '%s\n' '' 'All pods cluster-wide:'; ${K8S_CMD} get pods -A -o wide 2>/dev/null || true; else printf '%s\n' 'kubectl not found on remote host'; fi"
deploy_phase_end

# ── Step 1: Rsync source ──
deploy_phase_start "📦 [1/7] Syncing source to ${HOST}:${DEPLOY_DIR}"
if [ -x "${REPO_DIR}/scripts/prepare-guestkit-docker.sh" ]; then
    deploy_substep "Preparing GuestKit build context (guestkit/ → VMRogue)"
    bash "${REPO_DIR}/scripts/prepare-guestkit-docker.sh"
fi
deploy_ssh "${REMOTE}" "mkdir -p ${DEPLOY_DIR}"
rsync -avz --delete \
    --exclude='target/' --exclude='.git' --exclude='operator/bin/' \
    --exclude='node_modules/' --exclude='e2e/node_modules/' \
    --exclude='*.qcow2' --exclude='*.vmdk' --exclude='*.iso' \
    -e "$(deploy_rsync_ssh)" \
    "$REPO_DIR/" "${REMOTE}:${DEPLOY_DIR}/" 2>&1 | stream_local_rsync
deploy_phase_end

# ── Step 2: Build images ──
echo ""
if $QUICK; then
    deploy_skip_phase "⏭️  [2/7] Skipping image builds (--quick)"
else
    deploy_phase_start "🐳 [2/7] Building container images on remote"

    API_BUILD_CMD="cd ${DEPLOY_DIR} && ${CTR_BUILD} build --no-cache -t ${VMROGUE_IMAGE} ."
    OP_BUILD_CMD="cd ${DEPLOY_DIR}/operator && ${CTR_BUILD} build -t ${OPERATOR_IMAGE} ."
    if [ "${CTR_BUILD}" = "docker" ]; then
        API_BUILD_CMD="cd ${DEPLOY_DIR} && DOCKER_BUILDKIT=1 docker build --progress=plain --no-cache -t ${VMROGUE_IMAGE} ."
        OP_BUILD_CMD="cd ${DEPLOY_DIR}/operator && DOCKER_BUILDKIT=1 docker build --progress=plain -t ${OPERATOR_IMAGE} ."
    fi

    deploy_substep "📦 VMRogue API image build started"
    stream_remote "api-build" "${API_BUILD_CMD}"
    pkg_ok "VMRogue API image build complete"

    deploy_substep "📦 Operator image build started"
    stream_remote "operator-build" "${OP_BUILD_CMD}"
    pkg_ok "Operator image build complete"

    deploy_phase_end
fi

# ── Step 3: Import images into K8s runtime ──
echo ""
if $QUICK; then
    deploy_skip_phase "⏭️  [3/7] Skipping image import (--quick)"
else
    deploy_phase_start "📥 [3/7] Importing images into ${K8S_RUNTIME} runtime"

    case "${K8S_RUNTIME}" in
        k3s)
            K3S_BIN=$(deploy_ssh "${REMOTE}" "which k3s 2>/dev/null || echo /usr/local/bin/k3s")
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
            " 2>&1
            ;;
        rke2)
            RKE2_BIN=$(deploy_ssh "${REMOTE}" "which rke2 2>/dev/null || echo /usr/local/bin/rke2")
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
            " 2>&1
            ;;
        microk8s)
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} > /tmp/vmrogue.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/vmrogue-operator.tar
                microk8s ctr image import /tmp/vmrogue.tar
                microk8s ctr image import /tmp/vmrogue-operator.tar
                rm -f /tmp/vmrogue.tar /tmp/vmrogue-operator.tar
            " 2>&1
            ;;
        kind)
            deploy_ssh "${REMOTE}" "
                kind load docker-image ${VMROGUE_IMAGE}
                kind load docker-image ${OPERATOR_IMAGE}
            " 2>&1
            ;;
        minikube)
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} > /tmp/vmrogue.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/vmrogue-operator.tar
                minikube image load /tmp/vmrogue.tar
                minikube image load /tmp/vmrogue-operator.tar
                rm -f /tmp/vmrogue.tar /tmp/vmrogue-operator.tar
            " 2>&1
            ;;
        containerd)
            # Standard kubeadm K8s with containerd
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VMROGUE_IMAGE} | ${SUDO} ctr -n k8s.io images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ctr -n k8s.io images import -
            " 2>&1
            ;;
        cri)
            # CRI-O or other CRI runtime — import via tar
            deploy_ssh "${REMOTE}" "
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
            deploy_note "Images already in Docker — K8s shares the Docker daemon"
            ;;
        generic)
            pkg_warn "Unknown runtime — images built but import skipped"
            deploy_note "Ensure imagePullPolicy is Never or IfNotPresent"
            ;;
    esac

    deploy_phase_end
fi

# ── Step 4: CDI (if missing) + CRDs + NATS + RBAC + optional Cilium egress ──
echo ""
deploy_phase_start "📋 [4/7] CDI (if missing), CRDs, NATS, RBAC, and Cilium egress (if Cilium)"
"${REPO_DIR}/scripts/ensure-cdi-remote.sh" "${REMOTE}" || {
  pkg_fail "CDI ensure failed (set VMROGUE_SKIP_CDI=1 to skip on air-gapped clusters)"
  exit 1
}
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} create namespace ${NAMESPACE} --dry-run=client -o yaml | ${K8S_CMD} apply -f -
    if [ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" ] && [ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ] && ${K8S_CMD} get crd ciliumnetworkpolicies.cilium.io &>/dev/null && [ -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml ]; then
        echo 'Applying Cilium egress policy for ${NAMESPACE} (vmrogue / nats workloads)...'
        ${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml
    else
        echo 'Skipping Cilium vmrogue egress bootstrap (VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1, no Cilium CRD, or yaml missing)'
    fi
    if [ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" ] && [ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ] && ${K8S_CMD} get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null && [ -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml ]; then
        echo 'Applying Cilium clusterwide egress for KubeVirt virt-launcher (VM guest internet)...'
        ${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml
    fi
    for f in ${DEPLOY_DIR}/operator/config/crd/bases/*.yaml; do
        ${K8S_CMD} apply -f \"\$f\"
    done
    if [ -d ${DEPLOY_DIR}/operator/config/catalog/templates ]; then
        echo 'Applying VMTemplate catalog manifests...'
        for f in ${DEPLOY_DIR}/operator/config/catalog/templates/*.yaml; do
            [ -f \"\$f\" ] && ${K8S_CMD} apply -f \"\$f\" || true
        done
    fi
    if [ -d ${DEPLOY_DIR}/operator/config/catalog/profiles ]; then
        echo 'Applying VMProfile catalog manifests...'
        for f in ${DEPLOY_DIR}/operator/config/catalog/profiles/*.yaml; do
            [ -f \"\$f\" ] && ${K8S_CMD} apply -f \"\$f\" || true
        done
    fi
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/service_account.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/role.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/nats/nats-deployment.yaml
" 2>&1
deploy_phase_end

# ── Step 5: Deploy VMRogue API + Operator ──
echo ""
deploy_phase_start "🚀 [5/7] Deploying VMRogue API + Operator"

# Tear down existing workloads so the next apply creates fresh ReplicaSets/pods.
# --wait ensures objects are gone before apply (avoid races with stale pods).
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} delete deployment vmrogue-api vmrogue-operator \
        --ignore-not-found --wait=true --timeout=180s 2>/dev/null || true
    ${K8S_CMD} -n ${NAMESPACE} delete secret vmrogue-api-key --ignore-not-found --wait=false 2>/dev/null || true
" 2>&1

# Create API key secret
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} create secret generic vmrogue-api-key \
        --from-literal=api-key='${API_KEY}'
" 2>&1

# Apply VMRogue API deployment
deploy_ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s.yaml" 2>&1

# Optional: Prometheus / Alertmanager / Loki URLs when those namespaces exist
if [[ -f "${REPO_DIR}/scripts/lib/bootstrap-integrations.sh" ]]; then
    deploy_substep "🔗 Wiring vmrogue-integrations Secret (if monitoring/loki present)…"
    deploy_ssh "${REMOTE}" "bash ${DEPLOY_DIR}/scripts/lib/bootstrap-integrations.sh '${K8S_CMD}' '${NAMESPACE}'" 2>&1 || \
        pkg_warn "integrations bootstrap skipped or failed (non-fatal)"
fi

# Apply Operator deployment
deploy_ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/manager/manager.yaml" 2>&1

# Same image tags + imagePullPolicy: Never: replacing the Deployment can still leave
# kubelet using a cached layer unless pods are recreated after image import.
deploy_substep "🔄 Recycling API + operator pods for new images…"
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} rollout restart deployment/vmrogue-api deployment/vmrogue-operator 2>/dev/null || true
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/vmrogue-api --timeout=120s
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/vmrogue-operator --timeout=120s
    if ${K8S_CMD} -n ${NAMESPACE} get secret vmrogue-integrations &>/dev/null; then
        ${K8S_CMD} -n ${NAMESPACE} rollout restart deployment/vmrogue-api 2>/dev/null || true
        ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/vmrogue-api --timeout=120s
    fi
" 2>&1 || true
deploy_phase_end

# ── Step 6: Clean up source ──
echo ""
deploy_phase_start "🧹 [6/7] Cleaning up build directory"
deploy_ssh "${REMOTE}" "rm -rf ${DEPLOY_DIR}"
deploy_phase_end

# ── Step 7: Verify ──
echo ""
deploy_phase_start "🔍 [7/7] Verifying"
sleep 3
deploy_ssh "${REMOTE}" "
    echo ''
    echo '  Pods:'
    ${K8S_CMD} -n ${NAMESPACE} get pods -o wide
    echo ''
    echo '  Services:'
    ${K8S_CMD} -n ${NAMESPACE} get svc
" 2>&1
deploy_phase_end

# Do not use ports[0]: API order may list http-redirect (30150) before https (30151).
NODE_PORT=$(deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || echo "30151")
HTTP_REDIRECT_PORT=$(deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"http-redirect\")].nodePort}' 2>/dev/null" || echo "")

deploy_complete "${HOST}" "${USER}" "${REMOTE}" "${NODE_PORT}" "${HTTP_REDIRECT_PORT}" \
    "${API_KEY}" "${SCRIPT_DIR}" "${K8S_CMD}" "${NAMESPACE}"
