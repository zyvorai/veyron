#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Bring an EXISTING cluster up to the Veyron production baseline, in place.
#
# For clusters that already run k3s + Cilium + KubeVirt + CDI (so the greenfield
# installer would be destructive), but whose *configuration* still blocks
# production VM workloads. It fixes the gaps the acceptance gate flags:
#
#   1. Default StorageClass is node-local (local-path)  -> repoint to shared storage
#   2. KubeVirt feature gates missing                   -> merge in the Day-2 gates
#   3. vmStateStorageClass unset                        -> persistent TPM/EFI (Windows 11)
#   4. No VolumeSnapshotClass for the filesystem driver -> add one
#   5. containerd denies non-root access to Block PVCs  -> enable nonroot-devices (k3s)
#
# DRY RUN BY DEFAULT — prints the exact changes and exits. Pass --apply to commit.
# Idempotent: safe to re-run.
#
# Usage:
#   ./scripts/cluster/adapt-existing-cluster.sh                      # local, dry-run
#   ./scripts/cluster/adapt-existing-cluster.sh --apply              # local, commit
#   ./scripts/cluster/adapt-existing-cluster.sh <host> <user> --apply  # over SSH
#
# Override the chosen classes:
#   VEYRON_BLOCK_SC=zyvor-rbd-prod VEYRON_FS_SC=zyvor-cephfs-shared ...
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
    "APPLY_ARGS='${ARGS}' bash -s -- ${ARGS}" < "${BASH_SOURCE[0]}"
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

CHANGES=0
run() {
  # run <description> <command...>
  local desc="$1"; shift
  CHANGES=$((CHANGES + 1))
  if [[ "${APPLY}" == "1" ]]; then
    if "$@" >/dev/null 2>&1; then ok "${desc}"; else bad "FAILED: ${desc}"; return 1; fi
  else
    plan "${desc}"
  fi
}

echo "Veyron cluster adapt → $($K config current-context 2>/dev/null || hostname)"
[[ "${APPLY}" == "1" ]] && echo "MODE: APPLY (changes will be made)" || echo "MODE: dry-run (no changes; pass --apply to commit)"
printf '%s\n' "────────────────────────────────────────────────"

# ── 1. Default StorageClass must not be node-local ────────────────────────────
# A node-local default (local-path) pins every VM disk to one node: no live
# migration, and the disk dies with the node. This is the single most common
# reason a lab cluster cannot go to production.
echo "1) Default StorageClass"
NODE_LOCAL_PROVISIONERS='rancher.io/local-path|kubernetes.io/no-provisioner|openebs.io/local|topolvm.io'
CUR_DEFAULT="$($K get sc -o jsonpath='{range .items[?(@.metadata.annotations.storageclass\.kubernetes\.io/is-default-class=="true")]}{.metadata.name}{"\n"}{end}' 2>/dev/null | head -1)"

# Pick the target: an explicitly requested class, else the first non-node-local one.
BLOCK_SC="${VEYRON_BLOCK_SC:-}"
if [[ -z "${BLOCK_SC}" ]]; then
  while read -r name prov; do
    [[ -z "${name}" ]] && continue
    echo "${prov}" | grep -qE "${NODE_LOCAL_PROVISIONERS}" && continue
    # Prefer a block/RBD class for VM root disks.
    if echo "${prov}" | grep -qi 'rbd\|block'; then BLOCK_SC="${name}"; break; fi
    [[ -z "${BLOCK_SC}" ]] && BLOCK_SC="${name}"
  done < <($K get sc -o custom-columns=N:.metadata.name,P:.provisioner --no-headers 2>/dev/null)
fi

if [[ -z "${BLOCK_SC}" ]]; then
  bad "No shared (non-node-local) StorageClass exists — install a real CSI first. Cannot adapt."
  bad "Everything below depends on this; aborting."
  exit 1
fi

CUR_PROV="$($K get sc "${CUR_DEFAULT}" -o jsonpath='{.provisioner}' 2>/dev/null || true)"
if [[ "${CUR_DEFAULT}" == "${BLOCK_SC}" ]]; then
  ok "Default StorageClass is already '${BLOCK_SC}' (shared)"
