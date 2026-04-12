#!/usr/bin/env bash
# VMRogue Build, Test & Deploy Script
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

REPO="ghcr.io/ssahani/VMRogue"
VERSION="${VERSION:-$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')}"
IMAGE="${REPO}:${VERSION}"
IMAGE_LATEST="${REPO}:latest"
NAMESPACE="vmrogue-system"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${CYAN}[vmrogue]${NC} $*"; }
ok()   { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[✗]${NC} $*" >&2; }

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
    strip target/release/vmrogue 2>/dev/null || true

    local size
    size=$(du -h target/release/vmrogue | cut -f1)
    ok "Binary: target/release/vmrogue ($size)"
}

# ── Docker ───────────────────────────────────────────
cmd_docker() {
    log "Building Docker image: ${IMAGE}"
    docker build -t "${IMAGE}" -t "${IMAGE_LATEST}" .
    ok "Image built: ${IMAGE}"

    local size
    size=$(docker image inspect "${IMAGE}" --format='{{.Size}}' | numfmt --to=iec 2>/dev/null || echo "unknown")
    log "Image size: ${size}"
}

# ── Push ─────────────────────────────────────────────
cmd_push() {
    log "Pushing ${IMAGE}..."
    docker push "${IMAGE}"
    docker push "${IMAGE_LATEST}"
    ok "Pushed ${IMAGE} and ${IMAGE_LATEST}"
}

# ── Deploy ───────────────────────────────────────────
cmd_deploy() {
    log "Deploying to Kubernetes..."

    if ! kubectl cluster-info &>/dev/null; then
        err "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    # Create namespace if needed
    kubectl create namespace "${NAMESPACE}" --dry-run=client -o yaml | kubectl apply -f -

    # Apply manifests
    kubectl apply -f deploy/k8s.yaml

    # Update image
    kubectl -n "${NAMESPACE}" set image deployment/vmrogue-api \
        vmrogue="${IMAGE}" 2>/dev/null || true

    # Wait for rollout
    log "Waiting for rollout..."
    kubectl -n "${NAMESPACE}" rollout status deployment/vmrogue-api --timeout=120s

    ok "Deployed ${IMAGE} to ${NAMESPACE}"
    kubectl -n "${NAMESPACE}" get pods -l app.kubernetes.io/name=vmrogue
}

# ── Clean ────────────────────────────────────────────
cmd_clean() {
    log "Cleaning up..."
    cargo clean
    docker rmi "${IMAGE}" "${IMAGE_LATEST}" 2>/dev/null || true
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
    echo "VMRogue Build & Deploy"
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
