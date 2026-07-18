#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Join machines from a SECOND rack/site into the Veyron k3s cluster over a
# host-level WireGuard underlay (Tailscale alternative via V9S_MESH_BACKEND).
#
# Topology (see docs/MULTI_SITE_MESH.md):
#   - single control plane at site 1 (no etcd over WAN)
#   - site-2 machines join as WORKERS with their k3s --node-ip on the mesh
#   - VMs stay zone-pinned; live migration is within-site only; cross-site = DR
#
# Two modes:
#   --init-server <host> <user>   Run ONCE against the site-1 k3s server:
#                                 installs WireGuard, creates wg-veyron
#                                 (10.88.0.1/24 by default), re-binds k3s to the
#                                 mesh IP (--node-ip/--advertise-address via the
#                                 k3s config file), prints the join token.
#   <worker-host> <user>          Join one site-2 worker: WireGuard peer
#                                 exchange with the server over SSH, k3s agent
#                                 install (K3S_VERSION from versions.env),
#                                 zone + GPU labels.
#
# Env:
#   V9S_SERVER_HOST     site-1 server public IP/host (required for worker join)
#   V9S_SERVER_USER     ssh user on the server (default root)
#   V9S_MESH_CIDR       mesh subnet          (default 10.88.0.0/24, versions.env)
#   V9S_MESH_PORT       WireGuard UDP port   (default 51820, versions.env)
#   V9S_MESH_IP         this worker's mesh IP (default: next free .N)
#   V9S_MESH_BACKEND    wireguard (default) | tailscale
#   V9S_TAILSCALE_AUTHKEY  auth key when V9S_MESH_BACKEND=tailscale
#   V9S_SITE            zone label for site-2 (default site-2)
#   V9S_K3S_TOKEN       k3s join token (default: read from the server over SSH)
#   V9S_GPU_PREFLIGHT=1 also run the GPU preflight before joining
#   V9S_SKIP_PREFLIGHT=1 skip the KubeVirt node preflight — ONLY for mesh
#                       rehearsals on VMs (no /dev/kvm in a guest); real
#                       workers must pass preflight
#
# WireGuard MTU is pinned to 1420 (1500 - WG overhead); Cilium native routing
# rides the tunnel and inherits it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ -f "${SCRIPT_DIR}/versions.env" ]] && source "${SCRIPT_DIR}/versions.env"

G="\033[0;32m"; Y="\033[0;33m"; R="\033[0;31m"; N="\033[0m"
ok()   { printf "  ${G}✔${N} %s\n" "$*"; }
warn() { printf "  ${Y}!${N} %s\n" "$*"; }
bad()  { printf "  ${R}✘${N} %s\n" "$*"; exit 1; }

MESH_CIDR="${V9S_MESH_CIDR:-10.88.0.0/24}"
MESH_PORT="${V9S_MESH_PORT:-51820}"
MESH_BASE="${MESH_CIDR%.*}"          # 10.88.0
SERVER_MESH_IP="${MESH_BASE}.1"
WG_IF="wg-veyron"
BACKEND="${V9S_MESH_BACKEND:-wireguard}"

SSH_OPTS=(-o StrictHostKeyChecking=no -o ConnectTimeout=10)

# ─────────────────────────────────────────────────────────────────────────────
# --init-server: prepare the site-1 k3s server as the mesh hub.
# ─────────────────────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--init-server" ]]; then
  HOST="${2:?usage: $0 --init-server <host> <user>}"; USER_="${3:-root}"
  echo "Initializing mesh server on ${USER_}@${HOST} (${WG_IF} ${SERVER_MESH_IP}/24, udp/${MESH_PORT})"

  ssh "${SSH_OPTS[@]}" "${USER_}@${HOST}" bash -s <<EOF
