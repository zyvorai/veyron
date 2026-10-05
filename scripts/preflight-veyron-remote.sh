#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Customer-site readiness preflight for a DEPLOYED Veyron API. Reports which
# Day-2 operations the target cluster actually supports (KubeVirt feature gates
# + installed components), flags dev-grade security defaults, and runs the HTTPS
# smoke test. Run this as the go-live acceptance gate at a customer site.
#
# Usage:
#   VEYRON_API_KEY='...' ./scripts/preflight-veyron-remote.sh <host> [https_node_port] [--smoke]
#
# Fast by default (reachability + security posture + Day-2 capabilities, seconds).
# Pass --smoke (or PREFLIGHT_SMOKE=1) to also run the full verify-veyron-remote.sh
# functional smoke test (~2-3 min).
#
# Exits non-zero if the API is unreachable or (when requested) the smoke test fails.
set -euo pipefail

RUN_SMOKE="${PREFLIGHT_SMOKE:-0}"
POSARGS=()
for a in "$@"; do
  case "$a" in
    --smoke) RUN_SMOKE=1 ;;
    --no-smoke) RUN_SMOKE=0 ;;
    *) POSARGS+=("$a") ;;
  esac
done
set -- "${POSARGS[@]:-}"

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
KEY="${VEYRON_API_KEY:-$(source "$(dirname "${BASH_SOURCE[0]}")/lib/api-key.sh" && veyron_require_api_key "${HOST}" 2>/dev/null)}"
if [[ -n "${HOST}" && -z "${KEY}" ]]; then
    echo "VEYRON_API_KEY is not set and no saved key for ${HOST} (deploy prints and saves it to ~/.config/veyron/hosts/)" >&2
    exit 1
fi
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port] [--smoke]" >&2; exit 2; }
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE="https://${HOST}:${PORT}"

c() { curl -sk -H "X-API-Key: ${KEY}" "$@"; }
hr() { printf '%s\n' "────────────────────────────────────────────────"; }
ok(){ printf '  \033[0;32m✔\033[0m %s\n' "$*"; }
warn(){ printf '  \033[0;33m!\033[0m %s\n' "$*"; }
bad(){ printf '  \033[0;31m✘\033[0m %s\n' "$*"; }

echo "Veyron customer-site preflight → ${BASE}"; hr

# 1) API reachable
if ! c -o /dev/null -w '' "${BASE}/health" 2>/dev/null; then
  bad "API not reachable at ${BASE}/health"; exit 1
fi
ok "API reachable"

# 2) Security posture
if [[ "${KEY}" == "CHANGE_ME" ]]; then
  warn "Using the DEFAULT API key 'CHANGE_ME' — set VEYRON_API_KEY / veyron-api-key Secret before go-live"
else
  ok "Non-default API key in use"
fi
c -o /dev/null -w '' "${BASE}/api/v1/health/ready" 2>/dev/null && ok "Readiness endpoint healthy" || warn "Readiness probe not OK"

# 3) Acceptance gate — grade the cluster against the go-live requirements.
#    Hard FAILs block sign-off (exit non-zero); WARNs are capability gaps that
#    degrade gracefully. See docs/deploy.md.
GATE_FAILED=0
echo; echo "Acceptance gate (target cluster):"; hr
caps="$(c "${BASE}/api/v1/platform/capabilities" 2>/dev/null || true)"
if [[ -z "${caps}" || "${caps}" == *'"error"'* ]]; then
  bad "capabilities endpoint unavailable (redeploy API to enable /platform/capabilities)"
  GATE_FAILED=1
else
  python3 - "${caps}" <<'PY' || GATE_FAILED=1
import sys, json

G, Y, R, N = "\033[0;32m", "\033[0;33m", "\033[0;31m", "\033[0m"
d = json.loads(sys.argv[1])
st, nodes, ops = d.get("storage", {}), d.get("nodes", {}), d.get("day2_ops", {})
fails, warns = [], []

def check(cond, label, detail="", hard=True):
    if cond:
        print(f"  {G}✔{N} {label}")
    else:
        (fails if hard else warns).append(label)
        mark, col = (f"{R}✘{N}", R) if hard else (f"{Y}!{N}", Y)
        print(f"  {mark} {col}{label}{N}" + (f" — {detail}" if detail else ""))

print(f"\n  Platform: kubevirt={d.get('kubevirt_version')} nodes={nodes.get('count')} "
      f"kvm_capable={nodes.get('kvm_capable')}")

