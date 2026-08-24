#!/usr/bin/env bash
# ============================================================================
# deploy-all-remote.sh — Full Veyron + Operator K8s deployment to remote host
# ============================================================================
# Works with any combination of:
#   Build:   podman / docker / nerdctl
#   Runtime: k3s (containerd) / k8s (docker/containerd) / kind / microk8s
#
# Flow:
#   1. Rsync source to remote ~/.deployment/veyron
#   2. Build Veyron + Operator container images
#   3. Import images into K8s container runtime
#   4. Ensure CDI (if DataVolume CRD missing) + install CRDs + NATS + RBAC + optional Cilium egress
#   5. Deploy Veyron API + Operator pods (API key: CHANGE_ME)
#   6. Clean up source
#   7. Verify
#
# CDI: uses scripts/ensure-cdi-remote.sh (VEYRON_SKIP_CDI=1, VEYRON_CDI_VERSION=…)
# Cilium: when CRD ciliumnetworkpolicies.cilium.io exists, applies deploy/k8s/bootstrap/cilium-veyron-egress.yaml
#         so API/operator/NATS can reach kube-apiserver (default-egress + DNS-only CNPs).
#
# Usage:
#   ./scripts/deploy-all-remote.sh [host] [user]
#   ./scripts/deploy-all-remote.sh [host] [user] --quick   # skip image builds
#
# Preferred wrapper (SSH preflight): ./scripts/deploy-remote.sh [host] [user] [--quick]
# After deploy, smoke-test HTTPS API: ./scripts/verify-veyron-remote.sh <host> [node_port]
#
# Speed (default: fast incremental builds):
#   VEYRON_DEPLOY_NO_CACHE=1           Force clean image build (slow; was the old default)
#   VEYRON_DEPLOY_DIAGNOSTICS=1        Full cluster diagnostics before rsync (slow on large clusters)
#   VEYRON_SKIP_GUESTKIT_PREP=1        Skip local guestkit rsync when guestkit/ is already current
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
DEPLOY_WITH_OIDC=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --quick) QUICK=true ;;
        --with-oidc) DEPLOY_WITH_OIDC=true ;;
        --help|-h)
            echo "Usage: $0 [host] [user] [--quick] [--with-oidc]"
            echo "  --quick      Skip image builds (redeploy manifests only)"
            echo "  --with-oidc  Deploy enterprise SSO config from VEYRON_OIDC_* env vars"
            echo "               (see contrib/veyron-oidc-keycloak.env.example)"
            exit 0
            ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-}}"
if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [user] [--quick]" >&2
    echo "  or:  DEPLOY_HOST=<host> [DEPLOY_USER=<user>] $0 [--quick]" >&2
    exit 1
fi
if [[ -z "${USER}" ]]; then
    echo "Usage: $0 <host> <user> [--quick]" >&2
    echo "  or:  DEPLOY_HOST=<host> DEPLOY_USER=<user> $0 [--quick]" >&2
    exit 1
fi
REMOTE="${USER}@${HOST}"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
API_KEY="${VEYRON_API_KEY:-CHANGE_ME}"
VEYRON_IMAGE="docker.io/library/veyron:latest"
OPERATOR_IMAGE="docker.io/library/veyron-operator:latest"
NAMESPACE="${VEYRON_NAMESPACE:-veyron-system}"

[ -f "$REPO_DIR/Cargo.toml" ] || { pkg_fail "Not in veyron repo (missing Cargo.toml)"; exit 1; }

