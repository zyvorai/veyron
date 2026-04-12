#!/bin/bash
# ============================================================================
# remote-deploy.sh — Full vmrogue deployment to a remote server
# ============================================================================
# Rsync source to remote, compile there, install binary + service, verify.
#
# Usage:
#   ./deploy/remote-deploy.sh <host> [user] [password]
#   ./deploy/remote-deploy.sh HOST sus
#   ./deploy/remote-deploy.sh HOST sus mypassword
#   ./deploy/remote-deploy.sh HOST sus --quick    # skip deps
#   ./deploy/remote-deploy.sh HOST sus --uninstall
#
# Environment variables:
#   DEPLOY_HOST=HOST
#   DEPLOY_USER=sus
#   DEPLOY_PASS=mypassword
#   DEPLOY_DIR=/home/sus/vmrogue  (auto-detected from login user)
# ============================================================================

set -euo pipefail

info()  { echo "  [ok] $*"; }
warn()  { echo "  [!!] $*"; }
error() { echo "  [ERR] $*"; exit 1; }
step()  { echo ""; echo "  --- $*"; }

# ── Parse args ──
QUICK_MODE=false
UNINSTALL_MODE=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --quick)     QUICK_MODE=true ;;
        --uninstall) UNINSTALL_MODE=true ;;
        --help|-h)
            echo "Usage: $0 <host> [user] [password] [--quick|--uninstall]"
            echo ""
            echo "  --quick      Skip Rust toolchain install (rsync + build only)"
            echo "  --uninstall  Remove vmrogue from remote server"
            echo ""
            echo "Full mode installs: Rust toolchain, builds from source,"
            echo "deploys binary + systemd service + k3s manifest."
            exit 0
            ;;
        *)  POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
PASS="${POSITIONAL[2]:-${DEPLOY_PASS:-}}"

[ -z "$HOST" ] && error "Usage: $0 <host> [user] [password] [--quick]"

# Determine remote home directory based on login user
if [ "$USER" = "root" ]; then
    REMOTE_HOME="/root"
else
    REMOTE_HOME="/home/${USER}"
fi
REMOTE_DIR="${DEPLOY_DIR:-${REMOTE_HOME}/vmrogue}"

# Use sudo when not deploying as root
SUDO=""
[ "$USER" != "root" ] && SUDO="sudo"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

[ -f "$REPO_DIR/Cargo.toml" ] || error "Not in vmrogue repo: $REPO_DIR"

# ── SSH/rsync wrappers ──
_ssh() {
    if [ -n "$PASS" ]; then
        SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    else
        ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    fi
}

_scp() {
    if [ -n "$PASS" ]; then
        SSHPASS="$PASS" sshpass -e scp -o StrictHostKeyChecking=no "$@"
    else
        scp -o StrictHostKeyChecking=no "$@"
    fi
}

_rsync() {
    local ssh_cmd="ssh -o StrictHostKeyChecking=no"
    if [ -n "$PASS" ]; then
        ssh_cmd="sshpass -e $ssh_cmd"
    fi
    SSHPASS="$PASS" rsync -avz \
        --exclude='target/' \
        --exclude='.git' \
        --exclude='*.qcow2' --exclude='*.vmdk' --exclude='*.raw' \
        --exclude='*.iso' --exclude='*.img' \
        -e "$ssh_cmd" \
        "$@"
}

# ── Preflight ──
if [ -n "$PASS" ] && ! command -v sshpass &>/dev/null; then
    error "sshpass required for password auth. Install: dnf install sshpass"
fi

# ── Uninstall mode ──
if $UNINSTALL_MODE; then
    echo ""
    echo "  ============================================"
    echo "    vmrogue Remote Uninstall"
    echo "  ============================================"
    echo ""
    echo "  Host: ${USER}@${HOST}"
    echo ""

    step "Uninstalling vmrogue"
    _ssh "
        $SUDO systemctl stop vmrogue-web.service 2>/dev/null || true
        $SUDO systemctl disable vmrogue-web.service 2>/dev/null || true
        $SUDO rm -f /usr/local/bin/vmrogue
        $SUDO rm -f /etc/systemd/system/vmrogue-web.service
        $SUDO systemctl daemon-reload
        rm -rf $REMOTE_DIR
        echo 'Done'
    " 2>&1
    info "vmrogue removed from ${HOST}"
    echo ""
    exit 0
fi

TOTAL_STEPS=5
$QUICK_MODE && TOTAL_STEPS=3

echo ""
echo "  ============================================"
echo "    vmrogue Remote Deployment"
echo "  ============================================"
echo ""
echo "  Host:     ${USER}@${HOST}"
echo "  Auth:     $([ -n "$PASS" ] && echo "password" || echo "SSH key")"
echo "  Local:    $REPO_DIR"
echo "  Remote:   $REMOTE_DIR"
echo "  Mode:     $($QUICK_MODE && echo "quick (rsync + build only)" || echo "full (Rust toolchain + build + deploy)")"
echo ""

