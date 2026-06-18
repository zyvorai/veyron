#!/usr/bin/env bash
# Veyron Kubernetes Deploy Script
# Auto-detects container runtime (docker/podman) and K8s distribution (k8s/k3s/kind/minikube)
# Builds container image and deploys to the current K8s cluster
#
# Usage: ./scripts/deploy-k8s.sh [command]
#   build    Build container image locally
#   push     Push image to registry
#   load     Load image directly into cluster (k3s/kind/minikube)
#   deploy   Apply K8s manifests
#   all      Build + deploy (default)
#   status   Show deployment status
#   logs     Tail pod logs
#   delete   Remove everything
#   detect   Show detected environment

set -euo pipefail

REPO="${VEYRON_REGISTRY:-ghcr.io/ssahani/veyron}"
VERSION="${VERSION:-$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')}"
IMAGE="${REPO}:${VERSION}"
IMAGE_LATEST="${REPO}:latest"
NAMESPACE="${VEYRON_NAMESPACE:-veyron-system}"
MANIFEST="deploy/k8s.yaml"
API_KEY="Admin@321"

# ── Auto-detect container runtime ──
detect_container_runtime() {
    if [ -n "${CONTAINER_RUNTIME:-}" ]; then
        # User override
        if command -v "${CONTAINER_RUNTIME}" &>/dev/null; then
            RUNTIME="${CONTAINER_RUNTIME}"
            return
        fi
        echo "⚠️  CONTAINER_RUNTIME=${CONTAINER_RUNTIME} not found, auto-detecting..."
    fi

    if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
        RUNTIME="docker"
    elif command -v podman &>/dev/null; then
        RUNTIME="podman"
    elif command -v nerdctl &>/dev/null; then
        RUNTIME="nerdctl"
    else
        echo "❌ No container runtime found (tried: docker, podman, nerdctl)"
        echo "   Install one or set CONTAINER_RUNTIME= to override"
        exit 1
    fi
}

# ── Auto-detect Kubernetes distribution ──
detect_k8s_distro() {
    if [ -n "${K8S_DISTRO:-}" ]; then
        # User override
        DISTRO="${K8S_DISTRO}"
        return
    fi

    # Detect kubectl binary
    if [ -n "${KUBECTL:-}" ]; then
        : # user override
    elif command -v kubectl &>/dev/null; then
        KUBECTL="kubectl"
    elif command -v k3s &>/dev/null; then
        KUBECTL="k3s kubectl"
    elif command -v microk8s &>/dev/null; then
        KUBECTL="microk8s kubectl"
    else
        echo "❌ No kubectl found (tried: kubectl, k3s, microk8s)"
        echo "   Install kubectl or set KUBECTL= to override"
        exit 1
    fi

    # Detect cluster distribution from node info
    if ! ${KUBECTL} cluster-info &>/dev/null; then
        DISTRO="unknown"
        return
    fi

    local node_info
    node_info=$(${KUBECTL} get nodes -o jsonpath='{.items[0].status.nodeInfo.kubeletVersion}' 2>/dev/null || echo "")

    if echo "${node_info}" | grep -qi "k3s"; then
        DISTRO="k3s"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null | grep -q "minikube"; then
        DISTRO="minikube"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null | grep -q "microk8s"; then
        DISTRO="microk8s"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].spec.providerID}' 2>/dev/null | grep -q "kind://"; then
        DISTRO="kind"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null | grep -q "eks.amazonaws.com"; then
        DISTRO="eks"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null | grep -q "cloud.google.com/gke"; then
        DISTRO="gke"
    elif ${KUBECTL} get nodes -o jsonpath='{.items[0].spec.providerID}' 2>/dev/null | grep -q "azure://"; then
        DISTRO="aks"
    else
        DISTRO="k8s"
    fi
}

# ── Run detection ──
detect_container_runtime
detect_k8s_distro

# ── Show detected environment ──
cmd_detect() {
    echo "Environment Detection"
    echo ""
    echo "  Container runtime: ${RUNTIME}"
    echo "    $(${RUNTIME} --version 2>/dev/null || echo 'version unknown')"
    echo "  Kubernetes CLI:    ${KUBECTL}"
    echo "  K8s distribution:  ${DISTRO}"
    if ${KUBECTL} cluster-info &>/dev/null; then
        echo "  Cluster:           $(${KUBECTL} cluster-info 2>/dev/null | head -1)"
        echo "  Nodes:             $(${KUBECTL} get nodes --no-headers 2>/dev/null | wc -l)"
        local server_version
        server_version=$(${KUBECTL} version -o json 2>/dev/null | grep -o '"gitVersion":"[^"]*"' | head -1 || echo "")
        echo "  Server version:    ${server_version:-unknown}"
    else
        echo "  Cluster:           NOT CONNECTED"
    fi
    echo ""

    # Show capabilities
    echo "  Capabilities:"
    case "${DISTRO}" in
        k3s|kind|minikube|microk8s)
            echo "    - Direct image loading supported (skip registry push)"
            ;;
        *)
            echo "    - Registry push required for deployment"
            ;;
    esac
    if [ "${RUNTIME}" = "podman" ]; then
        echo "    - Podman: using --format=docker for k8s compatibility"
    fi
}

