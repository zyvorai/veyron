#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Host-level virtualization preflight for a KubeVirt node. The API-level gate
# (preflight-veyron-remote.sh) can only see what Kubernetes reports; this checks
# the things that make a node capable of running VMs at all.
#
# Run ON the node, or remotely:
#   ./scripts/cluster/preflight-node.sh                       # local
#   ./scripts/cluster/preflight-node.sh --gpu                 # + GPU/IOMMU section
#   ./scripts/cluster/preflight-node.sh [--gpu] <host> <user> # over SSH
#
# The GPU/IOMMU section is opt-in (--gpu, or V9S_GPU_PREFLIGHT=1) so plain runs
# stay identical to the non-GPU output. Prep a node with prepare-gpu-node.sh.
#
# Exits non-zero on any hard failure (no KVM, no hardware virt, swap on).
set -uo pipefail

# Opt-in GPU section. Parse the flag BEFORE remote mode — ssh 'bash -s' does not
# forward env or argv, so the flag must be re-injected into the remote command.
GPU_PREFLIGHT="${V9S_GPU_PREFLIGHT:-0}"
if [[ "${1:-}" == "--gpu" ]]; then
  GPU_PREFLIGHT=1
  shift
fi

# Remote mode: ship this script to the node and run it there.
if [[ $# -ge 1 ]]; then
  HOST="$1"; USER_="${2:-root}"
  exec ssh -o StrictHostKeyChecking=no "${USER_}@${HOST}" \
    "V9S_GPU_PREFLIGHT=${GPU_PREFLIGHT} bash -s" < "${BASH_SOURCE[0]}"
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

# 7) GPU / IOMMU passthrough readiness (opt-in: --gpu or V9S_GPU_PREFLIGHT=1).
if [[ "${GPU_PREFLIGHT}" == "1" ]]; then
  echo ""
  echo "GPU passthrough preflight"
  printf '%s\n' "────────────────────────────────────────────────"

  # 7a) NVIDIA PCI functions on this host (GPU + audio/USB siblings).
  NVIDIA_FUNCS="$(lspci -Dnn -d 10de: 2>/dev/null || true)"
  if [[ -z "${NVIDIA_FUNCS}" ]]; then
    bad "No NVIDIA PCI device found (lspci -d 10de:) — nothing to pass through"
  else
    while IFS= read -r line; do
      ok "NVIDIA function: ${line}"
    done <<< "${NVIDIA_FUNCS}"
  fi

  # 7b) IOMMU enabled: groups populated + kernel cmdline flag.
  IOMMU_GROUPS="$(find /sys/kernel/iommu_groups -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l)"
  if [[ "${IOMMU_GROUPS}" -gt 0 ]]; then
    ok "IOMMU active: ${IOMMU_GROUPS} groups"
  else
    bad "No IOMMU groups — enable VT-d/AMD-Vi in BIOS and add intel_iommu=on iommu=pt (or amd_iommu=on) to the kernel cmdline, then reboot"
  fi
  if grep -qE 'intel_iommu=on|amd_iommu=on' /proc/cmdline 2>/dev/null; then
    ok "Kernel cmdline has an iommu=on flag"
  else
    warn "Kernel cmdline lacks intel_iommu=on/amd_iommu=on (AMD often enables it by default; groups above are what counts)"
  fi

  # 7c) Every NVIDIA function must be bound to vfio-pci, not nouveau/nvidia.
  if [[ -n "${NVIDIA_FUNCS}" ]]; then
    while IFS= read -r line; do
      addr="${line%% *}"
      drv_path="/sys/bus/pci/devices/${addr}/driver"
      drv="$( [[ -e "${drv_path}" ]] && basename "$(readlink -f "${drv_path}")" || echo none )"
      case "${drv}" in
        vfio-pci) ok "${addr} bound to vfio-pci" ;;
        nouveau|nvidia) warn "${addr} bound to ${drv} — run prepare-gpu-node.sh --apply and reboot to hand it to vfio-pci" ;;
        none) warn "${addr} has no driver (acceptable pre-KubeVirt; vfio-pci will claim it if configured)" ;;
        *) warn "${addr} bound to unexpected driver '${drv}'" ;;
      esac
    done <<< "${NVIDIA_FUNCS}"
  fi

  # 7d) IOMMU group isolation: passthrough hands the whole group to the VM.
  if [[ -n "${NVIDIA_FUNCS}" && "${IOMMU_GROUPS}" -gt 0 ]]; then
    while IFS= read -r line; do
      addr="${line%% *}"
      group_link="/sys/bus/pci/devices/${addr}/iommu_group"
      [[ -e "${group_link}" ]] || continue
      group="$(basename "$(readlink -f "${group_link}")")"
      others="$(ls "/sys/kernel/iommu_groups/${group}/devices/" 2>/dev/null | grep -v "${addr}" || true)"
      # Same-slot sibling functions (audio/USB on the same card) are fine.
      slot="${addr%.*}"
      foreign="$(printf '%s\n' "${others}" | grep -v "^${slot}" || true)"
      if [[ -z "${foreign}" ]]; then
        ok "${addr} IOMMU group ${group} is isolated (same-card functions only)"
      else
        warn "${addr} shares IOMMU group ${group} with: $(echo "${foreign}" | tr '\n' ' ')— passthrough drags those along (ACS limitation)"
      fi
    done <<< "${NVIDIA_FUNCS}"
  fi
fi

printf '%s\n' "────────────────────────────────────────────────"
if [[ "${FAILED}" == "1" ]]; then
  printf "${R}✘ Node is NOT ready to run KubeVirt VMs.${N}\n"
  exit 1
fi
printf "${G}✔ Node preflight passed.${N}\n"