set -euo pipefail
command -v wg >/dev/null 2>&1 || {
  if command -v apt-get >/dev/null 2>&1; then apt-get update -qq && apt-get install -y -qq wireguard-tools >/dev/null
  elif command -v dnf >/dev/null 2>&1; then dnf install -y -q wireguard-tools >/dev/null
  elif command -v yum >/dev/null 2>&1; then yum install -y -q wireguard-tools >/dev/null
  elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install wireguard-tools >/dev/null
  else echo "no known package manager to install wireguard-tools" >&2; exit 1; fi
}
umask 077
mkdir -p /etc/wireguard
if [[ ! -f /etc/wireguard/${WG_IF}.key ]]; then
  wg genkey | tee /etc/wireguard/${WG_IF}.key | wg pubkey > /etc/wireguard/${WG_IF}.pub
fi
if [[ ! -f /etc/wireguard/${WG_IF}.conf ]]; then
  cat > /etc/wireguard/${WG_IF}.conf <<CONF
[Interface]
Address = ${SERVER_MESH_IP}/24
ListenPort = ${MESH_PORT}
PrivateKey = \$(cat /etc/wireguard/${WG_IF}.key)
MTU = 1420
CONF
fi
systemctl enable --now wg-quick@${WG_IF} >/dev/null 2>&1 || systemctl restart wg-quick@${WG_IF}
# Re-bind k3s onto the mesh IP via the config file (survives k3s upgrades).
mkdir -p /etc/rancher/k3s
touch /etc/rancher/k3s/config.yaml
python3 - <<'PY'
import re
path = "/etc/rancher/k3s/config.yaml"
with open(path) as f:
    text = f.read()
def upsert(text, key, value):
    line = f"{key}: {value}"
    if re.search(rf"^{key}:", text, re.M):
        return re.sub(rf"^{key}:.*$", line, text, flags=re.M)
    return text + ("" if text.endswith("\n") or not text else "\n") + line + "\n"
public_ip = ""
try:
    import subprocess
    public_ip = subprocess.check_output(
        ["sh", "-c", "ip -4 route get 1.1.1.1 | awk '{print \$7; exit}'"], text=True).strip()
except Exception:
    pass
text = upsert(text, "node-ip", "${SERVER_MESH_IP}")
text = upsert(text, "advertise-address", "${SERVER_MESH_IP}")
if public_ip:
    text = upsert(text, "node-external-ip", public_ip)
with open(path, "w") as f:
    f.write(text)
PY
systemctl restart k3s
echo "--- server ready ---"
echo "PUBKEY=\$(cat /etc/wireguard/${WG_IF}.pub)"
echo "TOKEN=\$(cat /var/lib/rancher/k3s/server/node-token)"
EOF
  echo ""
  ok "Server initialized. Export the printed TOKEN as V9S_K3S_TOKEN (or let worker join read it over SSH)."
  echo "  Next: V9S_SERVER_HOST=${HOST} $0 <worker-host> <user>"
  exit 0
fi

# ─────────────────────────────────────────────────────────────────────────────
# Worker join.
# ─────────────────────────────────────────────────────────────────────────────
WORKER_HOST="${1:?usage: $0 [--init-server] <host> <user>   (see header)}"
WORKER_USER="${2:-root}"
SERVER_HOST="${V9S_SERVER_HOST:?set V9S_SERVER_HOST=<site-1 server public IP>}"
SERVER_USER="${V9S_SERVER_USER:-root}"
SITE="${V9S_SITE:-site-2}"
K3S_VER="${K3S_VERSION:-v1.35.6+k3s1}"

echo "Joining ${WORKER_USER}@${WORKER_HOST} as a ${SITE} worker (mesh backend: ${BACKEND})"
printf '%s\n' "────────────────────────────────────────────────"

# 0) Host preflight first — a worker that can't run VMs is not worth joining.
if [[ "${V9S_SKIP_PREFLIGHT:-0}" == "1" ]]; then
  warn "Skipping node preflight (V9S_SKIP_PREFLIGHT=1 — rehearsal mode; real workers must pass it)"
