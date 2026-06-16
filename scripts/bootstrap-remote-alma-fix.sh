#!/usr/bin/env bash
# AlmaLinux remote fix: sshd on :22, iptables (no firewalld), VMRogue NodePorts.
set -euo pipefail

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "Run as root: sudo $0" >&2
    exit 1
fi

echo "==> sshd: listen on 0.0.0.0:22"
mkdir -p /etc/ssh/sshd_config.d
cat >/etc/ssh/sshd_config.d/99-vmrogue.conf <<'EOF'
Port 22
ListenAddress 0.0.0.0
ListenAddress ::
EOF
sshd -t
systemctl enable sshd
systemctl restart sshd

echo "==> iptables: allow SSH + VMRogue NodePorts (Alma without firewalld)"
for port in 22 30150 30151; do
    iptables -C INPUT -p tcp --dport "${port}" -j ACCEPT 2>/dev/null \
        || iptables -I INPUT -p tcp --dport "${port}" -j ACCEPT
done

echo "==> disable legacy iptables service (nft/cilium rules remain)"
systemctl disable iptables --now 2>/dev/null || true

echo "==> listening"
ss -tlnp | grep -E ':22|:3015[01]' || ss -tlnp | grep ':22'

echo "==> k3s / vmrogue pods"
K="/usr/local/bin/k3s kubectl"
if [[ -x /usr/local/bin/k3s ]]; then
    ${K} get nodes 2>/dev/null || true
    ${K} -n veyron-system get pods 2>/dev/null || true
fi

echo "==> done"