# ── Preflight: operator Go toolchain consistency ──
# The operator image builds with GOTOOLCHAIN=local, so the Dockerfile's golang
# base must be >= operator/go.mod's `go` directive — otherwise `go mod download`
# fails deep in the build with "go.mod requires go >= X (running Y)".
if [[ -f "$REPO_DIR/operator/go.mod" && -f "$REPO_DIR/operator/Dockerfile" ]]; then
    _gomod_go="$(grep -m1 -E '^go [0-9]' "$REPO_DIR/operator/go.mod" | awk '{print $2}' | cut -d. -f1-2)"
    _docker_go="$(grep -m1 -oE 'golang:[0-9]+\.[0-9]+' "$REPO_DIR/operator/Dockerfile" | cut -d: -f2)"
    if [[ -n "$_gomod_go" && -n "$_docker_go" ]]; then
        _older="$(printf '%s\n%s\n' "$_gomod_go" "$_docker_go" | sort -V | head -1)"
        if [[ "$_older" == "$_docker_go" && "$_docker_go" != "$_gomod_go" ]]; then
            pkg_fail "Operator Go mismatch: operator/go.mod requires go ${_gomod_go} but operator/Dockerfile pins golang:${_docker_go}. Bump the Dockerfile base image (GOTOOLCHAIN=local can't auto-upgrade)."
            exit 1
        fi
    fi
fi

# ── Resolve remote paths and tools ──
REMOTE_HOME=$(deploy_ssh "${REMOTE}" "echo \$HOME")
DEPLOY_DIR="${REMOTE_HOME}/.deployment/veyron"

# ── Serialize deploys targeting the same host ──
# DEPLOY_DIR is shared remote state with no isolation between runs. A second
# deploy started before a first one finishes will have its freshly-synced
# source wiped by the first run's own step-6 `rm -rf ${DEPLOY_DIR}` cleanup,
# mid-pipeline, right as it tries to apply CRDs/RBAC/NATS — silent corruption,
# not a clean failure. Fail fast instead of racing. `mkdir` (not flock, which
# isn't installed on macOS by default) gives a portable atomic lock.
DEPLOY_LOCK_DIR="/tmp/veyron-deploy-${HOST}.lock.d"
if ! mkdir "${DEPLOY_LOCK_DIR}" 2>/dev/null; then
    _lock_pid=""
    [ -f "${DEPLOY_LOCK_DIR}/pid" ] && _lock_pid="$(cat "${DEPLOY_LOCK_DIR}/pid" 2>/dev/null)"
    if [ -n "${_lock_pid}" ] && kill -0 "${_lock_pid}" 2>/dev/null; then
        pkg_fail "A deploy to ${HOST} is already in progress (pid ${_lock_pid}, lock: ${DEPLOY_LOCK_DIR}). Wait for it to finish before starting another."
        exit 1
    fi
    # Owning process is gone — stale lock left by a crash/kill. Reclaim it.
    rm -rf "${DEPLOY_LOCK_DIR}"
    mkdir "${DEPLOY_LOCK_DIR}" || { pkg_fail "Could not acquire deploy lock for ${HOST}"; exit 1; }
fi
echo $$ > "${DEPLOY_LOCK_DIR}/pid"
trap 'rm -rf "${DEPLOY_LOCK_DIR}"' EXIT

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

# ── Step 0: Remote diagnostics (light by default) ──
if [[ "${VEYRON_DEPLOY_DIAGNOSTICS:-}" == "1" || "${VEYRON_DEPLOY_DIAGNOSTICS:-}" == "true" ]]; then
    deploy_phase_start "🩺 [0/7] Remote system and cluster diagnostics"
    stream_remote "sysinfo" "set -o pipefail; uname -a; echo ''; cat /etc/os-release 2>/dev/null | sed -n '1,6p'; echo ''; echo \"CPU: \$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo unknown) cores\"; echo \"Memory:\"; free -h 2>/dev/null || vm_stat 2>/dev/null || true; echo ''; echo 'Disk:'; df -h / 2>/dev/null || true"
    stream_remote "k8s-info" "set -o pipefail; printf '%s\n' \"Cluster flavor: ${CLUSTER_FLAVOR}\" \"Runtime import path: ${K8S_RUNTIME}\" \"kubectl command: ${K8S_CMD}\"; if command -v kubectl >/dev/null 2>&1 || [ \"${CLUSTER_FLAVOR}\" = \"k3s\" ] || [ \"${CLUSTER_FLAVOR}\" = \"rke2\" ]; then printf '%s\n' ''; ${K8S_CMD} version --short 2>/dev/null || ${K8S_CMD} version 2>/dev/null || true; printf '%s\n' '' 'Nodes:'; ${K8S_CMD} get nodes -o wide 2>/dev/null || true; printf '%s\n' '' 'All namespaces:'; ${K8S_CMD} get ns 2>/dev/null || true; printf '%s\n' '' 'All pods cluster-wide:'; ${K8S_CMD} get pods -A -o wide 2>/dev/null || true; else printf '%s\n' 'kubectl not found on remote host'; fi"
    deploy_phase_end