# ── Step 1: Rsync repo ──
step "Step 1/${TOTAL_STEPS}: Syncing repository to ${HOST}"

_rsync "$REPO_DIR/" "${USER}@${HOST}:${REMOTE_DIR}/" 2>&1 | tail -3
info "Synced to ${HOST}:${REMOTE_DIR}"

if ! $QUICK_MODE; then
    # ── Step 2: Install Rust toolchain if needed ──
    step "Step 2/${TOTAL_STEPS}: Ensuring Rust toolchain on remote"

    _ssh "
        if command -v cargo &>/dev/null; then
            echo \"Rust already installed: \$(rustc --version)\"
        else
            echo 'Installing Rust via rustup...'
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source \$HOME/.cargo/env
            echo \"Installed: \$(rustc --version)\"
        fi

        # Ensure build deps
        if command -v dnf &>/dev/null; then
            $SUDO dnf install -y gcc openssl-devel pkg-config 2>&1 | tail -1
        elif command -v apt-get &>/dev/null; then
            $SUDO apt-get install -y build-essential libssl-dev pkg-config 2>&1 | tail -1
        fi
    " 2>&1
    info "Rust toolchain ready"
fi

# ── Build on remote ──
if $QUICK_MODE; then
    BUILD_STEP=2
else
    BUILD_STEP=3
fi
step "Step ${BUILD_STEP}/${TOTAL_STEPS}: Building release binary on remote"

_ssh "
    source \$HOME/.cargo/env 2>/dev/null || true
    cd $REMOTE_DIR
    cargo build --release 2>&1 | tail -5
    strip target/release/vmrogue 2>/dev/null || true
    ls -lh target/release/vmrogue
" 2>&1
info "Binary built on remote"

# ── Install binary + service ──
if $QUICK_MODE; then
    INSTALL_STEP=3
else
    INSTALL_STEP=4
fi
step "Step ${INSTALL_STEP}/${TOTAL_STEPS}: Installing binary and service"

_ssh "
    cd $REMOTE_DIR

    # Stop service before replacing binary (avoids 'Text file busy')
    $SUDO systemctl stop vmrogue-web.service 2>/dev/null || true

    # Install binary
    $SUDO cp target/release/vmrogue /usr/local/bin/vmrogue
    $SUDO chmod 755 /usr/local/bin/vmrogue

    # Install systemd service
    $SUDO cp deploy/vmrogue-web.service /etc/systemd/system/vmrogue-web.service
    $SUDO systemctl daemon-reload

    # Generate and install API key if not already present
    $SUDO mkdir -p /etc/vmrogue
    if [ ! -f /etc/vmrogue/env ]; then
        API_KEY=\$(openssl rand -hex 32)
        echo \"VMROGUE_API_KEY=\${API_KEY}\" | $SUDO tee /etc/vmrogue/env > /dev/null
        $SUDO chmod 600 /etc/vmrogue/env
        echo \"API key generated: \${API_KEY}\"
        echo \"Save this key — it is required for API access.\"
    else
        echo 'API key: already configured (kept existing)'
    fi

    # Install k3s manifest if k3s is running
    if [ -d /var/lib/rancher/k3s/server/manifests ]; then
        $SUDO cp deploy/k3s-vmrogue-web.yaml /var/lib/rancher/k3s/server/manifests/vmrogue-web.yaml
        echo 'k3s manifest: installed'
    fi

    # Restart service
    $SUDO systemctl enable vmrogue-web.service 2>/dev/null || true
    $SUDO systemctl restart vmrogue-web.service
" 2>&1
info "Binary and service installed"

# ── Verify ──
if $QUICK_MODE; then
    VERIFY_STEP=3
else
    VERIFY_STEP=5
fi
step "Step ${VERIFY_STEP}/${TOTAL_STEPS}: Verifying deployment"

sleep 2
_ssh "
    echo \"Binary:  \$(which vmrogue 2>/dev/null || echo NOT_FOUND)\"
    echo \"Version: \$(vmrogue --version 2>/dev/null || echo FAILED)\"
    echo \"Service: \$(systemctl is-active vmrogue-web 2>/dev/null || echo not-running)\"
    echo \"\"

    # Show service status
    systemctl status vmrogue-web --no-pager -l 2>/dev/null | head -10 || true

    # Check port
    echo \"\"
    if ss -tlnp 2>/dev/null | grep -q ':5151'; then
        echo 'Port 5151: listening'
    else
        echo 'Port 5151: not yet listening'
        echo 'Recent logs:'
        $SUDO journalctl -u vmrogue-web --no-pager -n 5 2>/dev/null || true
    fi
" 2>&1

echo ""
echo "  ============================================"
echo "  Deployment complete: ${USER}@${HOST}"
echo "  ============================================"
echo ""
echo "  Connect:"
echo "    ssh ${USER}@${HOST}"
echo ""
echo "  Web Dashboard:"
echo "    http://${HOST}:5151"
echo ""
echo "  Service management:"
echo "    sudo systemctl status vmrogue-web"
echo "    sudo journalctl -u vmrogue-web -f"
echo ""