elif echo "${CUR_PROV}" | grep -qE "${NODE_LOCAL_PROVISIONERS}"; then
  warn "Default is '${CUR_DEFAULT}' (${CUR_PROV}) — node-local, unsuitable for production VMs"
  run "unset default on '${CUR_DEFAULT}'" \
    $K annotate sc "${CUR_DEFAULT}" storageclass.kubernetes.io/is-default-class- --overwrite
  run "set default StorageClass -> '${BLOCK_SC}'" \
    $K annotate sc "${BLOCK_SC}" storageclass.kubernetes.io/is-default-class=true --overwrite
elif [[ -z "${CUR_DEFAULT}" ]]; then
  warn "No default StorageClass set"
  run "set default StorageClass -> '${BLOCK_SC}'" \
    $K annotate sc "${BLOCK_SC}" storageclass.kubernetes.io/is-default-class=true --overwrite
else
  ok "Default StorageClass '${CUR_DEFAULT}' is shared (${CUR_PROV})"
fi

# ── 2. KubeVirt feature gates ─────────────────────────────────────────────────
# MERGE, never replace: the cluster may already rely on gates we don't manage
# (e.g. VideoConfig for the dashboard console).
echo; echo "2) KubeVirt feature gates"
WANT_GATES=(VMLiveUpdateFeatures VolumesUpdateStrategy VMPersistentState Snapshot HotplugVolumes)
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

echo "  current: ${CUR_GATES_JSON}"
if [[ -z "${MISSING_GATES}" ]]; then
  ok "All required feature gates already enabled"
else
  warn "Missing gates: ${MISSING_GATES}"
  run "merge feature gates -> ${MERGED_GATES}" \
    $K patch kubevirt kubevirt -n kubevirt --type merge \
      -p "{\"spec\":{\"configuration\":{\"developerConfiguration\":{\"featureGates\":${MERGED_GATES}}}}}"
fi

# ── 3. Backend storage for persistent TPM / EFI state (Windows 11, BitLocker) ─
# KubeVirt provisions these backend PVCs itself; the class must support RWX
# filesystem volumes.
echo; echo "3) Persistent TPM/EFI state storage"
FS_SC="${VEYRON_FS_SC:-}"
if [[ -z "${FS_SC}" ]]; then
  FS_SC="$($K get sc -o custom-columns=N:.metadata.name,P:.provisioner --no-headers 2>/dev/null \
    | awk '$2 ~ /cephfs|nfs|glusterfs|azurefile|efs/ {print $1; exit}')"
fi
CUR_VMSTATE="$($K get kubevirt kubevirt -n kubevirt -o jsonpath='{.spec.configuration.vmStateStorageClass}' 2>/dev/null || true)"
if [[ -n "${CUR_VMSTATE}" ]]; then
  ok "vmStateStorageClass already set to '${CUR_VMSTATE}'"
elif [[ -z "${FS_SC}" ]]; then
  warn "No RWX/filesystem StorageClass found — persistent TPM/EFI (Windows 11 BitLocker) stays unavailable"
else
  warn "vmStateStorageClass unset — Windows 11 persistent TPM/EFI unavailable"
  run "set vmStateStorageClass -> '${FS_SC}'" \
    $K patch kubevirt kubevirt -n kubevirt --type merge \
      -p "{\"spec\":{\"configuration\":{\"vmStateStorageClass\":\"${FS_SC}\"}}}"
fi

# ── 4. VolumeSnapshotClass per CSI driver ────────────────────────────────────
# A snapshot class whose driver has no matching StorageClass provisioner is
# useless; and a StorageClass with no snapshot class cannot be snapshotted.
echo; echo "4) VolumeSnapshotClasses"
if ! $K get crd volumesnapshotclasses.snapshot.storage.k8s.io >/dev/null 2>&1; then
  warn "VolumeSnapshot CRDs not installed — snapshots unavailable (install the CSI snapshot-controller)"