elif [[ -x "${SCRIPT_DIR}/preflight-node.sh" ]]; then
  PREFLIGHT_ARGS=()
  [[ "${V9S_GPU_PREFLIGHT:-0}" == "1" ]] && PREFLIGHT_ARGS+=(--gpu)
  "${SCRIPT_DIR}/preflight-node.sh" "${PREFLIGHT_ARGS[@]}" "${WORKER_HOST}" "${WORKER_USER}" \
    || bad "preflight failed on ${WORKER_HOST} — fix the node before joining"
fi

# 1) Mesh connectivity.
if [[ "${BACKEND}" == "tailscale" ]]; then
  [[ -n "${V9S_TAILSCALE_AUTHKEY:-}" ]] || bad "V9S_MESH_BACKEND=tailscale needs V9S_TAILSCALE_AUTHKEY"
  WORKER_MESH_IP="$(ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" bash -s <<EOF
set -euo pipefail
command -v tailscale >/dev/null 2>&1 || curl -fsSL https://tailscale.com/install.sh | sh >/dev/null
tailscale up --auth-key='${V9S_TAILSCALE_AUTHKEY}' >/dev/null
tailscale ip -4 | head -1
EOF
)"
  SERVER_MESH_IP="$(ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" "tailscale ip -4 | head -1")"
  ok "Tailscale mesh: worker ${WORKER_MESH_IP} ↔ server ${SERVER_MESH_IP}"
else
  # WireGuard: allocate the worker mesh IP (next free .N) unless pinned.
  WORKER_MESH_IP="${V9S_MESH_IP:-}"
  if [[ -z "${WORKER_MESH_IP}" ]]; then
    USED="$(ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" \
      "wg show ${WG_IF} allowed-ips 2>/dev/null | grep -o '${MESH_BASE}\.[0-9]*' | sort -u; echo ${SERVER_MESH_IP}")"
    for n in $(seq 2 254); do
      if ! grep -q "^${MESH_BASE}\.${n}$" <<< "${USED}"; then WORKER_MESH_IP="${MESH_BASE}.${n}"; break; fi
    done
  fi
  [[ -n "${WORKER_MESH_IP}" ]] || bad "no free mesh IP in ${MESH_CIDR}"
  ok "Worker mesh IP: ${WORKER_MESH_IP}"

  SERVER_PUB="$(ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" "cat /etc/wireguard/${WG_IF}.pub")"
  [[ -n "${SERVER_PUB}" ]] || bad "server has no ${WG_IF} keypair — run: $0 --init-server ${SERVER_HOST} ${SERVER_USER}"

  # Worker side: keys + config + peer(server).
  WORKER_PUB="$(ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" bash -s <<EOF
set -euo pipefail
command -v wg >/dev/null 2>&1 || {
  if command -v apt-get >/dev/null 2>&1; then apt-get update -qq && apt-get install -y -qq wireguard-tools >/dev/null
  elif command -v dnf >/dev/null 2>&1; then dnf install -y -q wireguard-tools >/dev/null
  elif command -v yum >/dev/null 2>&1; then yum install -y -q wireguard-tools >/dev/null
  elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install wireguard-tools >/dev/null
  else echo "no known package manager to install wireguard-tools" >&2; exit 1; fi
}
umask 077
mkdir -p /etc/wireguard
if [[ ! -f /etc/wireguard/${WG_IF}.key ]]; then
  wg genkey | tee /etc/wireguard/${WG_IF}.key | wg pubkey > /etc/wireguard/${WG_IF}.pub
fi
cat > /etc/wireguard/${WG_IF}.conf <<CONF
[Interface]
Address = ${WORKER_MESH_IP}/24
PrivateKey = \$(cat /etc/wireguard/${WG_IF}.key)
MTU = 1420

