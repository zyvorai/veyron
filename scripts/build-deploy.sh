#!/usr/bin/env bash
# Veyron Build, Test & Deploy Script
# Usage: ./scripts/build-deploy.sh [command]
#
# Commands:
#   test      Run full test suite (unit + integration + doc)
#   build     Build release binary
#   docker    Build Docker image
#   push      Push Docker image to registry
#   deploy    Deploy to Kubernetes cluster
#   all       Run test -> build -> docker -> push -> deploy
#   clean     Remove build artifacts and images

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

REPO="ghcr.io/zyvorai/veyron"
VERSION="${VERSION:-$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')}"
IMAGE="${REPO}:${VERSION}"
IMAGE_LATEST="${REPO}:latest"
NAMESPACE="veyron-system"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${CYAN}[veyron]${NC} $*"; }
ok()   { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[✗]${NC} $*" >&2; }

# ── Auto-detect container runtime ──
detect_container_runtime() {
    if [ -n "${CONTAINER_RUNTIME:-}" ]; then
        if command -v "${CONTAINER_RUNTIME}" &>/dev/null; then
            RUNTIME="${CONTAINER_RUNTIME}"
            return
        fi
        warn "CONTAINER_RUNTIME=${CONTAINER_RUNTIME} not found, auto-detecting..."
    fi

    if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
        RUNTIME="docker"
    elif command -v podman &>/dev/null; then
        RUNTIME="podman"
    elif command -v nerdctl &>/dev/null; then
        RUNTIME="nerdctl"
    else
        err "No container runtime found (tried: docker, podman, nerdctl)"
        err "Install one or set CONTAINER_RUNTIME= to override"
        exit 1
    fi
}

# ── Auto-detect kubectl ──
detect_kubectl() {
    if [ -n "${KUBECTL:-}" ]; then
        return
    elif command -v kubectl &>/dev/null; then
        KUBECTL="kubectl"
    elif command -v k3s &>/dev/null; then
        KUBECTL="k3s kubectl"
    elif command -v microk8s &>/dev/null; then
        KUBECTL="microk8s kubectl"
    else
        KUBECTL="kubectl"  # will fail later with a clear message
    fi
}

detect_container_runtime
detect_kubectl
log "Container runtime: ${RUNTIME} | kubectl: ${KUBECTL}"

# ── Test ─────────────────────────────────────────────
cmd_test() {
    log "Running full test suite..."

    log "  cargo fmt --check"
    cargo fmt --all -- --check
    ok "Format check passed"

    log "  cargo clippy"
    cargo clippy --all-targets -- -D warnings
    ok "Clippy passed"

    log "  cargo test (unit + integration + doc)"
    RUST_MIN_STACK=8388608 cargo test
    ok "All tests passed"
}

# ── Build ────────────────────────────────────────────
cmd_build() {
    log "Building release binary..."
    cargo build --release --locked
    strip target/release/veyron 2>/dev/null || true

    local size
    size=$(du -h target/release/veyron | cut -f1)
    ok "Binary: target/release/veyron ($size)"
}

# ── Docker/Podman ────────────────────────────────────
cmd_docker() {
    log "Building container image: ${IMAGE} (${RUNTIME})"

    local build_args=()
    if [ "${RUNTIME}" = "podman" ]; then
        build_args+=(--format docker)
    fi

    ${RUNTIME} build "${build_args[@]}" -t "${IMAGE}" -t "${IMAGE_LATEST}" .
    ok "Image built: ${IMAGE}"

    local size
    size=$(${RUNTIME} image inspect "${IMAGE}" --format='{{.Size}}' | numfmt --to=iec 2>/dev/null || echo "unknown")
    log "Image size: ${size}"
}

# ── Push ─────────────────────────────────────────────
cmd_push() {
    log "Pushing ${IMAGE}..."
    ${RUNTIME} push "${IMAGE}"
    ${RUNTIME} push "${IMAGE_LATEST}"
    ok "Pushed ${IMAGE} and ${IMAGE_LATEST}"
}

# ── Deploy ───────────────────────────────────────────
cmd_deploy() {
    log "Deploying to Kubernetes (${KUBECTL})..."

    if ! ${KUBECTL} cluster-info &>/dev/null; then
        err "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Create namespace if needed
    ${KUBECTL} create namespace "${NAMESPACE}" --dry-run=client -o yaml | ${KUBECTL} apply -f -

    # Apply manifests
    ${KUBECTL} apply -f deploy/k8s.yaml

    # Update image
    ${KUBECTL} -n "${NAMESPACE}" set image deployment/veyron-api \
        veyron="${IMAGE}" 2>/dev/null || true

    # Wait for rollout
    log "Waiting for rollout..."
    ${KUBECTL} -n "${NAMESPACE}" rollout status deployment/veyron-api --timeout=120s

    ok "Deployed ${IMAGE} to ${NAMESPACE}"
    ${KUBECTL} -n "${NAMESPACE}" get pods -l app.kubernetes.io/name=veyron
}

# ── Clean ────────────────────────────────────────────
cmd_clean() {
    log "Cleaning up..."
    cargo clean
    ${RUNTIME} rmi "${IMAGE}" "${IMAGE_LATEST}" 2>/dev/null || true
    ok "Clean complete"
}

# ── All ──────────────────────────────────────────────
cmd_all() {
    cmd_test
    cmd_build
    cmd_docker
    cmd_push
    cmd_deploy
    echo ""
    ok "Full pipeline complete: test -> build -> docker -> push -> deploy"
}

# ── Help ─────────────────────────────────────────────
cmd_help() {
    echo "Veyron Build & Deploy"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  test      Run fmt + clippy + all tests"
    echo "  build     Build optimized release binary"
    echo "  docker    Build Docker image (${IMAGE})"
    echo "  push      Push image to container registry"
    echo "  deploy    Deploy to Kubernetes cluster"
    echo "  all       Full pipeline: test -> build -> docker -> push -> deploy"
    echo "  clean     Remove artifacts and images"
    echo ""
    echo "Environment:"
    echo "  VERSION   Override version (default: from Cargo.toml)"
    echo ""
    echo "Examples:"
    echo "  $0 test                    # Run tests only"
    echo "  $0 docker                  # Build Docker image"
    echo "  $0 all                     # Full pipeline"
    echo "  VERSION=0.3.0 $0 docker    # Build with custom version tag"
}

# ── Main ─────────────────────────────────────────────
case "${1:-help}" in
    test)   cmd_test   ;;
    build)  cmd_build  ;;
    docker) cmd_docker ;;
    push)   cmd_push   ;;
    deploy) cmd_deploy ;;
    all)    cmd_all    ;;
    clean)  cmd_clean  ;;
    help|*) cmd_help   ;;
esac