else
  EXISTING_DRIVERS="$($K get volumesnapshotclass -o jsonpath='{range .items[*]}{.driver}{"\n"}{end}' 2>/dev/null | sort -u)"
  while read -r name prov; do
    [[ -z "${name}" ]] && continue
    echo "${prov}" | grep -qE "${NODE_LOCAL_PROVISIONERS}" && continue
    # Bucket/object provisioners have no volume snapshots.
    echo "${prov}" | grep -qi 'bucket' && continue
    if echo "${EXISTING_DRIVERS}" | grep -qx "${prov}"; then
      ok "driver '${prov}' already has a VolumeSnapshotClass"
      continue
    fi
    SNAP_NAME="${name}-snapclass"
    warn "driver '${prov}' (StorageClass '${name}') has no VolumeSnapshotClass"
    if [[ "${APPLY}" == "1" ]]; then
      # Ceph CSI needs the secret pointing at its provisioner namespace.
      SNAP_NS="$($K get sc "${name}" -o jsonpath='{.parameters.csi\.storage\.k8s\.io/provisioner-secret-namespace}' 2>/dev/null)"
      SNAP_NS="${SNAP_NS:-rook-ceph}"
      PARAMS=""
      if echo "${prov}" | grep -qi 'ceph'; then
        PARAMS=$'parameters:\n  clusterID: '"${SNAP_NS}"$'\n  csi.storage.k8s.io/snapshotter-secret-name: rook-csi-'"$(echo "${prov}" | grep -qi cephfs && echo cephfs || echo rbd)"$'-provisioner\n  csi.storage.k8s.io/snapshotter-secret-namespace: '"${SNAP_NS}"
      fi
      if $K apply -f - >/dev/null 2>&1 <<EOF
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshotClass
metadata:
  name: ${SNAP_NAME}
  labels:
    veyron.io/managed-by: veyron-cluster-adapt
driver: ${prov}
deletionPolicy: Delete
${PARAMS}
EOF
      then ok "created VolumeSnapshotClass '${SNAP_NAME}' (driver ${prov})"
      else bad "FAILED to create VolumeSnapshotClass '${SNAP_NAME}' — create it by hand for driver ${prov}"
      fi
      CHANGES=$((CHANGES + 1))
    else
      plan "create VolumeSnapshotClass '${SNAP_NAME}' (driver ${prov})"
      CHANGES=$((CHANGES + 1))
    fi
  done < <($K get sc -o custom-columns=N:.metadata.name,P:.provisioner --no-headers 2>/dev/null)
fi

# ── 5. containerd block-device ownership (blank/Block-mode DataVolumes) ───────
# Without this, every non-root CDI importer pod (the default) fails opening
# /dev/cdi-block-volume with "Permission denied" on ANY Block-mode PVC — the
# DataVolume sits in ImportInProgress / CrashLoopBackOff forever. This is not
# specific to Windows golden images: it breaks every blank Block-mode volume
# (see kubevirt/containerized-data-importer doc/block_cri_ownership_config.md).
# Reproduced and root-caused live on a customer-representative lab cluster —
# k3s does not set this by default, so treat it as a go-live blocker.
echo; echo "5) Block-mode DataVolume permissions (containerd)"
K3S_CONF="/etc/rancher/k3s/config.yaml"
K3S_CONTAINERD_CONF="/var/lib/rancher/k3s/agent/etc/containerd/config.toml"
if command -v k3s >/dev/null 2>&1 || [[ -x /usr/local/bin/k3s ]]; then
  # k3s's containerd agent dir is root-only (0700) — a plain bash `[[ -f ]]`
  # test as a non-root SSH user silently reads as "file doesn't exist" rather
  # than denied, so this must go through sudo, not a bash builtin test.
  if sudo test -f "${K3S_CONTAINERD_CONF}" && sudo grep -q 'device_ownership_from_security_context = true' "${K3S_CONTAINERD_CONF}" 2>/dev/null; then
    ok "containerd already honors device_ownership_from_security_context"
  else
    warn "containerd will deny non-root CDI importer pods access to Block-mode PVCs"
    if [[ "${APPLY}" == "1" ]]; then
      CHANGES=$((CHANGES + 1))
      if ! sudo grep -q '^nonroot-devices:' "${K3S_CONF}" 2>/dev/null; then
        sudo mkdir -p "$(dirname "${K3S_CONF}")"
        echo 'nonroot-devices: true' | sudo tee -a "${K3S_CONF}" >/dev/null
      fi
      if sudo systemctl restart k3s 2>/dev/null; then
        # Wait for the API to come back before moving on.
        for _ in $(seq 1 30); do $K get nodes >/dev/null 2>&1 && break; sleep 5; done
        ok "set nonroot-devices: true and restarted k3s"
      else
        bad "FAILED to restart k3s — set 'nonroot-devices: true' in ${K3S_CONF} and restart it by hand"
      fi
    else
      plan "set 'nonroot-devices: true' in ${K3S_CONF} and restart k3s"
      CHANGES=$((CHANGES + 1))
    fi
  fi
