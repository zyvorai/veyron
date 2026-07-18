#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Prepare a host for NVIDIA GPU **PCI passthrough** into KubeVirt VMs:
#
#   1. Detect NVIDIA GPU functions (GPU + audio/USB siblings on the same card)
#   2. Verify IOMMU is enabled (BIOS VT-d/AMD-Vi + kernel cmdline) — prints the
#      exact remediation but NEVER edits the bootloader itself
#   3. Verify IOMMU group isolation (a group is passed through whole)
#   4. --apply: bind the GPU functions to vfio-pci at boot via modprobe.d
#      softdeps (so removing one file later re-enables the host driver for
#      a future vGPU setup) and stage the module load
#
# DRY RUN BY DEFAULT — prints the plan and exits. Pass --apply to commit.
# A REBOOT is required after --apply (initramfs + driver rebind).
#
# Usage:
#   ./scripts/cluster/prepare-gpu-node.sh                        # local, dry-run
#   ./scripts/cluster/prepare-gpu-node.sh --apply                # local, commit
#   ./scripts/cluster/prepare-gpu-node.sh <host> <user> --apply  # over SSH
#
# Env:
#   V9S_GPU_PCI_IDS       comma list of vendor:device ids (default: auto-detect
#                         every NVIDIA function, including audio/USB siblings)
#   V9S_GPU_INCLUDE_AUDIO 1 (default) to include same-card non-GPU functions
#
# After the reboot: verify with `preflight-node.sh --gpu`, then run
# enable-gpu-passthrough.sh to configure KubeVirt permittedHostDevices.
set -uo pipefail

APPLY=0
POSARGS=()
for a in "$@"; do
  case "$a" in
    --apply) APPLY=1 ;;
    --dry-run) APPLY=0 ;;
    *) POSARGS+=("$a") ;;
  esac
done
set -- "${POSARGS[@]:-}"

# Remote mode: re-exec this script on the node.
if [[ -n "${1:-}" ]]; then
  HOST="$1"; USER_="${2:-root}"
  ARGS=""; [[ "${APPLY}" == "1" ]] && ARGS="--apply"
  exec ssh -o StrictHostKeyChecking=no "${USER_}@${HOST}" \
    "V9S_GPU_PCI_IDS='${V9S_GPU_PCI_IDS:-}' V9S_GPU_INCLUDE_AUDIO='${V9S_GPU_INCLUDE_AUDIO:-1}' bash -s -- ${ARGS}" \
    < "${BASH_SOURCE[0]}"
fi

G="\033[0;32m"; Y="\033[0;33m"; R="\033[0;31m"; B="\033[0;34m"; N="\033[0m"
ok()   { printf "  ${G}✔${N} %s\n" "$*"; }
warn() { printf "  ${Y}!${N} %s\n" "$*"; }
bad()  { printf "  ${R}✘${N} %s\n" "$*"; }
plan() { printf "  ${B}→${N} %s\n" "$*"; }

INCLUDE_AUDIO="${V9S_GPU_INCLUDE_AUDIO:-1}"
MODPROBE_FILE="/etc/modprobe.d/vfio-veyron.conf"
MODLOAD_FILE="/etc/modules-load.d/vfio-veyron.conf"

echo "Veyron GPU node prep → $(hostname)"
[[ "${APPLY}" == "1" ]] && echo "MODE: APPLY (files will be written; reboot required)" || echo "MODE: dry-run (no changes; pass --apply to commit)"
printf '%s\n' "────────────────────────────────────────────────"

# ── 1. Detect NVIDIA functions ───────────────────────────────────────────────
if ! command -v lspci >/dev/null 2>&1; then
  bad "lspci not found (apt-get install -y pciutils)"; exit 1
fi

# 0000:01:00.0 ... [10de:2684] — capture address + id for every NVIDIA function.
NVIDIA_LINES="$(lspci -Dnn -d 10de: 2>/dev/null || true)"
if [[ -z "${NVIDIA_LINES}" ]]; then
  bad "No NVIDIA PCI device found (lspci -d 10de:) — nothing to prepare"; exit 1
fi

GPU_ADDRS=()
GPU_IDS=()
while IFS= read -r line; do
  addr="${line%% *}"
  id="$(sed -n 's/.*\[\(10de:[0-9a-f]\{4\}\)\].*/\1/p' <<< "${line}")"
  is_vga=0
  grep -qiE 'VGA compatible|3D controller' <<< "${line}" && is_vga=1
  if [[ "${is_vga}" == "1" || "${INCLUDE_AUDIO}" == "1" ]]; then
    GPU_ADDRS+=("${addr}")
    GPU_IDS+=("${id}")
    ok "NVIDIA function ${addr} [${id}]: $(sed 's/^[^ ]* //' <<< "${line}")"
  else
    warn "Skipping non-GPU function ${addr} [${id}] (V9S_GPU_INCLUDE_AUDIO=0)"
  fi
done <<< "${NVIDIA_LINES}"

# User override wins over auto-detection.
if [[ -n "${V9S_GPU_PCI_IDS:-}" ]]; then
  IFS=',' read -r -a GPU_IDS <<< "${V9S_GPU_PCI_IDS}"
  ok "Using V9S_GPU_PCI_IDS override: ${V9S_GPU_PCI_IDS}"
