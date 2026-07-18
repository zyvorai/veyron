#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Configure KubeVirt to hand prepared GPUs to VMs:
#
#   1. Merge feature gates: GPU, HostDevices (merge, never replace)
#   2. Merge spec.configuration.permittedHostDevices.pciHostDevices entries for
#      each prepared vendor:device id (keyed on pciVendorSelector — idempotent,
#      existing entries survive)
#   3. Optional (V9S_INSTALL_NVIDIA_KUBEVIRT_DP=1): deploy the NVIDIA
#      kubevirt-gpu-device-plugin DaemonSet on veyron.io/gpu-passthrough=true
#      nodes — needed for the Phase-2 vGPU/mdev path, NOT for plain passthrough
#      (KubeVirt's built-in PCI host-device plugin handles that with zero
#      extra components)
#   4. Wait for a node to advertise the resource and print a create-VM example
#
# Prereq: run prepare-gpu-node.sh --apply on each GPU node + reboot first
# (verify with preflight-node.sh --gpu).
#
# DRY RUN BY DEFAULT — pass --apply to commit.
#
# Usage:
#   ./scripts/cluster/enable-gpu-passthrough.sh                        # dry-run
#   ./scripts/cluster/enable-gpu-passthrough.sh --apply
#   ./scripts/cluster/enable-gpu-passthrough.sh <host> <user> --apply  # over SSH
#
# Env:
#   V9S_GPU_PCI_IDS        comma list of vendor:device (default: auto-detect
#                          from this host's lspci — set explicitly when running
#                          against a remote cluster from a non-GPU machine)
#   V9S_GPU_RESOURCE_NAME  device-plugin resource name (default nvidia.com/gpu)
#   V9S_INSTALL_NVIDIA_KUBEVIRT_DP=1  also deploy the NVIDIA device plugin
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# versions.env is the single source of version truth (may be absent remotely).
[[ -f "${SCRIPT_DIR}/versions.env" ]] && source "${SCRIPT_DIR}/versions.env"

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
    "V9S_GPU_PCI_IDS='${V9S_GPU_PCI_IDS:-}' V9S_GPU_RESOURCE_NAME='${V9S_GPU_RESOURCE_NAME:-}' V9S_INSTALL_NVIDIA_KUBEVIRT_DP='${V9S_INSTALL_NVIDIA_KUBEVIRT_DP:-}' NVIDIA_KUBEVIRT_GPU_DP_IMAGE='${NVIDIA_KUBEVIRT_GPU_DP_IMAGE:-}' bash -s -- ${ARGS}" \
    < "${BASH_SOURCE[0]}"
fi

G="\033[0;32m"; Y="\033[0;33m"; R="\033[0;31m"; B="\033[0;34m"; N="\033[0m"
ok()   { printf "  ${G}✔${N} %s\n" "$*"; }
warn() { printf "  ${Y}!${N} %s\n" "$*"; }
bad()  { printf "  ${R}✘${N} %s\n" "$*"; }
plan() { printf "  ${B}→${N} %s\n" "$*"; }

# k3s ships kubectl behind the k3s binary.
if command -v kubectl >/dev/null 2>&1 && kubectl get nodes >/dev/null 2>&1; then
  K="kubectl"
elif sudo k3s kubectl get nodes >/dev/null 2>&1; then
  K="sudo k3s kubectl"
else
  bad "cannot reach a cluster (no working kubectl / k3s kubectl)"; exit 1
fi

run() {
  local desc="$1"; shift
  if [[ "${APPLY}" == "1" ]]; then
    if "$@" >/dev/null 2>&1; then ok "${desc}"; else bad "FAILED: ${desc}"; return 1; fi
  else
    plan "${desc}"
  fi
}

RESOURCE_NAME="${V9S_GPU_RESOURCE_NAME:-nvidia.com/gpu}"

