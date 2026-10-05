#!/usr/bin/env bash
# ============================================================================
# package-binary-remote.sh — Build Veyron on a remote Linux host and tarball it
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
#   --trial        30-day evaluation build (--features trial) with a bundled signed
#                  trial.token — see docs/deploy.md. Produces a distinctly named
#                  veyron-<ver>-trial-linux-amd64.tar.gz, never overwrites the regular
#                  customer tarball. Requires a token minted with trial-tool first:
#                    cargo run --features trial --bin trial-tool -- issue \
#                      --who "..." --days 30 -o dist/trial.token
#   --trial-token=<path>  Token file to bundle (default: dist/trial.token)
#
# Environment:
#   DEPLOY_HOST / DEPLOY_USER     Defaults when host/user omitted
#   VEYRON_PACKAGE_DIR           Remote output dir (default: ~/veyron-dist)
#   VEYRON_PACKAGE_VERSION       Override version in archive name
#   DEPLOY_SSH_TIMEOUT            SSH connect timeout (default: 20)
#   DEPLOY_SSH_PORT               SSH port (default: 22)
#   VEYRON_REMOTE_SKIP_SSH_CHECK=1  Skip SSH preflight
#
# Examples:
#   ./scripts/package-binary-remote.sh <host> <user> --fetch
#   ./scripts/package-binary-remote.sh <host> <user> --reuse-image --fetch
#
# See: docs/deploy.md
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib/deploy-ssh.sh
source "${SCRIPT_DIR}/lib/deploy-ssh.sh"

FETCH=false
REUSE_IMAGE=false
INCLUDE_VIRTCTL=true
SKIP_DEPS=false
TRIAL_BUILD=false
TRIAL_TOKEN_FILE=""
POSITIONAL=()

for arg in "$@"; do
    case "$arg" in
        --fetch) FETCH=true ;;
        --reuse-image) REUSE_IMAGE=true ;;
        --no-virtctl) INCLUDE_VIRTCTL=false ;;
        --skip-deps) SKIP_DEPS=true ;;
        --trial) TRIAL_BUILD=true ;;
        --trial-token=*) TRIAL_TOKEN_FILE="${arg#--trial-token=}" ;;
        -h|--help)
            sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [user] [--fetch] [--reuse-image]" >&2
    echo "  See: docs/deploy.md" >&2
    exit 1
fi

[ -f "${REPO_DIR}/Cargo.toml" ] || { echo "Not in veyron repo (missing Cargo.toml)" >&2; exit 1; }

if $TRIAL_BUILD; then
    TRIAL_TOKEN_FILE="${TRIAL_TOKEN_FILE:-${REPO_DIR}/dist/trial.token}"
    if [[ ! -f "${TRIAL_TOKEN_FILE}" ]]; then
        echo "Error: --trial requires a signed token at ${TRIAL_TOKEN_FILE} (or --trial-token=<path>)." >&2
        echo "  Mint one first: cargo run --features trial --bin trial-tool -- issue --who \"...\" --days 30 -o dist/trial.token" >&2
        exit 1
    fi
fi

VERSION="${VEYRON_PACKAGE_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_DIR}/Cargo.toml" | head -1)}"
VERSION="${VERSION:-0.0.0}"
ARCH="linux-amd64"
REMOTE="${USER}@${HOST}"
REMOTE_HOME=$(deploy_ssh "${REMOTE}" 'echo "$HOME"')
BUILD_DIR="${REMOTE_HOME}/.deployment/veyron-package"
OUT_DIR="${VEYRON_PACKAGE_DIR:-${REMOTE_HOME}/veyron-dist}"
if $TRIAL_BUILD; then
    IMAGE_TAG="veyron-package:${VERSION}-trial"
    ARTIFACT="veyron-${VERSION}-trial-${ARCH}"
else
    IMAGE_TAG="veyron-package:${VERSION}"
    ARTIFACT="veyron-${VERSION}-${ARCH}"
fi
TARBALL="${ARTIFACT}.tar.gz"
CHECKSUM="${TARBALL}.sha256"
LOCAL_DIST="${REPO_DIR}/dist"

RSYNC_EXCLUDES=(
    --exclude='target/'
    --exclude='.git/'
    --exclude='operator/bin/'
    --exclude='e2e/node_modules/'
    --exclude='*.qcow2'
    --exclude='*.vmdk'
    --exclude='*.iso'
)

# shellcheck source=lib/package-remote-ui.sh
source "${SCRIPT_DIR}/lib/package-remote-ui.sh"

pkg_remote_banner "Veyron" "${VERSION}" "${REMOTE}" "${ARCH}"