else
    deploy_skip_phase "⏭️  [0/7] Skipping full diagnostics (set VEYRON_DEPLOY_DIAGNOSTICS=1 for cluster-wide pod listing)"
fi

# ── Prereqs: Ensure remote build tools ──
deploy_phase_start "🔧 Ensuring remote build prerequisites"
deploy_ssh "${REMOTE}" bash -s <<'REMOTE_PREREQS'
set -euo pipefail
if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update -qq 2>/dev/null || true
  sudo apt-get install -y -qq build-essential pkg-config libssl-dev curl wget git ca-certificates 2>/dev/null || true
elif command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y gcc make pkgconfig openssl-devel curl wget git ca-certificates 2>/dev/null || true
fi
_cur=0
if command -v node >/dev/null 2>&1; then
  _ver="$(node -v 2>/dev/null | sed -E 's/^v([0-9]+).*/\1/')"
  [ -n "${_ver}" ] && _cur="${_ver}"
fi
if [ "${_cur}" -lt 18 ]; then
  if command -v apt-get >/dev/null 2>&1; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - 2>/dev/null || true
    sudo apt-get install -y -qq nodejs 2>/dev/null || true
  elif command -v dnf >/dev/null 2>&1; then
    curl -fsSL https://rpm.nodesource.com/setup_22.x | sudo bash - 2>/dev/null || true
    sudo dnf install -y nodejs 2>/dev/null || true
  fi
fi
if ! command -v podman >/dev/null 2>&1 && ! command -v docker >/dev/null 2>&1 && ! command -v nerdctl >/dev/null 2>&1; then
  echo "  [prereq] installing podman…"
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get install -y -qq podman 2>/dev/null || true
  elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y podman 2>/dev/null || true
  elif command -v yum >/dev/null 2>&1; then
    sudo yum install -y podman 2>/dev/null || true
  fi
fi
echo "  [prereq] node $(node -v 2>/dev/null || echo NOT_FOUND)"
source "${HOME}/.cargo/env" 2>/dev/null || true
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "${HOME}/.cargo/env"
fi
echo "  [prereq] $(cargo --version)"
REMOTE_PREREQS
deploy_phase_end