echo "Veyron GPU passthrough enable → $($K config current-context 2>/dev/null || hostname)"
[[ "${APPLY}" == "1" ]] && echo "MODE: APPLY" || echo "MODE: dry-run (no changes; pass --apply to commit)"
printf '%s\n' "────────────────────────────────────────────────"

# ── 0. PCI ids ────────────────────────────────────────────────────────────────
IDS_CSV="${V9S_GPU_PCI_IDS:-}"
if [[ -z "${IDS_CSV}" ]] && command -v lspci >/dev/null 2>&1; then
  IDS_CSV="$(lspci -Dnn -d 10de: 2>/dev/null | sed -n 's/.*\[\(10de:[0-9a-f]\{4\}\)\].*/\1/p' | sort -u | paste -sd, -)"
fi
if [[ -z "${IDS_CSV}" ]]; then
  bad "No GPU PCI ids: set V9S_GPU_PCI_IDS=10de:xxxx[,10de:yyyy] (this host has no NVIDIA device to auto-detect)"
  exit 1
fi
ok "GPU PCI ids: ${IDS_CSV}"

# ── 1. Feature gates (merge, never replace) ───────────────────────────────────
echo; echo "1) KubeVirt feature gates"
WANT_GATES=(GPU HostDevices)
CUR_GATES_JSON="$($K get kubevirt kubevirt -n kubevirt -o jsonpath='{.spec.configuration.developerConfiguration.featureGates}' 2>/dev/null || echo '[]')"
[[ -z "${CUR_GATES_JSON}" ]] && CUR_GATES_JSON='[]'
MERGED="$(python3 - "${CUR_GATES_JSON}" "${WANT_GATES[@]}" <<'PY'
import sys, json
cur = json.loads(sys.argv[1] or "[]")
want = sys.argv[2:]
merged = list(dict.fromkeys(cur + [g for g in want if g not in cur]))
missing = [g for g in want if g not in cur]
print(json.dumps(merged))
print(",".join(missing))
PY
)"
MERGED_GATES="$(echo "${MERGED}" | sed -n 1p)"
MISSING_GATES="$(echo "${MERGED}" | sed -n 2p)"
if [[ -z "${MISSING_GATES}" ]]; then
  ok "Feature gates already enabled (GPU, HostDevices)"
else
  warn "Missing gates: ${MISSING_GATES}"
  run "merge feature gates -> ${MERGED_GATES}" \
    $K patch kubevirt kubevirt -n kubevirt --type merge \
      -p "{\"spec\":{\"configuration\":{\"developerConfiguration\":{\"featureGates\":${MERGED_GATES}}}}}"
fi

# ── 2. permittedHostDevices (merge keyed on pciVendorSelector) ────────────────
echo; echo "2) permittedHostDevices.pciHostDevices"
CUR_PCI_JSON="$($K get kubevirt kubevirt -n kubevirt -o jsonpath='{.spec.configuration.permittedHostDevices.pciHostDevices}' 2>/dev/null || echo '[]')"
[[ -z "${CUR_PCI_JSON}" ]] && CUR_PCI_JSON='[]'
MERGED_PCI="$(python3 - "${CUR_PCI_JSON}" "${IDS_CSV}" "${RESOURCE_NAME}" <<'PY'
import sys, json
cur = json.loads(sys.argv[1] or "[]")
ids = [i.strip() for i in sys.argv[2].split(",") if i.strip()]
resource = sys.argv[3]
have = {e.get("pciVendorSelector", "").upper() for e in cur}
added = []
for pci_id in ids:
    sel = pci_id.upper()
    if sel in have:
        continue
    cur.append({"pciVendorSelector": sel, "resourceName": resource, "externalResourceProvider": False})
    added.append(sel)
print(json.dumps(cur))
print(",".join(added))
PY
)"
MERGED_PCI_JSON="$(echo "${MERGED_PCI}" | sed -n 1p)"
ADDED_PCI="$(echo "${MERGED_PCI}" | sed -n 2p)"
echo "  current: ${CUR_PCI_JSON}"
if [[ -z "${ADDED_PCI}" ]]; then
  ok "All ids already permitted"
