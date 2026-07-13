#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Host-level virtualization preflight for a KubeVirt node. The API-level gate
# (preflight-veyron-remote.sh) can only see what Kubernetes reports; this checks
# the things that make a node capable of running VMs at all.
#
# Run ON the node, or remotely:
#   ./scripts/cluster/preflight-node.sh                 # local
#   ./scripts/cluster/preflight-node.sh <host> <user>   # over SSH
#
# Exits non-zero on any hard failure (no KVM, no hardware virt, swap on).
set -uo pipefail

# Remote mode: ship this script to the node and run it there.
if [[ $# -ge 1 ]]; then
  HOST="$1"; USER_="${2:-root}"
  exec ssh -o StrictHostKeyChecking=no "${USER_}@${HOST}" 'bash -s' < "${BASH_SOURCE[0]}"
fi

G="\033[0;32m"; Y="\033[0;33m"; R="\033[0;31m"; N="\033[0m"
FAILED=0
ok()   { printf "  ${G}✔${N} %s\n" "$*"; }
warn() { printf "  ${Y}!${N} %s\n" "$*"; }
bad()  { printf "  ${R}✘${N} %s\n" "$*"; FAILED=1; }

echo "KubeVirt node preflight → $(hostname)"
printf '%s\n' "────────────────────────────────────────────────"

# 1) Hardware virtualization extensions (Intel VT-x / AMD-V).
VIRT_CORES="$(grep -c -E '(vmx|svm)' /proc/cpuinfo 2>/dev/null || echo 0)"
if [[ "${VIRT_CORES}" -gt 0 ]]; then
  ok "Hardware virtualization: ${VIRT_CORES} core(s) with vmx/svm"
else
  bad "No vmx/svm CPU flags — hardware virtualization is off (check BIOS) or this is a nested guest without it"
fi

# 2) The three device nodes KubeVirt's own install checks require.
for dev in /dev/kvm /dev/vhost-net /dev/net/tun; do
  if [[ -e "${dev}" ]]; then
    ok "${dev} present"
  else
    bad "${dev} missing"
  fi
done
# Present-but-unreadable is a distinct (and common) failure.
if [[ -e /dev/kvm && ! -r /dev/kvm && "$(id -u)" != "0" ]]; then
  warn "/dev/kvm not readable by $(id -un) — fine if virt-handler runs privileged, check if VMs fail to start"
fi

# 3) Kernel modules.
for mod in kvm vhost_net tun; do
  if lsmod 2>/dev/null | grep -q "^${mod}\b"; then
    ok "module ${mod} loaded"
  else
    warn "module ${mod} not in lsmod (may be built into the kernel)"
  fi
done

# 4) libvirt's own validator — the authoritative check. Advisory when absent.
if command -v virt-host-validate >/dev/null 2>&1; then
  echo "  virt-host-validate qemu:"
  # Only the checks that actually gate KubeVirt; the cgroup/IOMMU ones are noisy.
  if virt-host-validate qemu 2>/dev/null | grep -E 'hardware virtualization|/dev/kvm|/dev/vhost-net|/dev/net/tun' | sed 's/^/    /'; then :; fi
  if virt-host-validate qemu 2>/dev/null | grep -E 'hardware virtualization|/dev/kvm' | grep -qi fail; then
    bad "virt-host-validate reports a FAIL on a KubeVirt-critical check"
  fi
else
  warn "virt-host-validate not installed (apt-get install -y libvirt-clients) — skipping authoritative check"
fi

# 5) Swap must be off for kubelet.
if [[ "$(swapon --show --noheadings 2>/dev/null | wc -l)" -gt 0 ]]; then
  bad "Swap is ON — disable it (swapoff -a; comment the swap line in /etc/fstab)"
else
  ok "Swap is off"
fi

# 6) Time sync — Windows guests and Kerberos/AD join are sensitive to drift.
if command -v chronyc >/dev/null 2>&1 && chronyc tracking >/dev/null 2>&1; then
  ok "chrony is tracking a time source"
elif timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -q yes; then
  ok "systemd-timesyncd is synchronized"
else
  warn "No confirmed time sync — install chrony (Windows domain join fails on clock drift)"
fi

printf '%s\n' "────────────────────────────────────────────────"
if [[ "${FAILED}" == "1" ]]; then
  printf "${R}✘ Node is NOT ready to run KubeVirt VMs.${N}\n"
  exit 1
fi
printf "${G}✔ Node preflight passed.${N}\n"
