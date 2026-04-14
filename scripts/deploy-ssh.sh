#!/usr/bin/env bash
# VMRogue SSH Deploy Script
# Builds, uploads, and deploys VMRogue to a remote server via SSH
#
# Usage: ./scripts/deploy-ssh.sh [host] [user]
#   host  Remote host (default: HOST)
#   user  SSH user (default: sus)

set -euo pipefail

HOST="${1:-HOST}"
USER="${2:-sus}"
REMOTE="${USER}@${HOST}"
BINARY="vmrogue"
REMOTE_BIN="/usr/local/bin/${BINARY}"
REMOTE_SERVICE="vmrogue-web"
SERVICE_FILE="/etc/systemd/system/${REMOTE_SERVICE}.service"

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

echo -e "${BOLD}${CYAN}"
echo "  ╦  ╦╔╦╗╦═╗╔═╗╔═╗╦ ╦╔═╗"
echo "  ╚╗╔╝║║║╠╦╝║ ║║ ╦║ ║║╣ "
echo "   ╚╝ ╩ ╩╩╚═╚═╝╚═╝╚═╝╚═╝"
echo -e "${NC}${BOLD}  SSH Deploy → ${REMOTE}${NC}"
echo ""

# Step 1: Build release binary
log "Building release binary..."
cargo build --release --locked 2>&1 | tail -3
strip target/release/${BINARY} 2>/dev/null || true
SIZE=$(du -h target/release/${BINARY} | cut -f1)
ok "Binary built: ${SIZE}"

# Step 2: Test SSH connectivity
log "Testing SSH connection to ${REMOTE}..."
if ! ssh -o ConnectTimeout=10 -o BatchMode=yes "${REMOTE}" "echo ok" &>/dev/null; then
    err "Cannot connect to ${REMOTE}"
    err "Make sure SSH key auth is set up: ssh-copy-id ${REMOTE}"
    exit 1
fi
ok "SSH connection verified"

# Step 3: Upload binary
log "Uploading binary to ${REMOTE}:${REMOTE_BIN}..."
scp -o ConnectTimeout=10 "target/release/${BINARY}" "${REMOTE}:/tmp/${BINARY}"
ssh "${REMOTE}" "sudo mv /tmp/${BINARY} ${REMOTE_BIN} && sudo chmod 755 ${REMOTE_BIN}"
ok "Binary deployed to ${REMOTE_BIN}"

# Step 4: Install systemd service
log "Installing systemd service..."
ssh "${REMOTE}" "sudo tee ${SERVICE_FILE} > /dev/null" <<'UNIT'
[Unit]
Description=VMRogue Web Dashboard
After=network-online.target k3s.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/vmrogue api-serve --port 5151 --host 0.0.0.0
Restart=on-failure
RestartSec=5
Environment=KUBECONFIG=/etc/rancher/k3s/k3s.yaml
EnvironmentFile=-/etc/vmrogue/env
DynamicUser=yes
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
RestrictNamespaces=yes

[Install]
WantedBy=multi-user.target
UNIT
ok "Service file installed"

# Step 5: Reload and restart
log "Restarting service..."
ssh "${REMOTE}" "sudo systemctl daemon-reload && sudo systemctl enable ${REMOTE_SERVICE} && sudo systemctl restart ${REMOTE_SERVICE}"
ok "Service restarted"

# Step 6: Verify
log "Verifying deployment..."
sleep 2
STATUS=$(ssh "${REMOTE}" "systemctl is-active ${REMOTE_SERVICE}" 2>/dev/null || echo "unknown")
if [ "${STATUS}" = "active" ]; then
    ok "Service is running"
else
    warn "Service status: ${STATUS}"
    ssh "${REMOTE}" "sudo journalctl -u ${REMOTE_SERVICE} -n 10 --no-pager" 2>/dev/null || true
fi

VERSION=$(ssh "${REMOTE}" "${REMOTE_BIN} --version" 2>/dev/null || echo "unknown")
echo ""
echo -e "${GREEN}${BOLD}  Deploy complete${NC}"
echo -e "  ${CYAN}Host:${NC}    ${REMOTE}"
echo -e "  ${CYAN}Binary:${NC}  ${REMOTE_BIN}"
echo -e "  ${CYAN}Version:${NC} ${VERSION}"
echo -e "  ${CYAN}Status:${NC}  ${STATUS}"
echo -e "  ${CYAN}URL:${NC}    http://${HOST}:5151"
echo ""
