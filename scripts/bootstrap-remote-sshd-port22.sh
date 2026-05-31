#!/usr/bin/env bash
# ============================================================================
# bootstrap-remote-sshd-port22.sh — Ensure sshd listens on 0.0.0.0:22 and firewalld allows SSH
# ============================================================================
# Run on the remote host via out-of-band console (KVM/IPMI/hosting panel) when
# SSH port 22 is blocked or sshd is not listening externally.
#
# Usage (on the remote machine as root or sudo):
#   curl -fsSL https://raw.githubusercontent.com/.../bootstrap-remote-sshd-port22.sh | sudo bash
#   sudo bash bootstrap-remote-sshd-port22.sh
# ============================================================================

set -euo pipefail

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "Run as root: sudo $0" >&2
    exit 1
fi

echo "==> sshd drop-in: Port 22, listen on all interfaces"
mkdir -p /etc/ssh/sshd_config.d
cat >/etc/ssh/sshd_config.d/99-vmrogue.conf <<'EOF'
Port 22
ListenAddress 0.0.0.0
ListenAddress ::
EOF

echo "==> Validate sshd_config"
sshd -t

echo "==> Enable and restart sshd"
systemctl enable sshd
systemctl restart sshd

echo "==> Open SSH in firewalld (if present)"
if command -v firewall-cmd >/dev/null 2>&1 && systemctl is-active firewalld >/dev/null 2>&1; then
    firewall-cmd --permanent --add-service=ssh
    firewall-cmd --reload
    firewall-cmd --list-services
fi

echo "==> Open SSH in ufw (if present)"
if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -qE 'Status: active'; then
    ufw allow 22/tcp
    ufw reload
fi

echo "==> Listening sockets on :22"
ss -tlnp | grep ':22' || { echo "ERROR: nothing listening on port 22" >&2; exit 1; }

echo "==> Done. Test from your laptop:"
echo "    ssh -p 22 \${USER}@\$(hostname -I | awk '{print \$1}') 'echo ok'"
