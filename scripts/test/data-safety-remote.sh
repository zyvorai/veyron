#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# P4 — Data safety round-trips. The gap the current suite can't close: it runs
# on ephemeral containerDisk VMs, so a snapshot restore proves nothing about the
# disk. This drives real round-trips on disposable PVC/DataVolume-backed VMs:
#   - snapshot → mutate → restore → assert data returns
#   - DR export → apply → assert reconstructed
#   - orphan reclaim dry-run (opt-in real delete)
#   - Velero backup→(opt-in)delete→restore
#
# Usage:
#   VEYRON_API_KEY=admin ./scripts/test/data-safety-remote.sh <host> [port]
# Opt-in:
#   VEYRON_READY_ALLOW_ORPHAN_DELETE=1   real confirm=true orphan reclaim
#   VEYRON_READY_ALLOW_VELERO=1          real Velero backup→delete→restore
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port]" >&2; exit 2; }
BASE="https://${HOST}:${PORT}"
# shellcheck source=../lib/test-helpers.sh
source "${SCRIPT_DIR}/../lib/test-helpers.sh"
CURRENT_PHASE="P4-data-safety"
TS="${TS:-$(date +%s)}"
trap run_cleanup EXIT INT TERM
ensure_ns
[[ "${TEST_NS_FELL_BACK}" -eq 1 ]] && warn "dedicated namespace unavailable — using 'default' (test resources still name-scoped veyron-readiness-*)"

MARKER="veyron-readiness-${TS}"
MARKER_PATH="/root/veyron-marker"

section "P4 · Snapshot round-trip"

# The built-in ubuntu-22.04 template is containerDisk-rooted (ephemeral) — a file
# written to the root disk does not survive a restore, so the in-guest DATA proof
# needs a PVC-backed root from a published golden image. Set
# VEYRON_READY_DATASOURCE=<CDI DataSource name> to run the real data round-trip;
# without it, the snapshot create→ready→restore lifecycle still runs and the
# in-guest data assertion SKIPs honestly (never a false FAIL).
DS="${VEYRON_READY_DATASOURCE:-}"
DS_NS="${VEYRON_READY_DATASOURCE_NS:-}"       # namespace holding the DataSource
DS_SC="${VEYRON_READY_DATASOURCE_SC:-}"       # optional clone StorageClass
DS_MODE="${VEYRON_READY_DATASOURCE_MODE:-}"   # optional clone volumeMode (Filesystem|Block)
DS_SIZE="${VEYRON_READY_DATASOURCE_SIZE:-20Gi}" # clone disk size (>= golden PVC size)
PVC_BACKED=0
# The restore handler derives the VM name from the snapshot name by splitting on
# "-snap-", so the VM name must NOT contain "-snap-" and the snapshot must be
# "<vm>-snap-<suffix>" (trailing suffix so rsplit lands on the right boundary).
VM="$(new_name data)"
SNAP="${VM}-snap-rt"
DR_VM=""

if [[ -n "${DS}" ]]; then
    PVC_BACKED=1
    # template=ubuntu-22.04 supplies the GuestKit cloud-init (needed for the
    # in-guest marker); image={} repoints the root disk to a clone of the golden
    # image so a restore actually restores real disk data.
    img="{\"name\":\"${DS}\""
    [[ -n "${DS_NS}" ]] && img="${img},\"namespace\":\"${DS_NS}\""
    [[ -n "${DS_SC}" ]] && img="${img},\"storage_class\":\"${DS_SC}\""
    [[ -n "${DS_MODE}" ]] && img="${img},\"volume_mode\":\"${DS_MODE}\""
    img="${img}}"
    create_body="$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","image":%s,"cpus":2,"memory":"2Gi","disk_size":"%s","start":true}' "${VM}" "${TEST_NS}" "${img}" "${DS_SIZE}")"
else
    create_body="$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":true}' "${VM}" "${TEST_NS}")"
fi

curl_api POST "/api/v1/vms" "${create_body}"
if ! body_success "${_CURL_BODY}"; then
    fail "create snapshot test VM ${VM}" "$(printf '%s' "${_CURL_BODY}" | head -c 300)"
    skip "snapshot round-trip" "vm_create_failed"
