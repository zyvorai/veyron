#!/usr/bin/env bash
# ============================================================================
# package-binary-remote.sh — Build VMRogue on a remote Linux host and tarball it
# ============================================================================
# Minimal rsync (no node_modules), podman/docker build via Dockerfile, extract
# static linux/amd64 binary (+ virtctl), produce .tar.gz + .sha256 for clients.
#
# Usage:
#   ./scripts/package-binary-remote.sh <host> [user] [--fetch] [--reuse-image]
#
# Options:
#   --fetch        Copy the tarball to ./dist/ on your laptop after build
#   --reuse-image  Skip podman build if the package image tag already exists
#   --no-virtctl   Omit virtctl from the tarball (smaller)
#
# Environment:
#   DEPLOY_HOST / DEPLOY_USER     Defaults when host/user omitted
#   VMROGUE_PACKAGE_DIR           Remote output dir (default: ~/vmrogue-dist)
#   VMROGUE_PACKAGE_VERSION       Override version in archive name
#   DEPLOY_SSH_TIMEOUT            SSH connect timeout (default: 20)
#   VMROGUE_REMOTE_SKIP_SSH_CHECK=1  Skip SSH preflight
#
# Examples:
#   ./scripts/package-binary-remote.sh HOST sus --fetch
#   ./scripts/package-binary-remote.sh HOST sus --reuse-image --fetch
#
# See: docs/PACKAGE_BINARY_REMOTE.md
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

FETCH=false
REUSE_IMAGE=false
INCLUDE_VIRTCTL=true
POSITIONAL=()

for arg in "$@"; do
    case "$arg" in
        --fetch) FETCH=true ;;
        --reuse-image) REUSE_IMAGE=true ;;
        --no-virtctl) INCLUDE_VIRTCTL=false ;;
        -h|--help)
            sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
SSH_TIMEOUT="${DEPLOY_SSH_TIMEOUT:-20}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [user] [--fetch] [--reuse-image]" >&2
    echo "  See: docs/PACKAGE_BINARY_REMOTE.md" >&2
    exit 1
fi

[ -f "${REPO_DIR}/Cargo.toml" ] || { echo "Not in vmrogue repo (missing Cargo.toml)" >&2; exit 1; }