# ── Build container image ──
cmd_build() {
    echo "Building container image: ${IMAGE}"
    echo "  Runtime: ${RUNTIME}"

    local build_args=()
    if [ "${RUNTIME}" = "podman" ]; then
        # Podman: use docker format for k8s compatibility
        build_args+=(--format docker)
    fi

    ${RUNTIME} build "${build_args[@]}" -t "${IMAGE}" -t "${IMAGE_LATEST}" .
    echo "Image built: ${IMAGE}"

    local size_info
    size_info=$(${RUNTIME} image inspect "${IMAGE}" --format='{{.Size}}' 2>/dev/null || echo "")
    if [ -n "${size_info}" ]; then
        echo "  Size: $(echo "${size_info}" | numfmt --to=iec 2>/dev/null || echo "${size_info}")"
    fi
}

# ── Push to registry ──
cmd_push() {
    echo "Pushing ${IMAGE}..."
    ${RUNTIME} push "${IMAGE}"
    ${RUNTIME} push "${IMAGE_LATEST}"
    echo "Pushed ${IMAGE}"
}

# ── Load image directly into cluster (no registry needed) ──
cmd_load() {
    echo "Loading image into ${DISTRO} cluster..."

    case "${DISTRO}" in
        k3s)
            local image_tar
            image_tar=$(mktemp /tmp/veyron-image-XXXXXX.tar)
            trap "rm -f '${image_tar}'" EXIT
            ${RUNTIME} save "${IMAGE}" -o "${image_tar}"
            if command -v k3s &>/dev/null; then
                sudo k3s ctr images import "${image_tar}"
            else
                # Remote k3s or crictl-based import
                sudo ctr --namespace k8s.io images import "${image_tar}"
            fi
            echo "Image loaded into k3s"
            ;;
        kind)
            if [ "${RUNTIME}" = "podman" ]; then
                local image_tar
                image_tar=$(mktemp /tmp/veyron-image-XXXXXX.tar)
                trap "rm -f '${image_tar}'" EXIT
                ${RUNTIME} save "${IMAGE}" -o "${image_tar}"
                kind load image-archive "${image_tar}"
            else
                kind load docker-image "${IMAGE}"
            fi
            echo "Image loaded into kind"
            ;;
        minikube)
            if [ "${RUNTIME}" = "podman" ]; then
                local image_tar
                image_tar=$(mktemp /tmp/veyron-image-XXXXXX.tar)
                trap "rm -f '${image_tar}'" EXIT
                ${RUNTIME} save "${IMAGE}" -o "${image_tar}"
                minikube image load "${image_tar}"
            else
                minikube image load "${IMAGE}"
            fi
            echo "Image loaded into minikube"
            ;;
        microk8s)
            local image_tar
            image_tar=$(mktemp /tmp/veyron-image-XXXXXX.tar)
            trap "rm -f '${image_tar}'" EXIT
            ${RUNTIME} save "${IMAGE}" -o "${image_tar}"
            microk8s ctr image import "${image_tar}"
            echo "Image loaded into microk8s"
            ;;
        *)
            echo "Direct image loading not supported for ${DISTRO}"
            echo "Use 'push' to push to a registry instead"
            exit 1
            ;;
    esac
}

# ── Deploy to K8s ──
cmd_deploy() {
    echo "Deploying to ${DISTRO} cluster..."
    echo "  kubectl: ${KUBECTL}"

    if ! ${KUBECTL} cluster-info &>/dev/null; then
        echo "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Create namespace
    ${KUBECTL} create namespace "${NAMESPACE}" --dry-run=client -o yaml | ${KUBECTL} apply -f -

    # Create or update API key secret
    ${KUBECTL} -n "${NAMESPACE}" delete secret veyron-api-key --ignore-not-found 2>/dev/null
    ${KUBECTL} -n "${NAMESPACE}" create secret generic veyron-api-key \
        --from-literal=api-key="${API_KEY}"
    echo "API key secret created"

    # For local clusters, set imagePullPolicy to Never/IfNotPresent
    local pull_policy=""
    case "${DISTRO}" in
        k3s|kind|minikube|microk8s)
            pull_policy="IfNotPresent"
            ;;
    esac

    # Apply manifests
    ${KUBECTL} apply -f "${MANIFEST}"
    echo "Manifests applied"

    # Update image and pull policy
    ${KUBECTL} -n "${NAMESPACE}" set image deployment/veyron-api \
        veyron="${IMAGE}" 2>/dev/null || true

    if [ -n "${pull_policy}" ]; then
        ${KUBECTL} -n "${NAMESPACE}" patch deployment veyron-api \
            -p "{\"spec\":{\"template\":{\"spec\":{\"containers\":[{\"name\":\"veyron\",\"imagePullPolicy\":\"${pull_policy}\"}]}}}}" \
            2>/dev/null || true
    fi

    # Wait for rollout
    echo "Waiting for rollout..."
    ${KUBECTL} -n "${NAMESPACE}" rollout status deployment/veyron-api --timeout=120s

    echo "Deployed successfully"
    echo ""
    cmd_status
}