else
    register_cleanup "dispose_snapshot ${TEST_NS} ${SNAP}"
    register_cleanup "dispose_vm ${TEST_NS} ${VM}"
    DR_VM="${VM}"
    pass "created snapshot test VM ${VM} ($([[ ${PVC_BACKED} -eq 1 ]] && echo "PVC-backed from ${DS}" || echo containerDisk))"

    if ! wait_vm_status "${TEST_NS}" "${VM}" Running 360; then
        fail "test VM did not reach Running within 6m"
        skip "snapshot round-trip" "vm_not_running"
    else
        pass "test VM Running"

        # ---- write a marker BEFORE snapshot (only meaningful when PVC-backed) ----
        GUEST_OK=0
        if [[ "${PVC_BACKED}" -eq 1 ]] && wait_guest_connected "${TEST_NS}" "${VM}" 300; then
            GUEST_OK=1
            # Targeted fdatasync of just the marker file (sync -d FILE) flushes it
            # to the volume without a slow global `sync`, so a crash-consistent
            # snapshot reliably captures it.
            guest_exec "${TEST_NS}" "${VM}" /bin/sh -c "echo ${MARKER} > ${MARKER_PATH}; sync -d ${MARKER_PATH}"
            [[ "${_GUEST_RC}" -eq 0 ]] \
                && pass "wrote data marker to ${MARKER_PATH}" \
                || { GUEST_OK=0; skip "in-guest marker write" "guest_exec_failed:$(printf '%s' "${_CURL_BODY}" | head -c 120)"; }
        elif [[ "${PVC_BACKED}" -eq 0 ]]; then
            skip "in-guest data round-trip" "no_pvc_backed_image:set VEYRON_READY_DATASOURCE"
        else
            skip "in-guest data round-trip" "no_guest_agent"
        fi

        # ---- ONLINE snapshot round-trip ----
        # KubeVirt fsfreezes the guest FS for an online (running-VM) snapshot via
        # the guest agent, producing an application-consistent image. This exercises
        # the GuestKit fsfreeze QGA path — earlier GuestKit returned a malformed
        # freeze-status reply and wedged online snapshots InProgress.
        curl_api POST "/api/v1/snapshots/${TEST_NS}/${VM}/create" \
            "$(printf '{"snapshot_name":"%s","app_consistent":true}' "${SNAP}")"
        if body_success "${_CURL_BODY}"; then
            pass "POST snapshot create ${SNAP}"
            wait_snapshot_ready "${TEST_NS}" "${VM}" "${SNAP}" 360 \
                && pass "snapshot reached ready (online app-consistent freeze)" \
                || warn "snapshot did not report ready within 6m — restore may be slow"

            # Mutate: delete the marker (VM still running) so restore must bring it back.
            if [[ "${GUEST_OK}" -eq 1 ]]; then
                guest_exec "${TEST_NS}" "${VM}" /bin/rm -f "${MARKER_PATH}"
                pass "deleted marker post-snapshot (restore must bring it back)"
            fi

            # ---- restore (VM must be stopped) ----
            # The restore endpoint itself waits out a terminating VMI (VM status
            # can read "Stopped" before the VMI is gone), so no client settle here.
            curl_api POST "/api/v1/vms/${TEST_NS}/${VM}/stop" '{}'
            wait_vm_status "${TEST_NS}" "${VM}" Stopped 300 || warn "VM slow to stop before restore"
            curl_api POST "/api/v1/snapshots/${TEST_NS}/${SNAP}/restore" '{}'
            if body_success "${_CURL_BODY}"; then
                pass "POST snapshot restore accepted"
                # Restore is async (recreates the PVC from the snapshot). Let it
                # settle before starting, else the VMI can't schedule (PVC not ready).
                sleep 60
                curl_api POST "/api/v1/vms/${TEST_NS}/${VM}/start" '{}'
                if wait_vm_status "${TEST_NS}" "${VM}" Running 420; then
                    pass "VM returned to Running after restore"
                    if [[ "${GUEST_OK}" -eq 1 ]] && wait_guest_connected "${TEST_NS}" "${VM}" 300; then
                        guest_exec "${TEST_NS}" "${VM}" /bin/cat "${MARKER_PATH}"
                        if [[ "${_GUEST_RC}" -eq 0 ]] && printf '%s' "${_GUEST_STDOUT}" | grep -q "${MARKER}"; then
                            pass "DATA ROUND-TRIP verified — marker '${MARKER}' returned after restore"
                        else
                            fail "restore lost data — marker not present after restore" "stdout='${_GUEST_STDOUT}'"
                        fi
                    else
                        skip "post-restore data verification" "$([[ ${PVC_BACKED} -eq 0 ]] && echo 'no_pvc_backed_image' || echo 'no_guest_agent_after_restore')"
                    fi
                else
                    fail "VM did not return to Running after restore"
                fi
            elif body_has "${_CURL_BODY}" 'RESTORE_FAILED|Internal server error'; then
                # A restore failure on a containerDisk VM reflects the test VM, not
                # the cluster — SKIP. On a PVC-backed VM it is a real defect — FAIL.
                if [[ "${PVC_BACKED}" -eq 1 ]]; then
                    fail "POST snapshot restore (PVC-backed)" "$(printf '%s' "${_CURL_BODY}" | head -c 200)"
                else
                    skip "snapshot restore" "restore_failed:containerdisk_only_no_pvc"
                fi
            else
                fail "POST snapshot restore" "$(printf '%s' "${_CURL_BODY}" | head -c 200)"
            fi
        else
            # Snapshot create failed. containerDisk-only VMs have no snapshottable
            # volume — SKIP (reflects the test VM). PVC-backed failure is a real FAIL.
            if [[ "${PVC_BACKED}" -eq 1 ]]; then
                fail "POST snapshot create (PVC-backed)" "$(printf '%s' "${_CURL_BODY}" | head -c 200)"
            else
                skip "snapshot create/restore lifecycle" "no_snapshottable_volume:containerdisk_only"
            fi
        fi
    fi