VERSION="${VMROGUE_PACKAGE_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_DIR}/Cargo.toml" | head -1)}"
VERSION="${VERSION:-0.0.0}"
ARCH="linux-amd64"
REMOTE="${USER}@${HOST}"
REMOTE_HOME=$(ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" "${REMOTE}" 'echo "$HOME"')
BUILD_DIR="${REMOTE_HOME}/.deployment/vmrogue-package"
OUT_DIR="${VMROGUE_PACKAGE_DIR:-${REMOTE_HOME}/vmrogue-dist}"
IMAGE_TAG="vmrogue-package:${VERSION}"
ARTIFACT="vmrogue-${VERSION}-${ARCH}"
TARBALL="${ARTIFACT}.tar.gz"
CHECKSUM="${TARBALL}.sha256"
LOCAL_DIST="${REPO_DIR}/dist"

RSYNC_EXCLUDES=(
    --exclude='target/'
    --exclude='.git/'
    --exclude='operator/bin/'
    --exclude='web/dashboard-next/node_modules/'
    --exclude='web/dashboard-next/dist/'
    --exclude='web/dashboard-next/test-results/'
    --exclude='e2e/node_modules/'
    --exclude='*.qcow2'
    --exclude='*.vmdk'
    --exclude='*.iso'
)

log() { printf '  %s\n' "$*"; }
step() { echo ""; printf '── %s\n' "$*"; }

if [[ "${VMROGUE_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    step "Preflight: SSH (${REMOTE})"
    ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=accept-new \
        "${REMOTE}" "true"
    log "SSH OK"
fi

CTR_BUILD=$(ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; else echo none; fi')
if [[ "${CTR_BUILD}" = "none" ]]; then
    echo "Remote host needs podman or docker to build the package image." >&2
    exit 1
fi

step "Sync source → ${HOST}:${BUILD_DIR}"
ssh "${REMOTE}" "mkdir -p '${BUILD_DIR}'"
rsync -az --delete "${RSYNC_EXCLUDES[@]}" \
    -e "ssh -o StrictHostKeyChecking=no" \
    "${REPO_DIR}/" "${REMOTE}:${BUILD_DIR}/"

step "Build package image on remote (${CTR_BUILD})"
BUILD_NEEDED=true
if $REUSE_IMAGE; then
    if ssh "${REMOTE}" "${CTR_BUILD} image exists '${IMAGE_TAG}' >/dev/null 2>&1"; then
        BUILD_NEEDED=false
        log "Reusing existing image ${IMAGE_TAG} (--reuse-image)"
    fi
fi

if $BUILD_NEEDED; then
    BUILD_CMD="cd '${BUILD_DIR}' && ${CTR_BUILD} build -t '${IMAGE_TAG}' ."
    if [[ "${CTR_BUILD}" = "docker" ]]; then
        BUILD_CMD="cd '${BUILD_DIR}' && DOCKER_BUILDKIT=1 docker build --progress=plain -t '${IMAGE_TAG}' ."
    fi
    log "Building (this usually takes 10–15 minutes on first run)…"
    ssh "${REMOTE}" "${BUILD_CMD}" 2>&1 | sed 's/^/  [build] /'
    log "Image ${IMAGE_TAG} ready"
fi

step "Extract binaries into ${OUT_DIR}"
ssh "${REMOTE}" bash -s <<REMOTE_PACK
set -euo pipefail
OUT_DIR='${OUT_DIR}'
IMAGE_TAG='${IMAGE_TAG}'
INCLUDE_VIRTCTL='${INCLUDE_VIRTCTL}'
ARTIFACT='${ARTIFACT}'
VERSION='${VERSION}'
CTR='${CTR_BUILD}'

mkdir -p "\${OUT_DIR}/\${ARTIFACT}"
\${CTR} rm -f vmrogue-package-extract 2>/dev/null || true
\${CTR} create --name vmrogue-package-extract "\${IMAGE_TAG}" >/dev/null
\${CTR} cp vmrogue-package-extract:/usr/local/bin/vmrogue "\${OUT_DIR}/\${ARTIFACT}/vmrogue"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/vmrogue"
if [[ "\${INCLUDE_VIRTCTL}" == "true" ]]; then
  \${CTR} cp vmrogue-package-extract:/usr/local/bin/virtctl "\${OUT_DIR}/\${ARTIFACT}/virtctl" 2>/dev/null || true
  chmod +x "\${OUT_DIR}/\${ARTIFACT}/virtctl" 2>/dev/null || true
fi
\${CTR} rm -f vmrogue-package-extract >/dev/null

cat > "\${OUT_DIR}/\${ARTIFACT}/vmrogue.env.example" <<'ENV_EOF'
# Copy to vmrogue.env and adjust before starting the API.
VMROGUE_API_KEY=change-me-strong-secret
# VMROGUE_API_KEYS=admin:secret1,write:secret2,readonly:secret3
KUBECONFIG=/path/to/kubeconfig.yaml
# VMROGUE_NAMESPACE=default
RUST_LOG=info
VMROGUE_SCHEDULER_LEASE_DISABLED=1
ENV_EOF

cat > "\${OUT_DIR}/\${ARTIFACT}/README.txt" <<README_EOF
VMRogue ${VERSION} — Linux amd64 client bundle
=============================================

Contents:
  vmrogue              CLI + API server (embedded dashboard)
  virtctl              Optional KubeVirt helper (if present)
  vmrogue.env.example  Environment template

Requirements:
  - Kubernetes cluster with KubeVirt (and kubeconfig access)
  - Linux x86_64

Quick start (API + dashboard):
  export KUBECONFIG=/path/to/kubeconfig
  export VMROGUE_API_KEY=your-secret
  ./vmrogue api-serve --host 0.0.0.0 --port 5151
  Open http://<this-host>:5151/dashboard

CLI examples:
  ./vmrogue list --namespace all
  ./vmrogue create my-vm --template ubuntu-22.04

HTTPS:
  ./vmrogue api-serve --host 0.0.0.0 --port 5151 \\
    --tls --tls-cert /path/to/tls.crt --tls-key /path/to/tls.key

Full guide: docs/PACKAGE_BINARY_REMOTE.md (in the VMRogue source repo)
README_EOF

cd "\${OUT_DIR}"
rm -f "\${ARTIFACT}.tar.gz" "\${ARTIFACT}.tar.gz.sha256"
tar czf "\${ARTIFACT}.tar.gz" "\${ARTIFACT}"
sha256sum "\${ARTIFACT}.tar.gz" | tee "\${ARTIFACT}.tar.gz.sha256"
ls -lh "\${ARTIFACT}.tar.gz" "\${ARTIFACT}/vmrogue"
file "\${ARTIFACT}/vmrogue"
"\${OUT_DIR}/\${ARTIFACT}/vmrogue" --help | head -3
REMOTE_PACK

REMOTE_TARBALL="${OUT_DIR}/${TARBALL}"
REMOTE_CHECKSUM="${OUT_DIR}/${CHECKSUM}"

step "Package ready on remote"
log "Tarball:  ${REMOTE}:${REMOTE_TARBALL}"
log "Checksum: ${REMOTE}:${REMOTE_CHECKSUM}"

if $FETCH; then
    step "Fetching tarball to ${LOCAL_DIST}/"
    mkdir -p "${LOCAL_DIST}"
    scp -o StrictHostKeyChecking=no \
        "${REMOTE}:${REMOTE_TARBALL}" \
        "${REMOTE}:${REMOTE_CHECKSUM}" \
        "${LOCAL_DIST}/"
    log "Local: ${LOCAL_DIST}/${TARBALL}"
    log "Local: ${LOCAL_DIST}/${CHECKSUM}"
    if command -v shasum >/dev/null 2>&1; then
        (cd "${LOCAL_DIST}" && shasum -a 256 -c "${CHECKSUM}") && log "Checksum OK (shasum)"
    elif command -v sha256sum >/dev/null 2>&1; then
        (cd "${LOCAL_DIST}" && sha256sum -c "${CHECKSUM}") && log "Checksum OK (sha256sum)"
    fi
fi

echo ""
echo "════════════════════════════════════════"
echo "  Package complete"
echo "════════════════════════════════════════"
echo "  Remote:  ${USER}@${HOST}"
echo "  Archive: ${REMOTE_TARBALL}"
echo ""
echo "  Download:"
echo "    scp ${REMOTE}:${REMOTE_TARBALL} ."
echo "    scp ${REMOTE}:${REMOTE_CHECKSUM} ."
if $FETCH; then
    echo ""
    echo "  Fetched to: ${LOCAL_DIST}/${TARBALL}"
fi
echo ""
echo "  Docs: docs/PACKAGE_BINARY_REMOTE.md"
echo "════════════════════════════════════════"