# --- Hard requirements: without these, VMs are not production-viable. ---
check(bool(d.get("kubevirt_version")), "KubeVirt installed and reporting a version")
check(nodes.get("kvm_capable", 0) > 0, "Hardware virtualization exposed to KubeVirt",
      "no node advertises devices.kubevirt.io/kvm — VMs would need slow software emulation")

dsc = st.get("default_storage_class")
check(dsc is not None, "A default StorageClass exists",
      "VM disk creation without an explicit class will fail")
check(not st.get("default_storage_class_node_local", False),
      "Default StorageClass is shared (not node-local)",
      f"default is '{dsc}' — node-local storage pins each VM to one node, "
      "breaks live migration, and loses the disk if that node dies")
check(st.get("shared_storage_class_present", False), "A shared (non-node-local) StorageClass exists")
check(ops.get("snapshots", False), "A VolumeSnapshotClass object exists",
      "the snapshot CRD may be installed but with zero classes — nothing can actually snapshot")
if st.get("volume_snapshot_classes"):
    check(st.get("snapshot_drivers_matched", False),
          "VolumeSnapshotClass driver matches a StorageClass provisioner",
          "snapshots will fail at bind time")

# --- Capability gaps: degrade gracefully, but the customer must know. ---
check(ops.get("cpu_memory_live_hotplug", False), "Live CPU/memory hotplug",
      "enable KubeVirt feature gate VMLiveUpdateFeatures", hard=False)
check(ops.get("storage_class_migration", False), "Storage-class disk migration",
      "enable KubeVirt feature gate VolumesUpdateStrategy", hard=False)
check(ops.get("live_migration", False), "Live migration",
      f"needs >=2 nodes (have {nodes.get('count')}) and RWX storage "
      f"(rwx_capable={st.get('rwx_capable')})", hard=False)
check(ops.get("windows_golden_images", False), "Windows golden-image workflow",
      "needs CDI + a shared StorageClass", hard=False)
check(ops.get("persistent_tpm_efi", False), "Persistent TPM/EFI state (Windows 11, BitLocker)",
      f"needs feature gate VMPersistentState + vmStateStorageClass "
      f"(currently {st.get('vm_state_storage_class')})", hard=False)
check(ops.get("velero_backups", False), "Velero off-cluster backups", "optional", hard=False)

sc_summary = ", ".join(
    f"{s['name']}{'*' if s['default'] else ''}"
    f"{' (node-local)' if s['node_local'] else ''}"
    f"{' (rwx)' if s['rwx_capable'] else ''}"
    for s in st.get("storage_classes", []))
print(f"\n  StorageClasses: {sc_summary or 'none'}   (* = default)")

if fails:
    print(f"\n  {R}GATE FAILED{N} — {len(fails)} blocking issue(s): " + "; ".join(fails))
    sys.exit(1)
print(f"\n  {G}Gate passed{N}" + (f" ({len(warns)} capability gap(s) — see above)" if warns else ""))
PY
fi

# 4) Functional smoke test (opt-in — it takes ~2-3 min)
if [[ "${RUN_SMOKE}" == "1" ]]; then
  echo; echo "HTTPS API smoke test:"; hr
  if [[ -x "${SCRIPT_DIR}/verify-veyron-remote.sh" ]]; then
    VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/verify-veyron-remote.sh" "${HOST}" "${PORT}" || { bad "smoke test failed"; exit 1; }
  else
    warn "verify-veyron-remote.sh not found; skipping smoke test"
  fi
else
  echo; warn "Skipped functional smoke test (pass --smoke to run verify-veyron-remote.sh, ~2-3 min)"
fi

hr
if [[ "${GATE_FAILED}" == "1" ]]; then
  bad "Preflight FAILED — the cluster is not ready for go-live (see blocking issues above)."
  bad "Remediate with: ./scripts/cluster/adapt-existing-cluster.sh <host> <user> --apply"
  exit 1
fi
ok "Preflight complete."
ok "Full smoke test:  VEYRON_API_KEY=... $0 ${HOST} ${PORT} --smoke"
ok "Daily-ops E2E:    VEYRON_API_KEY=... ./scripts/test-vm-daily-ops-remote.sh ${HOST} ${PORT}"
ok "Console sweep:    VEYRON_API_KEY=... ./scripts/dashboard-console-check.sh --host ${HOST}"