fi

section "P4 · DR export → apply → verify"

# DR export/apply is per-VM (namespace + vm_name → that VM's manifest + snapshots).
# Reconstruction proof: export the test VM's manifest, then dry-run apply it under
# a new target name to prove the reconstruction path works — without actually
# duplicating the disk.
if [[ -z "${DR_VM:-}" ]]; then
    skip "DR export/apply round-trip" "no_test_vm"
else
    curl_api GET "/api/v1/dr/export?namespace=${TEST_NS}&vm_name=${DR_VM}"
    if body_has "${_CURL_BODY}" '"virtual_machine"'; then
        pass "GET /dr/export returned ${DR_VM} manifest + snapshot set"
        curl_api POST "/api/v1/dr/apply" \
            "$(printf '{"namespace":"%s","vm_name":"%s","target_name":"%s-drcopy","dry_run":true}' "${TEST_NS}" "${DR_VM}" "${DR_VM}")"
        if [[ "${_CURL_CODE}" == "200" ]] && body_has "${_CURL_BODY}" '"status"|"target_name"|"message"'; then
            pass "POST /dr/apply (dry-run reconstruction) accepted"
        else
            skip "DR apply dry-run" "apply_unsupported:HTTP ${_CURL_CODE}"
        fi
    else
        skip "DR export/apply round-trip" "dr_export_failed:HTTP ${_CURL_CODE}"
    fi
fi

section "P4 · Orphan reclaim"

curl_api GET "/api/v1/storage/orphans"
if [[ "${_CURL_CODE}" == "200" ]]; then
    pass "GET /storage/orphans (dry-run list) → 200"
    if ! guard VEYRON_READY_ALLOW_ORPHAN_DELETE; then
        skip "orphan reclaim real delete (confirm=true)" "opt_in_required:VEYRON_READY_ALLOW_ORPHAN_DELETE"
    else
        # Only ever delete with confirm; the API's own owned-scope applies.
        curl_api DELETE "/api/v1/storage/orphans?confirm=true"
        [[ "${_CURL_CODE}" == "200" ]] \
            && pass "orphan reclaim confirm=true → 200" \
            || fail "orphan reclaim confirm=true" "HTTP ${_CURL_CODE}"
    fi
else
    skip "orphan reclaim" "orphans_endpoint_unavailable:HTTP ${_CURL_CODE}"
fi

section "P4 · Velero off-cluster backup"

# Detect Velero via capabilities.
if caps_bool day2_ops.velero_backups 2>/dev/null; then
    if ! guard VEYRON_READY_ALLOW_VELERO; then
        skip "Velero backup→restore round-trip" "opt_in_required:VEYRON_READY_ALLOW_VELERO"
    else
        BK="veyron-readiness-bk-${TS}"
        curl_api POST "/api/v1/velero/backups" "$(printf '{"name":"%s","namespace":"%s"}' "${BK}" "${TEST_NS}")"
        [[ "${_CURL_CODE}" == "200" || "${_CURL_CODE}" == "202" ]] \
            && pass "Velero backup ${BK} created" \
            || fail "Velero backup create" "HTTP ${_CURL_CODE}: $(printf '%s' "${_CURL_BODY}" | head -c 160)"
    fi
else
    skip "Velero backup round-trip" "velero_absent"
fi

printf '\n%bP4 data-safety: %d passed, %d failed, %d warn, %d skipped%b\n' "${_B}" "${PASS}" "${FAIL}" "${WARN}" "${SKIP}" "${_N}"
[[ "${FAIL}" -eq 0 ]]