# ── Show status ──
cmd_status() {
    echo "Deployment Status (${DISTRO})"
    echo ""
    ${KUBECTL} -n "${NAMESPACE}" get deployment veyron-api -o wide 2>/dev/null || echo "Deployment not found"
    echo ""
    ${KUBECTL} -n "${NAMESPACE}" get pods -l app.kubernetes.io/name=veyron -o wide 2>/dev/null
    echo ""
    ${KUBECTL} -n "${NAMESPACE}" get svc veyron-api 2>/dev/null || true
    echo ""

    # Show access info
    local node_ip pod_port
    node_ip=$(${KUBECTL} get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}' 2>/dev/null)
    pod_port=$(${KUBECTL} -n "${NAMESPACE}" get svc veyron-api -o jsonpath='{.spec.ports[?(@.name=="https")].nodePort}' 2>/dev/null || echo "")

    if [ -n "${pod_port}" ]; then
        echo "  Dashboard: https://${node_ip}:${pod_port}/dashboard"
    else
        echo "  Port-forward: ${KUBECTL} -n ${NAMESPACE} port-forward svc/veyron-api 443:443"
        echo "  Dashboard:    https://localhost:443/dashboard"
    fi
    echo "  API Key:    ${API_KEY}"
}

# ── Tail logs ──
cmd_logs() {
    ${KUBECTL} -n "${NAMESPACE}" logs -l app.kubernetes.io/name=veyron -f --tail=50
}

# ── Delete everything ──
cmd_delete() {
    echo "Removing Veyron from ${NAMESPACE}..."
    ${KUBECTL} delete -f "${MANIFEST}" --ignore-not-found
    ${KUBECTL} -n "${NAMESPACE}" delete secret veyron-api-key --ignore-not-found
    echo "Removed"
}

# ── All (build + load/push + deploy) ──
cmd_all() {
    cmd_build

    # For local clusters, load directly; otherwise push to registry
    case "${DISTRO}" in
        k3s|kind|minikube|microk8s)
            echo ""
            echo "Local cluster detected (${DISTRO}), loading image directly..."
            cmd_load
            ;;
        *)
            echo ""
            echo "Remote/standard cluster (${DISTRO}), pushing to registry..."
            cmd_push
            ;;
    esac

    cmd_deploy
}

# ── Help ──
cmd_help() {
    echo "Veyron Kubernetes Deploy"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  build     Build container image"
    echo "  push      Push image to registry"
    echo "  load      Load image directly into cluster (k3s/kind/minikube/microk8s)"
    echo "  deploy    Apply K8s manifests and deploy"
    echo "  all       Build + load/push + deploy (default)"
    echo "  status    Show deployment status"
    echo "  logs      Tail pod logs"
    echo "  delete    Remove everything"
    echo "  detect    Show detected environment"
    echo ""
    echo "Environment:"
    echo "  VERSION            Override version (default: from Cargo.toml)"
    echo "  VEYRON_REGISTRY   Override registry (default: ghcr.io/ssahani/veyron)"
    echo "  CONTAINER_RUNTIME  Override runtime (default: auto-detect docker/podman/nerdctl)"
    echo "  K8S_DISTRO         Override distro  (default: auto-detect k8s/k3s/kind/minikube)"
    echo "  KUBECTL            Override kubectl binary (default: auto-detect)"
    echo ""
    echo "Detected:"
    echo "  Runtime: ${RUNTIME} | Distro: ${DISTRO} | kubectl: ${KUBECTL}"
}

# ── Main ──
echo ""
echo "  Veyron K8s Deploy"
echo "  Image:   ${IMAGE}"
echo "  Runtime: ${RUNTIME} | Cluster: ${DISTRO}"
echo ""

case "${1:-all}" in
    build)  cmd_build  ;;
    push)   cmd_push   ;;
    load)   cmd_load   ;;
    deploy) cmd_deploy ;;
    all)    cmd_all    ;;
    status) cmd_status ;;
    logs)   cmd_logs   ;;
    delete) cmd_delete ;;
    detect) cmd_detect ;;
    help|*) cmd_help   ;;
esac
