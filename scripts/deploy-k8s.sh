#!/usr/bin/env bash
# VMRogue Kubernetes Deploy Script
# Builds container image and deploys to the current K8s cluster
#
# Usage: ./scripts/deploy-k8s.sh [command]
#   build    Build container image locally
#   push     Push image to registry
#   deploy   Apply K8s manifests
#   all      Build + deploy (default)
#   status   Show deployment status
#   logs     Tail pod logs
#   delete   Remove everything

set -euo pipefail

REPO="${VMROGUE_REGISTRY:-ghcr.io/ssahani/vmrogue}"
VERSION="${VERSION:-$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')}"
IMAGE="${REPO}:${VERSION}"
IMAGE_LATEST="${REPO}:latest"
NAMESPACE="vmrogue-system"
MANIFEST="deploy/k8s.yaml"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

log()  { echo -e "${CYAN}[vmrogue]${NC} $*"; }
ok()   { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[x]${NC} $*" >&2; }

header() {
    echo -e "${BOLD}${CYAN}"
    echo "  ╦  ╦╔╦╗╦═╗╔═╗╔═╗╦ ╦╔═╗"
    echo "  ╚╗╔╝║║║╠╦╝║ ║║ ╦║ ║║╣ "
    echo "   ╚╝ ╩ ╩╩╚═╚═╝╚═╝╚═╝╚═╝"
    echo -e "${NC}${BOLD}  K8s Deploy — ${IMAGE}${NC}"
    echo ""
}

# ── Build container image ──
cmd_build() {
    log "Building container image: ${IMAGE}"
    docker build -t "${IMAGE}" -t "${IMAGE_LATEST}" .
    ok "Image built: ${IMAGE}"
    docker image inspect "${IMAGE}" --format='Size: {{.Size}}' 2>/dev/null | numfmt --to=iec --field=2 || true
}

# ── Push to registry ──
cmd_push() {
    log "Pushing ${IMAGE}..."
    docker push "${IMAGE}"
    docker push "${IMAGE_LATEST}"
    ok "Pushed ${IMAGE}"
}

# ── Deploy to K8s ──
cmd_deploy() {
    log "Deploying to Kubernetes..."

    if ! kubectl cluster-info &>/dev/null; then
        err "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Create namespace
    kubectl create namespace "${NAMESPACE}" --dry-run=client -o yaml | kubectl apply -f -

    # Create API key secret if it doesn't exist
    if ! kubectl -n "${NAMESPACE}" get secret vmrogue-api-key &>/dev/null; then
        local api_key
        api_key=$(openssl rand -hex 32)
        kubectl -n "${NAMESPACE}" create secret generic vmrogue-api-key \
            --from-literal=api-key="${api_key}"
        ok "API key secret created"
        echo -e "  ${CYAN}API Key:${NC} ${api_key}"
        echo "  Save this key — you'll need it to access the dashboard."
    else
        ok "API key secret already exists"
    fi

    # Apply manifests
    kubectl apply -f "${MANIFEST}"
    ok "Manifests applied"

    # Update image if not using :latest
    kubectl -n "${NAMESPACE}" set image deployment/vmrogue-api \
        vmrogue="${IMAGE}" 2>/dev/null || true

    # Wait for rollout
    log "Waiting for rollout..."
    kubectl -n "${NAMESPACE}" rollout status deployment/vmrogue-api --timeout=120s

    ok "Deployed successfully"
    echo ""
    cmd_status
}

# ── Show status ──
cmd_status() {
    echo -e "${BOLD}Deployment Status${NC}"
    echo ""
    kubectl -n "${NAMESPACE}" get deployment vmrogue-api -o wide 2>/dev/null || warn "Deployment not found"
    echo ""
    kubectl -n "${NAMESPACE}" get pods -l app.kubernetes.io/name=vmrogue -o wide 2>/dev/null
    echo ""
    kubectl -n "${NAMESPACE}" get svc vmrogue-api 2>/dev/null || true
    echo ""

    # Show access info
    local node_ip pod_port
    node_ip=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}' 2>/dev/null)
    pod_port=$(kubectl -n "${NAMESPACE}" get svc vmrogue-api -o jsonpath='{.spec.ports[0].nodePort}' 2>/dev/null || echo "")

    if [ -n "${pod_port}" ]; then
        echo -e "  ${CYAN}Dashboard:${NC} http://${node_ip}:${pod_port}/dashboard"
    else
        echo -e "  ${CYAN}Port-forward:${NC} kubectl -n ${NAMESPACE} port-forward svc/vmrogue-api 5151:5151"
        echo -e "  ${CYAN}Dashboard:${NC}    http://localhost:5151/dashboard"
    fi
}

# ── Tail logs ──
cmd_logs() {
    kubectl -n "${NAMESPACE}" logs -l app.kubernetes.io/name=vmrogue -f --tail=50
}

# ── Delete everything ──
cmd_delete() {
    log "Removing VMRogue from ${NAMESPACE}..."
    kubectl delete -f "${MANIFEST}" --ignore-not-found
    kubectl -n "${NAMESPACE}" delete secret vmrogue-api-key --ignore-not-found
    ok "Removed"
}

# ── All (build + deploy) ──
cmd_all() {
    cmd_build
    cmd_deploy
}

# ── Help ──
cmd_help() {
    echo "VMRogue Kubernetes Deploy"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  build     Build container image"
    echo "  push      Push image to registry"
    echo "  deploy    Apply K8s manifests and deploy"
    echo "  all       Build + deploy (default)"
    echo "  status    Show deployment status"
    echo "  logs      Tail pod logs"
    echo "  delete    Remove everything"
    echo ""
    echo "Environment:"
    echo "  VERSION            Override version (default: from Cargo.toml)"
    echo "  VMROGUE_REGISTRY   Override registry (default: ghcr.io/ssahani/vmrogue)"
}

# ── Main ──
header
case "${1:-all}" in
    build)  cmd_build  ;;
    push)   cmd_push   ;;
    deploy) cmd_deploy ;;
    all)    cmd_all    ;;
    status) cmd_status ;;
    logs)   cmd_logs   ;;
    delete) cmd_delete ;;
    help|*) cmd_help   ;;
esac