else
  warn "Adding: ${ADDED_PCI} -> ${RESOURCE_NAME}"
  run "merge permittedHostDevices" \
    $K patch kubevirt kubevirt -n kubevirt --type merge \
      -p "{\"spec\":{\"configuration\":{\"permittedHostDevices\":{\"pciHostDevices\":${MERGED_PCI_JSON}}}}}"
fi

# ── 3. Optional NVIDIA kubevirt-gpu-device-plugin (Phase-2 / vGPU prep) ──────
if [[ "${V9S_INSTALL_NVIDIA_KUBEVIRT_DP:-0}" == "1" ]]; then
  echo; echo "3) NVIDIA kubevirt-gpu-device-plugin"
  DP_IMAGE="${NVIDIA_KUBEVIRT_GPU_DP_IMAGE:-nvcr.io/nvidia/kubevirt-gpu-device-plugin:v1.3.1}"
  plan "apply DaemonSet nvidia-kubevirt-gpu-dp (image ${DP_IMAGE}, nodeSelector veyron.io/gpu-passthrough=true)"
  if [[ "${APPLY}" == "1" ]]; then
    cat <<EOF | $K apply -f - >/dev/null && ok "device plugin applied" || bad "device plugin apply failed"
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: nvidia-kubevirt-gpu-dp
  namespace: kube-system
  labels: { app: nvidia-kubevirt-gpu-dp }
spec:
  selector: { matchLabels: { app: nvidia-kubevirt-gpu-dp } }
  template:
    metadata:
      labels: { app: nvidia-kubevirt-gpu-dp }
    spec:
      nodeSelector: { veyron.io/gpu-passthrough: "true" }
      priorityClassName: system-node-critical
      tolerations: [{ operator: Exists }]
      containers:
        - name: device-plugin
          image: ${DP_IMAGE}
          securityContext: { privileged: true }
          volumeMounts:
            - { name: device-plugin, mountPath: /var/lib/kubelet/device-plugins }
      volumes:
        - name: device-plugin
          hostPath: { path: /var/lib/kubelet/device-plugins }
EOF
  fi
fi

# ── 4. Verify + how-to ───────────────────────────────────────────────────────
printf '%s\n' "────────────────────────────────────────────────"
if [[ "${APPLY}" == "1" ]]; then
  echo "Waiting up to 120s for a node to advertise ${RESOURCE_NAME} ..."
  for _ in $(seq 1 24); do
    TOTAL="$($K get nodes -o jsonpath="{range .items[*]}{.status.allocatable['${RESOURCE_NAME//./\\.}']}{'\n'}{end}" 2>/dev/null | awk '{s+=$1} END {print s+0}')"
    [[ "${TOTAL:-0}" -gt 0 ]] && break
    sleep 5
  done
  if [[ "${TOTAL:-0}" -gt 0 ]]; then
    ok "Nodes advertise ${TOTAL} × ${RESOURCE_NAME}"
  else
    warn "No node advertises ${RESOURCE_NAME} yet — virt-handler re-scans after the KubeVirt CR change; check 'kubectl get nodes -o json | jq .items[].status.allocatable'"
  fi
fi
echo
echo "Create a GPU VM through the Veyron API:"
echo '  curl -sk -X POST https://<host>:30151/api/v1/vms -H "X-API-Key: $VEYRON_API_KEY" \'
echo '    -H "Content-Type: application/json" \'
echo "    -d '{\"name\":\"gpu-vm\",\"template\":\"ubuntu22\",\"gpu\":{\"count\":1,\"resource_name\":\"${RESOURCE_NAME}\"}}'"
echo
echo "Note: passthrough GPU VMs can NEVER live-migrate (the API refuses with a 409"
echo "and suggests a stop/start cold move). See docs/GPU_PASSTHROUGH.md."
