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
SKIP_DEPS=false
POSITIONAL=()

for arg in "$@"; do
    case "$arg" in
        --fetch) FETCH=true ;;
        --reuse-image) REUSE_IMAGE=true ;;
        --no-virtctl) INCLUDE_VIRTCTL=false ;;
        --skip-deps) SKIP_DEPS=true ;;
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

# shellcheck source=lib/package-remote-ui.sh
source "${SCRIPT_DIR}/lib/package-remote-ui.sh"

pkg_remote_banner "VMRogue" "${VERSION}" "${REMOTE}" "${ARCH}"

if [[ "${VMROGUE_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    pkg_remote_phase "Preflight"
    ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=accept-new \
        "${REMOTE}" "true"
    pkg_ok "SSH ${REMOTE}"
fi

pkg_remote_phase "Sync source"
pkg_remote_kv "Remote build dir" "${BUILD_DIR}"
ssh "${REMOTE}" "mkdir -p '${BUILD_DIR}'"
rsync -az --delete "${RSYNC_EXCLUDES[@]}" \
    -e "ssh -o StrictHostKeyChecking=no" \
    "${REPO_DIR}/" "${REMOTE}:${BUILD_DIR}/"

if ! $SKIP_DEPS; then
    pkg_remote_phase "Build dependencies"
    ssh "${REMOTE}" bash -s <<'REMOTE_DEPS'
set -euo pipefail
SUDO=""
[ "$(id -u)" -ne 0 ] && command -v sudo &>/dev/null && SUDO=sudo
if command -v podman &>/dev/null || command -v docker &>/dev/null; then
  echo "  container runtime: OK"
  exit 0
fi
if command -v dnf &>/dev/null; then
  $SUDO dnf install -y podman 2>&1 | tail -5
elif command -v apt-get &>/dev/null; then
  $SUDO apt-get update -qq
  $SUDO apt-get install -y podman 2>&1 | tail -5 || $SUDO apt-get install -y docker.io 2>&1 | tail -5
else
  echo "Install podman or docker on the build host" >&2
  exit 1
fi
echo "  build deps: OK"
REMOTE_DEPS
fi

CTR_BUILD=$(ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; else echo none; fi')
if [[ "${CTR_BUILD}" = "none" ]]; then
    echo "Remote host needs podman or docker (re-run without --skip-deps)." >&2
    exit 1
fi

pkg_remote_phase "Container build (${CTR_BUILD})"
BUILD_NEEDED=true
if $REUSE_IMAGE; then
    if ssh "${REMOTE}" "${CTR_BUILD} image exists '${IMAGE_TAG}' >/dev/null 2>&1"; then
        BUILD_NEEDED=false
        pkg_ok "Reusing image ${IMAGE_TAG} (--reuse-image)"
    fi
fi

if $BUILD_NEEDED; then
    BUILD_CMD="cd '${BUILD_DIR}' && ${CTR_BUILD} build -t '${IMAGE_TAG}' ."
    if [[ "${CTR_BUILD}" = "docker" ]]; then
        BUILD_CMD="cd '${BUILD_DIR}' && DOCKER_BUILDKIT=1 docker build --progress=plain -t '${IMAGE_TAG}' ."
    fi
    pkg_info "First build often takes 10–15 minutes…"
    ssh "${REMOTE}" "${BUILD_CMD}" 2>&1 | sed 's/^/  [build] /'
    pkg_ok "Image ${IMAGE_TAG} ready"
fi

pkg_remote_phase "Assemble customer bundle"
pkg_remote_kv "Output" "${OUT_DIR}/${ARTIFACT}"
ssh "${REMOTE}" bash -s <<REMOTE_PACK
set -euo pipefail
OUT_DIR='${OUT_DIR}'
BUILD_DIR='${BUILD_DIR}'
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
VMROGUE_API_KEY=CHANGE_ME
# VMROGUE_API_KEYS=admin:secret1,write:secret2,readonly:secret3
KUBECONFIG=/path/to/kubeconfig.yaml
# VMROGUE_NAMESPACE=default
RUST_LOG=info
VMROGUE_SCHEDULER_LEASE_DISABLED=1
ENV_EOF

LIB="\${BUILD_DIR}/scripts/lib"
for f in package-install.sh package-client-install.sh package-client-test.sh; do
  test -f "\${LIB}/\${f}" || { echo "missing \${LIB}/\${f}" >&2; exit 1; }
done
cp "\${LIB}/package-install.sh" "\${OUT_DIR}/\${ARTIFACT}/install.sh"
cp "\${LIB}/package-client-install.sh" "\${OUT_DIR}/\${ARTIFACT}/install-client-deps.sh"
cp "\${LIB}/package-client-test.sh" "\${OUT_DIR}/\${ARTIFACT}/test-package.sh"
mkdir -p "\${OUT_DIR}/\${ARTIFACT}/.package-lib"
cp "\${LIB}/package-ui.sh" "\${OUT_DIR}/\${ARTIFACT}/.package-lib/"
cp "\${LIB}/package-auth-bootstrap.sh" "\${OUT_DIR}/\${ARTIFACT}/.package-lib/"
cp "\${LIB}/install-everything.sh" "\${OUT_DIR}/\${ARTIFACT}/"
cp "\${LIB}/package-uninstall-lib.sh" "\${OUT_DIR}/\${ARTIFACT}/.package-lib/"
cp "\${LIB}/package-uninstall.sh" "\${OUT_DIR}/\${ARTIFACT}/uninstall.sh"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/install.sh" "\${OUT_DIR}/\${ARTIFACT}/install-client-deps.sh" \
  "\${OUT_DIR}/\${ARTIFACT}/test-package.sh" "\${OUT_DIR}/\${ARTIFACT}/install-everything.sh" \
  "\${OUT_DIR}/\${ARTIFACT}/uninstall.sh"
chmod +x "\${LIB}/write-customer-help.sh"
"\${LIB}/write-customer-help.sh" "\${OUT_DIR}/\${ARTIFACT}" "VMRogue" k8s
cp "\${LIB}/START_HERE.txt" "\${OUT_DIR}/\${ARTIFACT}/"
cat > "\${OUT_DIR}/\${ARTIFACT}/.package-lib/product.meta" <<'META'
PRODUCT_NAME=VMRogue
ACCESS_SCHEME=http
ACCESS_PORT=5151
ACCESS_PATH=/dashboard
AUTO_FULL_INSTALL=0
FINISH_EXTRA_1='Cluster (once): ./install-cluster.sh then deploy VMRogue in the cluster'
FINISH_EXTRA_2='Start: set -a && source vmrogue.env && set +a && ./vmrogue api-serve --host 0.0.0.0 --port 5151'
FINISH_EXTRA_3='Docs: CLUSTER_SETUP.txt'
META

CLUSTER_SRC="\${BUILD_DIR}/scripts/cluster"
mkdir -p "\${OUT_DIR}/\${ARTIFACT}/cluster/bootstrap"
cp -a "\${CLUSTER_SRC}/." "\${OUT_DIR}/\${ARTIFACT}/cluster/"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/cluster/install-cluster-prereqs.sh" 2>/dev/null || true
chmod +x "\${OUT_DIR}/\${ARTIFACT}/cluster/bootstrap/ensure-cilium-pod-egress.sh" 2>/dev/null || true
cp "\${LIB}/CLUSTER_SETUP.txt" "\${LIB}/PREREQUISITES.txt" "\${OUT_DIR}/\${ARTIFACT}/"
cp "\${LIB}/package-cluster-prereqs-run.sh" "\${OUT_DIR}/\${ARTIFACT}/install-cluster.sh"
cp "\${LIB}/package-cluster-bootstrap.sh" "\${OUT_DIR}/\${ARTIFACT}/apply-cluster-network.sh"
cp "\${LIB}/package-cluster-test.sh" "\${OUT_DIR}/\${ARTIFACT}/test-cluster.sh"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/install-cluster.sh" "\${OUT_DIR}/\${ARTIFACT}/apply-cluster-network.sh" "\${OUT_DIR}/\${ARTIFACT}/test-cluster.sh"
cat > "\${OUT_DIR}/\${ARTIFACT}/cluster/env.sh" <<'ENVSH'
PRODUCT=VMRogue
APP_NAMESPACE=vmrogue-system
APP_PORT=5151
ENV_PREFIX=VMROGUE
ENVSH

cat > "\${OUT_DIR}/\${ARTIFACT}/QUICKSTART.txt" <<'QEOF'
VMRogue — install guide
========================

CLUSTER FIRST (once per cluster — needs kubectl + admin kubeconfig)
  export KUBECONFIG=/path/to/kubeconfig
  ./install-cluster.sh              # Cilium + KubeVirt + CDI (see CLUSTER_SETUP.txt for flags)
  # Deploy VMRogue in-cluster (Helm/k8s from source repo)
  ./apply-cluster-network.sh        # Cilium egress (skip: VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1)
  ./test-cluster.sh

CLIENT ON THIS MACHINE
  1. tar xzf vmrogue-*-linux-amd64.tar.gz && cd vmrogue-*-linux-amd64
  2. ./install-everything.sh    (or ./install.sh then ./test-package.sh)
     Kubeconfig: auto-detected, or ./install.sh --kubeconfig /path/to/config
  3. nano vmrogue.env   (VMROGUE_API_KEY; KUBECONFIG set by install if found)
  4. set -a && source vmrogue.env && set +a
  5. ./vmrogue api-serve --host 0.0.0.0 --port 5151
  6. ./test-package.sh

Checklist: PREREQUISITES.txt  |  Flags: CLUSTER_SETUP.txt
Remove: ./uninstall.sh --yes [--remove-dir]

Packaged by Zyvor — zyvor.dev · HyperSDK · © 2026
QEOF

cp "\${BUILD_DIR}/scripts/zyvor-branding/ZYVOR_INSTALL.txt" "\${OUT_DIR}/\${ARTIFACT}/ZYVOR_INSTALL.txt" 2>/dev/null || true

cat > "\${OUT_DIR}/\${ARTIFACT}/README.txt" <<README_EOF
VMRogue ${VERSION} — Linux amd64 client bundle
=============================================

START: cat START_HERE.txt  |  full help: cat HELP.txt

WHAT IS IN THIS ARCHIVE
  vmrogue, virtctl (optional)
  install.sh / uninstall.sh     Client on this machine
  install-cluster.sh            Cluster: Cilium + KubeVirt + CDI (kubectl admin)
  apply-cluster-network.sh      Cilium egress bootstrap
  test-cluster.sh / test-package.sh
  CLUSTER_SETUP.txt             All flags and order of operations
  PREREQUISITES.txt             Checklist
  cluster/                      Prereq installer + bootstrap YAML

WHAT MUST EXIST (read PREREQUISITES.txt)
  - Kubernetes + KubeVirt (Deployed) + kubeconfig with VM RBAC
  - CDI recommended (DataVolumes); skip with VMROGUE_SKIP_CDI=1
  - Cilium: only if your CNI is Cilium — then run apply-cluster-network.sh
  - VMRogue deployed IN the cluster (Helm/k8s — separate from this tarball)

ORDER: install-cluster.sh → deploy VMRogue in cluster → apply-cluster-network.sh
      → install.sh → vmrogue.env → test-cluster.sh → api-serve

CLUSTER FLAGS (also V9S_* aliases — see CLUSTER_SETUP.txt)
  VMROGUE_SKIP_CILIUM=1  VMROGUE_SKIP_CDI=1  VMROGUE_SKIP_KUBEVIRT=1
  VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1
  VMROGUE_KUBEVIRT_VERSION / VMROGUE_CDI_VERSION / VMROGUE_CILIUM_CHART_VERSION

CLIENT
  ./install.sh && nano vmrogue.env && ./vmrogue api-serve --host 0.0.0.0 --port 5151
  http://<host>:5151/dashboard

UNINSTALL: ./uninstall.sh --yes [--remove-dir]
README_EOF

for req in install.sh uninstall.sh HELP.txt START_HERE.txt README.txt QUICKSTART.txt CLUSTER_SETUP.txt PREREQUISITES.txt \
  install-cluster.sh apply-cluster-network.sh test-cluster.sh test-package.sh \
  install-client-deps.sh vmrogue vmrogue.env.example; do
  test -e "\${OUT_DIR}/\${ARTIFACT}/\${req}" || { echo "bundle missing \${req}" >&2; exit 1; }
done
chmod +x "\${LIB}/finalize-customer-bundle.sh"
"\${LIB}/finalize-customer-bundle.sh" "\${OUT_DIR}/\${ARTIFACT}" "\${BUILD_DIR}" "VMRogue" "\${VERSION}"
echo "Customer bundle OK (install.sh, README, QUICKSTART, test scripts, binary)"

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

if $FETCH; then
    pkg_remote_phase "Fetch to laptop"
    mkdir -p "${LOCAL_DIST}"
    scp -o StrictHostKeyChecking=no \
        "${REMOTE}:${REMOTE_TARBALL}" \
        "${REMOTE}:${REMOTE_CHECKSUM}" \
        "${LOCAL_DIST}/"
    pkg_ok "Local: ${LOCAL_DIST}/${TARBALL}"
    if command -v shasum >/dev/null 2>&1; then
        (cd "${LOCAL_DIST}" && shasum -a 256 -c "${CHECKSUM}") && pkg_ok "Checksum verified"
    elif command -v sha256sum >/dev/null 2>&1; then
        (cd "${LOCAL_DIST}" && sha256sum -c "${CHECKSUM}") && pkg_ok "Checksum verified"
    fi
fi

pkg_remote_done "VMRogue" "${REMOTE}:${REMOTE_TARBALL}" "${REMOTE}:${REMOTE_CHECKSUM}"