# Re-detect build tool in case podman was just installed by the prereqs step
CTR_BUILD=$(deploy_ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; elif command -v nerdctl >/dev/null 2>&1; then echo nerdctl; else echo none; fi')
if [ "$CTR_BUILD" = "none" ] && ! $QUICK; then
    deploy_build_fail "${HOST}"
fi

# ── Step 1: Rsync source ──
deploy_phase_start "📦 [1/7] Syncing source to ${HOST}:${DEPLOY_DIR}"
# GuestKit build context: guestkit/Cargo.toml + src/ are gitignored and populated
# from the sibling ../guestkit by prepare-guestkit-docker.sh. Invoke it via `bash`
# (NOT gated on the exec bit — a missing +x silently skipped prep and shipped an
# empty guestkit/, failing the build with an opaque "could not find Cargo.toml in
# /build/guestkit").
if [[ "${VEYRON_SKIP_GUESTKIT_PREP:-}" != "1" && "${VEYRON_SKIP_GUESTKIT_PREP:-}" != "true" \
      && -f "${REPO_DIR}/scripts/prepare-guestkit-docker.sh" ]]; then
    deploy_substep "Preparing GuestKit build context (guestkit/ → Veyron)"
    bash "${REPO_DIR}/scripts/prepare-guestkit-docker.sh" \
        || pkg_warn "GuestKit prep step failed (will use existing guestkit/ if already populated)"
fi
# Fail fast if the GuestKit build context is still incomplete — the remote image
# build would otherwise fail deep in the Dockerfile with an opaque cargo error.
if [[ ! -f "${REPO_DIR}/guestkit/Cargo.toml" ]]; then
    pkg_fail "guestkit/Cargo.toml missing — GuestKit build context is incomplete. Place the sibling repo at ../guestkit (or set GUESTKIT_SRC), then re-run; scripts/prepare-guestkit-docker.sh populates guestkit/. Use VEYRON_SKIP_GUESTKIT_PREP=1 only when guestkit/ is already populated."
    exit 1
fi
deploy_ssh "${REMOTE}" "mkdir -p ${DEPLOY_DIR}"
rsync -az --delete \
    --exclude='target/' --exclude='.git' --exclude='operator/bin/' \
    --exclude='node_modules/' --exclude='e2e/' --exclude='examples/' \
    --exclude='docs/' --exclude='.cursor/' --exclude='charts/' \
    --exclude='guestkit/target/' --exclude='*.plan.md' \
    --exclude='*.qcow2' --exclude='*.vmdk' --exclude='*.iso' \
    -e "$(deploy_rsync_ssh)" \
    "$REPO_DIR/" "${REMOTE}:${DEPLOY_DIR}/" 2>&1 | stream_local_rsync
deploy_phase_end

# ── Step 2: Build images ──
echo ""
if $QUICK; then
    deploy_skip_phase "⏭️  [2/7] Skipping image builds (--quick)"
else
    deploy_phase_start "🐳 [2/7] Building container images on remote (parallel, layer cache enabled)"

    CACHE_FROM_ARGS=""
    if [[ "${VEYRON_DEPLOY_NO_CACHE:-}" != "1" && "${VEYRON_DEPLOY_NO_CACHE:-}" != "true" ]]; then
        # podman --cache-from requires a repository without tag/digest
        VM_CACHE_REF="${VEYRON_IMAGE%%:*}"
        OP_CACHE_REF="${OPERATOR_IMAGE%%:*}"
        if [ "${CTR_BUILD}" = "docker" ]; then
            VM_CACHE_REF="${VEYRON_IMAGE}"
            OP_CACHE_REF="${OPERATOR_IMAGE}"
        fi
        if deploy_ssh "${REMOTE}" "${CTR_BUILD} image exists ${VEYRON_IMAGE} >/dev/null 2>&1"; then
            CACHE_FROM_ARGS="--cache-from ${VM_CACHE_REF}"
        fi
        if deploy_ssh "${REMOTE}" "${CTR_BUILD} image exists ${OPERATOR_IMAGE} >/dev/null 2>&1"; then
            CACHE_FROM_ARGS="${CACHE_FROM_ARGS} --cache-from ${OP_CACHE_REF}"
        fi
    fi
    NO_CACHE_ARG=""
    if [[ "${VEYRON_DEPLOY_NO_CACHE:-}" == "1" || "${VEYRON_DEPLOY_NO_CACHE:-}" == "true" ]]; then
        NO_CACHE_ARG="--no-cache"
        deploy_note "VEYRON_DEPLOY_NO_CACHE=1 — full rebuild (slow)"
    fi

    EXPORT_LINE=""
    PROGRESS_ARG=""
    if [ "${CTR_BUILD}" = "docker" ]; then
        EXPORT_LINE="export DOCKER_BUILDKIT=1"
        if [[ "${VEYRON_DEPLOY_VERBOSE:-}" == "1" ]]; then
            PROGRESS_ARG="--progress=plain"
        fi
    elif [ "${CTR_BUILD}" = "podman" ]; then
        EXPORT_LINE="export BUILDAH_FORMAT=docker"
    fi

    PARALLEL_BUILD=$(cat <<REMOTE_BUILD_EOF
set -euo pipefail
${EXPORT_LINE}
API_CMD='${CTR_BUILD} build ${NO_CACHE_ARG} ${CACHE_FROM_ARGS} ${PROGRESS_ARG} -t ${VEYRON_IMAGE} .'
OP_CMD='${CTR_BUILD} build ${NO_CACHE_ARG} ${CACHE_FROM_ARGS} ${PROGRESS_ARG} -t ${OPERATOR_IMAGE} .'
( cd '${DEPLOY_DIR}' && eval "\$API_CMD" ) &
api_pid=\$!
( cd '${DEPLOY_DIR}/operator' && eval "\$OP_CMD" ) &
op_pid=\$!
wait \$api_pid
wait \$op_pid
REMOTE_BUILD_EOF
)

    deploy_substep "📦 Veyron API + Operator image builds (parallel)"
    stream_remote "image-build" "${PARALLEL_BUILD}"
    pkg_ok "Container image builds complete"

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
                ${CTR_BUILD} save ${VEYRON_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${K3S_BIN} ctr images import -
            " 2>&1
            ;;
        rke2)
            RKE2_BIN=$(deploy_ssh "${REMOTE}" "which rke2 2>/dev/null || echo /usr/local/bin/rke2")
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VEYRON_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ${RKE2_BIN} ctr images import -
            " 2>&1
            ;;
        microk8s)
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VEYRON_IMAGE} > /tmp/veyron.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/veyron-operator.tar
                microk8s ctr image import /tmp/veyron.tar
                microk8s ctr image import /tmp/veyron-operator.tar
                rm -f /tmp/veyron.tar /tmp/veyron-operator.tar
            " 2>&1
            ;;
        kind)
            deploy_ssh "${REMOTE}" "
                kind load docker-image ${VEYRON_IMAGE}
                kind load docker-image ${OPERATOR_IMAGE}
            " 2>&1
            ;;
        minikube)
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VEYRON_IMAGE} > /tmp/veyron.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/veyron-operator.tar
                minikube image load /tmp/veyron.tar
                minikube image load /tmp/veyron-operator.tar
                rm -f /tmp/veyron.tar /tmp/veyron-operator.tar
            " 2>&1
            ;;
        containerd)
            # Standard kubeadm K8s with containerd
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VEYRON_IMAGE} | ${SUDO} ctr -n k8s.io images import -
                ${CTR_BUILD} save ${OPERATOR_IMAGE} | ${SUDO} ctr -n k8s.io images import -
            " 2>&1
            ;;
        cri)
            # CRI-O or other CRI runtime — import via tar
            deploy_ssh "${REMOTE}" "
                ${CTR_BUILD} save ${VEYRON_IMAGE} > /tmp/veyron.tar
                ${CTR_BUILD} save ${OPERATOR_IMAGE} > /tmp/veyron-operator.tar
                ${SUDO} crictl pull /tmp/veyron.tar 2>/dev/null || \
                    ${SUDO} ctr -n k8s.io images import /tmp/veyron.tar
                ${SUDO} crictl pull /tmp/veyron-operator.tar 2>/dev/null || \
                    ${SUDO} ctr -n k8s.io images import /tmp/veyron-operator.tar
                rm -f /tmp/veyron.tar /tmp/veyron-operator.tar
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
  pkg_fail "CDI ensure failed (set VEYRON_SKIP_CDI=1 to skip on air-gapped clusters)"
  exit 1
}
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} create namespace ${NAMESPACE} --dry-run=client -o yaml | ${K8S_CMD} apply -f -
    if [ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" ] && [ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ] && ${K8S_CMD} get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
        for cilium_yaml in ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-veyron-egress.yaml ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-veyron-egress.yaml; do
            if [ -f \"\${cilium_yaml}\" ]; then
                echo 'Applying Cilium egress policy for ${NAMESPACE} (veyron / nats workloads)...'
                sed -e \"s|__VEYRON_APP_NAMESPACE__|${NAMESPACE}|g\" \
                    -e \"s|namespace: veyron-system|namespace: ${NAMESPACE}|g\" \
                    -e \"s|namespace: veyron-system|namespace: ${NAMESPACE}|g\" \
                    \"\${cilium_yaml}\" | ${K8S_CMD} apply -f -
                break
            fi
        done
    else
        echo 'Skipping Cilium veyron egress bootstrap (VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1, no Cilium CRD, or yaml missing)'
    fi
    if [ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" ] && [ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ] && ${K8S_CMD} get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null && [ -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml ]; then
        echo 'Applying Cilium clusterwide egress for KubeVirt virt-launcher (VM guest internet)...'
        ${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml
    fi
    if [ -d ${DEPLOY_DIR}/operator/config/crd/bases ]; then
        find ${DEPLOY_DIR}/operator/config/crd/bases -maxdepth 1 -name '*.yaml' -print | sort | while IFS= read -r f; do
            ${K8S_CMD} apply -f \"\$f\"
        done
    else
        echo \"WARN: missing ${DEPLOY_DIR}/operator/config/crd/bases — skipping CRD apply\"
    fi
    if [ -d ${DEPLOY_DIR}/operator/config/catalog/templates ]; then
        echo 'Applying VMTemplate catalog manifests...'
        find ${DEPLOY_DIR}/operator/config/catalog/templates -maxdepth 1 -name '*.yaml' -print | sort | while IFS= read -r f; do
            ${K8S_CMD} apply -f \"\$f\"
        done
    fi
    if [ -d ${DEPLOY_DIR}/operator/config/catalog/profiles ]; then
        echo 'Applying VMProfile catalog manifests...'
        find ${DEPLOY_DIR}/operator/config/catalog/profiles -maxdepth 1 -name '*.yaml' -print | sort | while IFS= read -r f; do
            ${K8S_CMD} apply -f \"\$f\"
        done
    fi
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/service_account.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/rbac/role.yaml
    ${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/nats/nats-deployment.yaml
" 2>&1
deploy_phase_end

# ── Step 5: Deploy Veyron API + Operator ──
echo ""
deploy_phase_start "🚀 [5/7] Deploying Veyron API + Operator"

# Tear down existing workloads so the next apply creates fresh ReplicaSets/pods.
# --wait ensures objects are gone before apply (avoid races with stale pods).
deploy_ssh "${REMOTE}" "
    for ns in ${NAMESPACE} veyron-system; do
        ${K8S_CMD} -n \"\${ns}\" delete deployment veyron-api veyron-operator veyron-api veyron-operator \
            --ignore-not-found --wait=true --timeout=180s 2>/dev/null || true
        ${K8S_CMD} -n \"\${ns}\" delete svc veyron-api veyron-api \
            --ignore-not-found --wait=true --timeout=60s 2>/dev/null || true
        ${K8S_CMD} -n \"\${ns}\" delete secret veyron-api-key veyron-api-key \
            --ignore-not-found --wait=false 2>/dev/null || true
    done
" 2>&1

# Create API key + JWT + bootstrap admin password (dashboard login: admin / CHANGE_ME)
JWT_SECRET="${VEYRON_JWT_SECRET:-veyron-dev-jwt-secret-change-me}"
JWT_ISSUER="${VEYRON_JWT_ISSUER:-veyron}"
ADMIN_PASSWORD="${VEYRON_BOOTSTRAP_ADMIN_PASSWORD:-${API_KEY}}"
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} create secret generic veyron-api-key \
        --from-literal=api-key='${API_KEY}' \
        --from-literal=jwt-secret='${JWT_SECRET}' \
        --from-literal=jwt-issuer='${JWT_ISSUER}' \
        --from-literal=admin-password='${ADMIN_PASSWORD}'
" 2>&1

# Enterprise SSO (Keycloak/Auth0/Okta) — only touched when --with-oidc is passed,
# so a plain redeploy never disturbs an OIDC config set up by an earlier run.
# Values go through `create secret --from-literal` (not `--set` on a chart) since
# a client secret can contain characters a templating engine's --set mishandles.
# See contrib/veyron-oidc-keycloak.env.example.
if [ "${DEPLOY_WITH_OIDC}" = true ]; then
    if [ -z "${VEYRON_OIDC_ISSUER:-}" ] || [ -z "${VEYRON_OIDC_CLIENT_ID:-}" ]; then
        pkg_fail "--with-oidc requires VEYRON_OIDC_ISSUER and VEYRON_OIDC_CLIENT_ID in the environment"
        exit 1
    fi
    deploy_substep "🔐 Wiring OIDC SSO Secret (veyron-oidc)…"
    deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} delete secret veyron-oidc --ignore-not-found --wait=false" 2>&1
    OIDC_LITERALS="--from-literal=VEYRON_OIDC_ISSUER='${VEYRON_OIDC_ISSUER}' --from-literal=VEYRON_OIDC_CLIENT_ID='${VEYRON_OIDC_CLIENT_ID}'"
    [ -n "${VEYRON_OIDC_CLIENT_SECRET:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_CLIENT_SECRET='${VEYRON_OIDC_CLIENT_SECRET}'"
    [ -n "${VEYRON_OIDC_AUTHORIZATION_URL:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_AUTHORIZATION_URL='${VEYRON_OIDC_AUTHORIZATION_URL}'"
    [ -n "${VEYRON_OIDC_TOKEN_URL:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_TOKEN_URL='${VEYRON_OIDC_TOKEN_URL}'"
    [ -n "${VEYRON_OIDC_JWKS_URL:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_JWKS_URL='${VEYRON_OIDC_JWKS_URL}'"
    [ -n "${VEYRON_OIDC_REDIRECT_URI:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_REDIRECT_URI='${VEYRON_OIDC_REDIRECT_URI}'"
    [ -n "${VEYRON_OIDC_ROLE_CLAIM:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_ROLE_CLAIM='${VEYRON_OIDC_ROLE_CLAIM}'"
    [ -n "${VEYRON_OIDC_GROUP_ADMIN:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_GROUP_ADMIN='${VEYRON_OIDC_GROUP_ADMIN}'"
    [ -n "${VEYRON_OIDC_GROUP_WRITE:-}" ] && OIDC_LITERALS="${OIDC_LITERALS} --from-literal=VEYRON_OIDC_GROUP_WRITE='${VEYRON_OIDC_GROUP_WRITE}'"
    deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} create secret generic veyron-oidc ${OIDC_LITERALS}" 2>&1
fi

# Apply Veyron API deployment
deploy_ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/deploy/k8s.yaml" 2>&1

# Optional: Prometheus / Alertmanager / Loki URLs when those namespaces exist
if [[ -f "${REPO_DIR}/scripts/lib/bootstrap-integrations.sh" ]]; then
    deploy_substep "🔗 Wiring veyron-integrations Secret (if monitoring/loki present)…"
    deploy_ssh "${REMOTE}" "bash ${DEPLOY_DIR}/scripts/lib/bootstrap-integrations.sh '${K8S_CMD}' '${NAMESPACE}'" 2>&1 || \
        pkg_warn "integrations bootstrap skipped or failed (non-fatal)"
fi

# Apply Operator deployment
deploy_ssh "${REMOTE}" "${K8S_CMD} apply -f ${DEPLOY_DIR}/operator/config/manager/manager.yaml" 2>&1

# Same image tags + imagePullPolicy: Never: replacing the Deployment can still leave
# kubelet using a cached layer unless pods are recreated after image import.
deploy_substep "🔄 Recycling API + operator pods for new images…"
deploy_ssh "${REMOTE}" "
    ${K8S_CMD} -n ${NAMESPACE} rollout restart deployment/veyron-api deployment/veyron-operator 2>/dev/null || true
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/veyron-api --timeout=120s
    ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/veyron-operator --timeout=120s
    if ${K8S_CMD} -n ${NAMESPACE} get secret veyron-integrations &>/dev/null; then
        ${K8S_CMD} -n ${NAMESPACE} rollout restart deployment/veyron-api 2>/dev/null || true
        ${K8S_CMD} -n ${NAMESPACE} rollout status deployment/veyron-api --timeout=120s
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
NODE_PORT=$(deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc veyron-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || echo "30151")
HTTP_REDIRECT_PORT=$(deploy_ssh "${REMOTE}" "${K8S_CMD} -n ${NAMESPACE} get svc veyron-api -o jsonpath='{.spec.ports[?(@.name==\"http-redirect\")].nodePort}' 2>/dev/null" || echo "")

deploy_complete "${HOST}" "${USER}" "${REMOTE}" "${NODE_PORT}" "${HTTP_REDIRECT_PORT}" \
    "${API_KEY}" "${SCRIPT_DIR}" "${K8S_CMD}" "${NAMESPACE}"