if [[ "${VEYRON_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    pkg_remote_phase "Preflight"
    SSH_ERR=""
    if ! SSH_ERR=$(deploy_ssh_preflight "${REMOTE}"); then
        pkg_fail "SSH preflight failed on port ${DEPLOY_SSH_PORT}: ${SSH_ERR}"
        exit 1
    fi
    pkg_ok "SSH ${REMOTE} (port ${DEPLOY_SSH_PORT})"
fi

pkg_remote_phase "Sync source"
pkg_remote_kv "Remote build dir" "${BUILD_DIR}"
deploy_ssh "${REMOTE}" "mkdir -p '${BUILD_DIR}'"
rsync -az --delete "${RSYNC_EXCLUDES[@]}" \
    -e "$(deploy_rsync_ssh)" \
    "${REPO_DIR}/" "${REMOTE}:${BUILD_DIR}/"

if ! $SKIP_DEPS; then
    pkg_remote_phase "Build dependencies"
    deploy_ssh "${REMOTE}" bash -s <<'REMOTE_DEPS'
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

CTR_BUILD=$(deploy_ssh "${REMOTE}" 'if command -v podman >/dev/null 2>&1; then echo podman; elif command -v docker >/dev/null 2>&1; then echo docker; else echo none; fi')
if [[ "${CTR_BUILD}" = "none" ]]; then
    echo "Remote host needs podman or docker (re-run without --skip-deps)." >&2
    exit 1
fi

pkg_remote_phase "Container build (${CTR_BUILD})"
BUILD_NEEDED=true
if $REUSE_IMAGE; then
    if deploy_ssh "${REMOTE}" "${CTR_BUILD} image exists '${IMAGE_TAG}' >/dev/null 2>&1"; then
        BUILD_NEEDED=false
        pkg_ok "Reusing image ${IMAGE_TAG} (--reuse-image)"
    fi
fi

if $BUILD_NEEDED; then
    BUILD_ARGS=""
    $TRIAL_BUILD && BUILD_ARGS="--build-arg VEYRON_EXTRA_FEATURES=trial"
    BUILD_CMD="cd '${BUILD_DIR}' && ${CTR_BUILD} build ${BUILD_ARGS} -t '${IMAGE_TAG}' ."
    if [[ "${CTR_BUILD}" = "docker" ]]; then
        BUILD_CMD="cd '${BUILD_DIR}' && DOCKER_BUILDKIT=1 docker build ${BUILD_ARGS} --progress=plain -t '${IMAGE_TAG}' ."
    fi
    pkg_info "First build often takes 10–15 minutes…"
    deploy_ssh "${REMOTE}" "${BUILD_CMD}" 2>&1 | sed 's/^/  [build] /'
    pkg_ok "Image ${IMAGE_TAG} ready"
fi

pkg_remote_phase "Assemble customer bundle"
pkg_remote_kv "Output" "${OUT_DIR}/${ARTIFACT}"
deploy_ssh "${REMOTE}" bash -s <<REMOTE_PACK
set -euo pipefail
OUT_DIR='${OUT_DIR}'
BUILD_DIR='${BUILD_DIR}'
IMAGE_TAG='${IMAGE_TAG}'
INCLUDE_VIRTCTL='${INCLUDE_VIRTCTL}'
ARTIFACT='${ARTIFACT}'
VERSION='${VERSION}'
CTR='${CTR_BUILD}'

mkdir -p "\${OUT_DIR}/\${ARTIFACT}"
\${CTR} rm -f veyron-package-extract 2>/dev/null || true
\${CTR} create --name veyron-package-extract "\${IMAGE_TAG}" >/dev/null
\${CTR} cp veyron-package-extract:/usr/local/bin/veyron "\${OUT_DIR}/\${ARTIFACT}/veyron"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/veyron"
if [[ "\${INCLUDE_VIRTCTL}" == "true" ]]; then
  \${CTR} cp veyron-package-extract:/usr/local/bin/virtctl "\${OUT_DIR}/\${ARTIFACT}/virtctl" 2>/dev/null || true
  chmod +x "\${OUT_DIR}/\${ARTIFACT}/virtctl" 2>/dev/null || true
fi
# In-guest agent binary (per-VM auto-install) — the image has it at /usr/local/bin/guestkit
# alongside veyron/virtctl, but nothing extracted it into the bundle until now.
\${CTR} cp veyron-package-extract:/usr/local/bin/guestkit "\${OUT_DIR}/\${ARTIFACT}/guestkit" 2>/dev/null || true
chmod +x "\${OUT_DIR}/\${ARTIFACT}/guestkit" 2>/dev/null || true
\${CTR} rm -f veyron-package-extract >/dev/null

# Kubernetes deploy path: bundled Helm chart + a runtime-only Dockerfile that repackages
# the binaries already staged above (no source tree, no cargo at customer install time)
# + install-to-kubernetes.sh (customer-facing wrapper, staged below with the other scripts).
cp -a "\${BUILD_DIR}/charts/veyron" "\${OUT_DIR}/\${ARTIFACT}/chart"
cp "\${BUILD_DIR}/deployments/docker/Dockerfile.veyron-bundle" "\${OUT_DIR}/\${ARTIFACT}/Dockerfile"

cat > "\${OUT_DIR}/\${ARTIFACT}/veyron.env.example" <<'ENV_EOF'
# Copy to veyron.env and adjust before starting the API.
VEYRON_API_KEY=CHANGE_ME
# VEYRON_API_KEYS=admin:secret1,write:secret2,readonly:secret3
KUBECONFIG=/path/to/kubeconfig.yaml
# VEYRON_NAMESPACE=default
RUST_LOG=info
VEYRON_SCHEDULER_LEASE_DISABLED=1
ENV_EOF

LIB="\${BUILD_DIR}/scripts/lib"
for f in package-install.sh package-client-install.sh package-client-test.sh package-client-k8s-install.sh; do
  test -f "\${LIB}/\${f}" || { echo "missing \${LIB}/\${f}" >&2; exit 1; }
done
cp "\${LIB}/package-install.sh" "\${OUT_DIR}/\${ARTIFACT}/install.sh"
cp "\${LIB}/package-client-install.sh" "\${OUT_DIR}/\${ARTIFACT}/install-client-deps.sh"
cp "\${LIB}/package-client-test.sh" "\${OUT_DIR}/\${ARTIFACT}/test-package.sh"
cp "\${LIB}/package-client-k8s-install.sh" "\${OUT_DIR}/\${ARTIFACT}/install-to-kubernetes.sh"
chmod +x "\${OUT_DIR}/\${ARTIFACT}/install-to-kubernetes.sh"
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
"\${LIB}/write-customer-help.sh" "\${OUT_DIR}/\${ARTIFACT}" "Veyron" k8s
cp "\${LIB}/START_HERE.txt" "\${OUT_DIR}/\${ARTIFACT}/"
cat > "\${OUT_DIR}/\${ARTIFACT}/.package-lib/product.meta" <<'META'
PRODUCT_NAME=Veyron
ACCESS_SCHEME=http
ACCESS_PORT=5151
ACCESS_PATH=/dashboard
AUTO_FULL_INSTALL=0
FINISH_EXTRA_1='Cluster (once): ./install-cluster.sh then deploy Veyron in the cluster'
FINISH_EXTRA_2='Start: set -a && source veyron.env && set +a && ./veyron api-serve --host 0.0.0.0 --port 5151'
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
PRODUCT=Veyron
APP_NAMESPACE=veyron-system
APP_PORT=5151
ENV_PREFIX=VEYRON
ENVSH

cat > "\${OUT_DIR}/\${ARTIFACT}/QUICKSTART.txt" <<'QEOF'
Veyron — install guide
========================

CLUSTER FIRST (once per cluster — needs kubectl + admin kubeconfig)
  export KUBECONFIG=/path/to/kubeconfig
  ./install-cluster.sh              # Cilium + KubeVirt + CDI (see CLUSTER_SETUP.txt for flags)
  # Deploy Veyron in-cluster (builds an image from this bundle's own binaries — no
  # source tree, no compile step — then helm upgrade --install ./chart):
  ./install-to-kubernetes.sh --registry <your-registry>/veyron --push
  ./apply-cluster-network.sh        # Cilium egress (skip: VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1)
  ./test-cluster.sh

CLIENT ON THIS MACHINE
  1. tar xzf veyron-*-linux-amd64.tar.gz && cd veyron-*-linux-amd64
  2. ./install-everything.sh    (or ./install.sh then ./test-package.sh)
     Kubeconfig: auto-detected, or ./install.sh --kubeconfig /path/to/config
  3. nano veyron.env   (VEYRON_API_KEY; KUBECONFIG set by install if found)
  4. set -a && source veyron.env && set +a
  5. ./veyron api-serve --host 0.0.0.0 --port 5151
  6. ./test-package.sh

Checklist: PREREQUISITES.txt  |  Flags: CLUSTER_SETUP.txt
Remove: ./uninstall.sh --yes [--remove-dir]

Packaged by Zyvor — zyvor.dev · Zyvor · © 2026
QEOF

cat > "\${OUT_DIR}/\${ARTIFACT}/README.txt" <<README_EOF
Veyron ${VERSION} — Linux amd64 client bundle
=============================================

START: cat START_HERE.txt  |  full help: cat HELP.txt

WHAT IS IN THIS ARCHIVE
  veyron, virtctl (optional), guestkit (optional in-guest agent)
  install.sh / uninstall.sh     Client on this machine
  install-cluster.sh            Cluster: Cilium + KubeVirt + CDI (kubectl admin)
  chart/, Dockerfile, install-to-kubernetes.sh   Kubernetes deploy path
  apply-cluster-network.sh      Cilium egress bootstrap
  test-cluster.sh / test-package.sh
  CLUSTER_SETUP.txt             All flags and order of operations
  PREREQUISITES.txt             Checklist
  cluster/                      Prereq installer + bootstrap YAML

WHAT MUST EXIST (read PREREQUISITES.txt)
  - Kubernetes + KubeVirt (Deployed) + kubeconfig with VM RBAC
  - CDI recommended (DataVolumes); skip with VEYRON_SKIP_CDI=1
  - Cilium: only if your CNI is Cilium — then run apply-cluster-network.sh
  - Veyron deployed IN the cluster — via install-to-kubernetes.sh (this tarball), or
    the standalone client process below
  - Kubernetes deploy also needs: docker or podman, Helm 3.12+

ORDER: install-cluster.sh → install-to-kubernetes.sh → apply-cluster-network.sh
      (standalone alternative: install.sh → veyron.env → test-cluster.sh → api-serve)

CLUSTER FLAGS (also V9S_* aliases — see CLUSTER_SETUP.txt)
  VEYRON_SKIP_CILIUM=1  VEYRON_SKIP_CDI=1  VEYRON_SKIP_KUBEVIRT=1
  VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1
  VEYRON_KUBEVIRT_VERSION / VEYRON_CDI_VERSION / VEYRON_CILIUM_CHART_VERSION

CLIENT
  ./install.sh && nano veyron.env && ./veyron api-serve --host 0.0.0.0 --port 5151
  http://<host>:5151/dashboard

UNINSTALL: ./uninstall.sh --yes [--remove-dir]
README_EOF

chmod +x "\${LIB}/finalize-customer-bundle.sh"
"\${LIB}/finalize-customer-bundle.sh" "\${OUT_DIR}/\${ARTIFACT}" "\${BUILD_DIR}" "Veyron" "\${VERSION}"
for req in LICENSE NOTICE install.sh uninstall.sh HELP.txt START_HERE.txt README.txt QUICKSTART.txt CLUSTER_SETUP.txt PREREQUISITES.txt \
  install-cluster.sh apply-cluster-network.sh test-cluster.sh test-package.sh \
  install-client-deps.sh veyron veyron.env.example chart Dockerfile install-to-kubernetes.sh; do
  test -e "\${OUT_DIR}/\${ARTIFACT}/\${req}" || { echo "bundle missing \${req}" >&2; exit 1; }
done
echo "Customer bundle OK (install.sh, README, QUICKSTART, test scripts, binary)"
REMOTE_PACK

if $TRIAL_BUILD; then
    pkg_remote_phase "Bundle signed trial token"
    scp -o StrictHostKeyChecking=no "${TRIAL_TOKEN_FILE}" "${REMOTE}:${OUT_DIR}/${ARTIFACT}/trial.token"
    deploy_ssh "${REMOTE}" bash -s <<REMOTE_TRIAL_NOTICE
set -euo pipefail
D='${OUT_DIR}/${ARTIFACT}'
test -f "\${D}/trial.token" || { echo "trial.token missing after scp" >&2; exit 1; }
for f in README.txt QUICKSTART.txt; do
  [ -f "\${D}/\${f}" ] || continue
  { printf '%s\n' \
      '=== 30-DAY EVALUATION BUILD ===' \
      'This bundle includes a signed trial.token (next to the veyron binary).' \
      'Keep it in place, or: export VEYRON_TRIAL_TOKEN="\$(cat trial.token)"' \
      'After expiry, contact sales@zyvor.dev for a renewed token.' \
      ''; \
    cat "\${D}/\${f}"; \
  } > "\${D}/\${f}.new"
  mv "\${D}/\${f}.new" "\${D}/\${f}"
done
echo "Trial notice prepended to README.txt / QUICKSTART.txt"
REMOTE_TRIAL_NOTICE
    pkg_ok "trial.token bundled (30-day evaluation build)"
fi

deploy_ssh "${REMOTE}" bash -s <<REMOTE_TAR
set -euo pipefail
cd "${OUT_DIR}"
rm -f "${ARTIFACT}.tar.gz" "${ARTIFACT}.tar.gz.sha256"
tar czf "${ARTIFACT}.tar.gz" "${ARTIFACT}"
sha256sum "${ARTIFACT}.tar.gz" | tee "${ARTIFACT}.tar.gz.sha256"
ls -lh "${ARTIFACT}.tar.gz" "${ARTIFACT}/veyron"
file "${ARTIFACT}/veyron"
"${OUT_DIR}/${ARTIFACT}/veyron" --help | head -3
REMOTE_TAR

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

pkg_remote_done "Veyron" "${REMOTE}:${REMOTE_TARBALL}" "${REMOTE}:${REMOTE_CHECKSUM}"