elif sudo test -f /etc/containerd/config.toml; then
  if sudo grep -q 'device_ownership_from_security_context = true' /etc/containerd/config.toml 2>/dev/null; then
    ok "containerd already honors device_ownership_from_security_context"
  else
    warn "containerd will deny non-root CDI importer pods access to Block-mode PVCs —"
    warn "  add device_ownership_from_security_context = true under [plugins.\"io.containerd.grpc.v1.cri\"]"
    warn "  in /etc/containerd/config.toml and restart containerd (not auto-fixed on non-k3s hosts)"
  fi
fi

# ── 6. Report blockers we will NOT auto-fix ──────────────────────────────────
echo; echo "6) Blockers requiring a human"
NODE_COUNT="$($K get nodes --no-headers 2>/dev/null | wc -l | tr -d ' ')"
if [[ "${NODE_COUNT}" -lt 2 ]]; then
  warn "Single-node cluster — live migration is impossible until a 2nd node joins."
fi
if $K get cephcluster -A >/dev/null 2>&1; then
  CEPH_HEALTH="$($K get cephcluster -A -o jsonpath='{.items[0].status.ceph.health}' 2>/dev/null)"
  case "${CEPH_HEALTH}" in
    HEALTH_OK) ok "Ceph is HEALTH_OK" ;;
    HEALTH_WARN)
      warn "Ceph is HEALTH_WARN — storage is degraded, NOT production-ready. Investigate:"
      warn "  kubectl -n rook-ceph exec deploy/rook-ceph-tools -- ceph status" ;;
    *) bad "Ceph health is '${CEPH_HEALTH:-unknown}'" ;;
  esac
fi
# GPU nodes that advertise resources KubeVirt is not configured to hand out.
GPU_ALLOC="$($K get nodes -o jsonpath='{range .items[*]}{.metadata.name}{" "}{.status.allocatable}{"\n"}{end}' 2>/dev/null \
  | grep -E 'nvidia\.com/|amd\.com/|intel\.com/' || true)"
if [[ -n "${GPU_ALLOC}" ]]; then
  PERMITTED="$($K get kubevirt kubevirt -n kubevirt -o jsonpath='{.spec.configuration.permittedHostDevices}' 2>/dev/null || true)"
  if [[ -z "${PERMITTED}" || "${PERMITTED}" == "{}" ]]; then
    warn "Nodes advertise GPU resources but KubeVirt has no permittedHostDevices —"
    warn "  VMs cannot request them. Run: ./scripts/cluster/enable-gpu-passthrough.sh"
  else
    ok "GPU resources advertised and permittedHostDevices configured"
  fi
fi
# Multi-zone cluster whose StorageClasses ignore zones: PVCs can bind to the
# wrong site and pin VMs across the WAN (see docs/multi-site.md).
ZONE_COUNT="$($K get nodes -o jsonpath='{range .items[*]}{.metadata.labels.topology\.kubernetes\.io/zone}{"\n"}{end}' 2>/dev/null | grep -v '^$' | sort -u | wc -l | tr -d ' ')"
if [[ "${ZONE_COUNT}" -gt 1 ]]; then
  TOPO_SC="$($K get sc -o jsonpath='{range .items[*]}{.metadata.name}{" "}{.allowedTopologies}{"\n"}{end}' 2>/dev/null | awk 'NF>1' | wc -l | tr -d ' ')"
  if [[ "${TOPO_SC}" -eq 0 ]]; then
    warn "Cluster spans ${ZONE_COUNT} zones but no StorageClass sets allowedTopologies —"
    warn "  cross-site PVC binding is possible; add zone-scoped StorageClasses (docs/multi-site.md §4)"
  else
    ok "Multi-zone cluster has ${TOPO_SC} zone-scoped StorageClass(es)"
  fi
fi

printf '%s\n' "────────────────────────────────────────────────"
if [[ "${APPLY}" == "1" ]]; then
  ok "Adapt complete (${CHANGES} change(s) attempted)."
  echo "  Re-run the gate:  VEYRON_API_KEY=... ./scripts/preflight-veyron-remote.sh <host> 30151"
else
  if [[ "${CHANGES}" -eq 0 ]]; then
    ok "Cluster already matches the baseline — nothing to do."
  else
    warn "${CHANGES} change(s) planned. Re-run with --apply to commit."
  fi
fi