fi
# De-duplicate ids (a dual-GPU host repeats the same vendor:device).
mapfile -t GPU_IDS < <(printf '%s\n' "${GPU_IDS[@]}" | awk 'NF && !seen[$0]++')
IDS_CSV="$(IFS=','; echo "${GPU_IDS[*]}")"

# ── 2. IOMMU state ───────────────────────────────────────────────────────────
IOMMU_GROUPS="$(find /sys/kernel/iommu_groups -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l)"
CPU_VENDOR="$(grep -m1 vendor_id /proc/cpuinfo | awk '{print $3}')"
if [[ "${IOMMU_GROUPS}" -gt 0 ]]; then
  ok "IOMMU active: ${IOMMU_GROUPS} groups"
else
  bad "IOMMU is NOT active. Fix manually (this script never edits the bootloader):"
  if [[ "${CPU_VENDOR}" == "GenuineIntel" ]]; then
    echo "      1. BIOS/UEFI: enable VT-d"
    echo "      2. Add to the kernel cmdline:  intel_iommu=on iommu=pt"
  else
    echo "      1. BIOS/UEFI: enable AMD-Vi / IOMMU"
    echo "      2. Add to the kernel cmdline:  amd_iommu=on iommu=pt"
  fi
  echo "         GRUB:      edit GRUB_CMDLINE_LINUX in /etc/default/grub, then:"
  echo "                    update-grub   (Debian/Ubuntu)  |  grub2-mkconfig -o /boot/grub2/grub.cfg  (RHEL/Fedora)"
  echo "         systemd-boot: append to the options line in /boot/loader/entries/*.conf"
  echo "      3. Reboot and re-run this script"
  exit 1
fi

# ── 3. IOMMU group isolation ─────────────────────────────────────────────────
ISOLATION_WARN=0
for addr in "${GPU_ADDRS[@]}"; do
  group_link="/sys/bus/pci/devices/${addr}/iommu_group"
  [[ -e "${group_link}" ]] || continue
  group="$(basename "$(readlink -f "${group_link}")")"
  slot="${addr%.*}"
  foreign="$(ls "/sys/kernel/iommu_groups/${group}/devices/" 2>/dev/null | grep -v "^${slot}" || true)"
  if [[ -z "${foreign}" ]]; then
    ok "${addr} IOMMU group ${group} is isolated"
  else
    warn "${addr} shares IOMMU group ${group} with: $(echo "${foreign}" | tr '\n' ' ')"
    ISOLATION_WARN=1
  fi
done
if [[ "${ISOLATION_WARN}" == "1" ]]; then
  warn "Shared groups are passed through WHOLE. Try another PCIe slot first."
  warn "ACS override patches exist but weaken isolation — see docs/GPU_PASSTHROUGH.md before considering them."
fi

# ── 4. vfio-pci binding plan / apply ─────────────────────────────────────────
printf '%s\n' "────────────────────────────────────────────────"
echo "vfio-pci binding for ids: ${IDS_CSV}"
plan "write ${MODPROBE_FILE}:"
echo "        options vfio-pci ids=${IDS_CSV}"
echo "        softdep nouveau pre: vfio-pci"
echo "        softdep nvidia pre: vfio-pci"
echo "        softdep snd_hda_intel pre: vfio-pci"
plan "write ${MODLOAD_FILE}:  vfio-pci"

if [[ "${APPLY}" == "1" ]]; then
  if [[ "$(id -u)" != "0" ]]; then SUDO="sudo"; else SUDO=""; fi
  ${SUDO} tee "${MODPROBE_FILE}" >/dev/null <<EOF
# Managed by Veyron prepare-gpu-node.sh — bind NVIDIA functions to vfio-pci for
# KubeVirt PCI passthrough. Delete this file (and regen initramfs) to give the
# GPU back to host drivers (e.g. for an NVIDIA vGPU host-driver setup).
options vfio-pci ids=${IDS_CSV}
softdep nouveau pre: vfio-pci
softdep nvidia pre: vfio-pci
softdep snd_hda_intel pre: vfio-pci
EOF
  ${SUDO} tee "${MODLOAD_FILE}" >/dev/null <<EOF
vfio-pci
EOF
  ok "wrote ${MODPROBE_FILE} + ${MODLOAD_FILE}"

  if command -v update-initramfs >/dev/null 2>&1; then
    ${SUDO} update-initramfs -u >/dev/null 2>&1 && ok "initramfs regenerated (update-initramfs -u)" || warn "update-initramfs failed — run it manually"
  elif command -v dracut >/dev/null 2>&1; then
    ${SUDO} dracut -f >/dev/null 2>&1 && ok "initramfs regenerated (dracut -f)" || warn "dracut failed — run it manually"
  else
    warn "No update-initramfs/dracut found — regenerate the initramfs manually"
  fi
  printf '%s\n' "────────────────────────────────────────────────"
  printf "${Y}! REBOOT REQUIRED${N} — then verify: ./scripts/cluster/preflight-node.sh --gpu\n"
  echo "  Next: ./scripts/cluster/enable-gpu-passthrough.sh  (KubeVirt permittedHostDevices)"
else
  printf '%s\n' "────────────────────────────────────────────────"
  echo "Dry-run complete. Pass --apply to write the files (reboot required after)."
fi