[Peer]
PublicKey = ${SERVER_PUB}
Endpoint = ${SERVER_HOST}:${MESH_PORT}
AllowedIPs = ${MESH_CIDR}
PersistentKeepalive = 25
CONF
systemctl enable --now wg-quick@${WG_IF} >/dev/null 2>&1 || systemctl restart wg-quick@${WG_IF}
cat /etc/wireguard/${WG_IF}.pub
EOF
)"

  # Server side: add/refresh the worker peer (idempotent per pubkey) + persist.
  ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" \
    "wg set ${WG_IF} peer '${WORKER_PUB}' allowed-ips ${WORKER_MESH_IP}/32 persistent-keepalive 25 && wg-quick save ${WG_IF} >/dev/null 2>&1 || true"
  ok "Peer exchange done (server ← ${WORKER_PUB:0:16}…)"

  # Handshake + MTU-sized ping through the tunnel.
  sleep 2
  if ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" "ping -c2 -W3 ${SERVER_MESH_IP} >/dev/null"; then
    ok "Mesh ping worker → server OK"
  else
    bad "worker cannot reach ${SERVER_MESH_IP} over ${WG_IF} — check udp/${MESH_PORT} reachability to ${SERVER_HOST}"
  fi
  if ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" "ping -c2 -W3 -M do -s 1392 ${SERVER_MESH_IP} >/dev/null"; then
    ok "1420-MTU path verified (1392 + headers)"
  else
    warn "Large packets do not fit — lower MTU in /etc/wireguard/${WG_IF}.conf on both ends (e.g. 1380)"
  fi
fi

# 2) k3s agent install, node-ip on the mesh.
TOKEN="${V9S_K3S_TOKEN:-}"
if [[ -z "${TOKEN}" ]]; then
  TOKEN="$(ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" "cat /var/lib/rancher/k3s/server/node-token")"
fi
[[ -n "${TOKEN}" ]] || bad "no k3s token (set V9S_K3S_TOKEN or make the server node-token readable)"

ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" bash -s <<EOF
set -euo pipefail
if systemctl is-active --quiet k3s-agent; then
  echo "k3s-agent already active — skipping install"
else
  curl -sfL https://get.k3s.io | INSTALL_K3S_VERSION="${K3S_VER}" \
    K3S_URL="https://${SERVER_MESH_IP}:6443" K3S_TOKEN="${TOKEN}" \
    sh -s - agent --node-ip "${WORKER_MESH_IP}"
fi
EOF
ok "k3s agent installed (${K3S_VER}, node-ip ${WORKER_MESH_IP})"

# 3) Labels: zone pinning (+ GPU role when requested).
NODE_NAME="$(ssh "${SSH_OPTS[@]}" "${WORKER_USER}@${WORKER_HOST}" hostname | tr '[:upper:]' '[:lower:]')"
LABEL_CMDS="kubectl label node ${NODE_NAME} topology.kubernetes.io/zone=${SITE} --overwrite"
[[ "${V9S_GPU_PREFLIGHT:-0}" == "1" ]] && LABEL_CMDS="${LABEL_CMDS} && kubectl label node ${NODE_NAME} veyron.io/gpu-passthrough=true --overwrite"
for _ in $(seq 1 24); do
  if ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" \
      "kubectl get node ${NODE_NAME} >/dev/null 2>&1 || k3s kubectl get node ${NODE_NAME} >/dev/null 2>&1"; then
    break
  fi
  sleep 5
done
ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" \
  "${LABEL_CMDS} 2>/dev/null || k3s ${LABEL_CMDS//kubectl/kubectl} 2>/dev/null || true"
ok "Node labeled: topology.kubernetes.io/zone=${SITE}"

printf '%s\n' "────────────────────────────────────────────────"
echo "Verify:"
echo "  ssh ${SERVER_USER}@${SERVER_HOST} kubectl get node ${NODE_NAME} -o wide   # Ready, INTERNAL-IP=${WORKER_MESH_IP}"
echo "  ssh ${SERVER_USER}@${SERVER_HOST} 'kubectl -n kube-system exec ds/cilium -- cilium status --brief'  # if Cilium CLI pod present"
echo "  wg show   (on either end: latest handshake < 2 min)"
echo ""
echo "Reminder: keep VMs zone-pinned (scheduling.node_selector topology.kubernetes.io/zone=${SITE});"
echo "live migration is within-site only — cross-site is DR (Velero/Atlas), see docs/MULTI_SITE_MESH.md."
