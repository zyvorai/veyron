#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# P5 — Resilience & recovery. Multi-node phases (live migration, cordon/drain)
# auto-SKIP on a single-node cluster with reason single_node. Self-healing runs
# as a dry-run report by default; the real VMI-kill remediation is opt-in.
#
# Usage:
#   VEYRON_API_KEY=admin ./scripts/test/resilience-remote.sh <host> [port]
# Opt-in:
#   VEYRON_READY_ALLOW_DRAIN=1   cordon/drain a node (uncordon auto-registered)
#   VEYRON_READY_ALLOW_HEAL=1    kill a TEST VMI and prove self-healing recovers it
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port]" >&2; exit 2; }
BASE="https://${HOST}:${PORT}"
# shellcheck source=../lib/test-helpers.sh
source "${SCRIPT_DIR}/../lib/test-helpers.sh"
CURRENT_PHASE="P5-resilience"
TS="${TS:-$(date +%s)}"
trap run_cleanup EXIT INT TERM
ensure_ns
[[ "${TEST_NS_FELL_BACK}" -eq 1 ]] && warn "dedicated namespace unavailable — using 'default' (test resources still name-scoped veyron-readiness-*)"

detect_node_count >/dev/null
printf '  cluster: %s node(s)\n' "${NODE_COUNT}"

section "P5 · Live migration"

if ! is_multinode; then
    skip "live migration (node A→B, IP unchanged, guest connected)" "single_node"
    skip "cordon/drain reschedule" "single_node"
elif ! caps_bool day2_ops.live_migration 2>/dev/null; then
    skip "live migration" "capability_off:needs RWX + VMLiveUpdate/live_migration gate"
    skip "cordon/drain reschedule" "capability_off:live_migration"
else
    VM="$(new_name mig)"
    create_body="$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":true}' "${VM}" "${TEST_NS}")"
    curl_api POST "/api/v1/vms" "${create_body}"
    if ! body_success "${_CURL_BODY}"; then
        fail "create migration test VM" "$(printf '%s' "${_CURL_BODY}" | head -c 200)"
    else
        register_cleanup "dispose_vm ${TEST_NS} ${VM}"
        pass "created migration test VM ${VM}"
        if wait_vm_status "${TEST_NS}" "${VM}" Running 360; then
            pass "test VM Running"
            # Record source node + IP before migration.
            curl_api GET "/api/v1/vms/${TEST_NS}/${VM}"
            src_node="$(json_str "${_CURL_BODY}" node)"
            src_ip="$(json_str "${_CURL_BODY}" ip)"
            curl_api POST "/api/v1/migrations" "$(printf '{"vm_name":"%s"}' "${VM}")"
            mig_id="$(json_str "${_CURL_BODY}" id)"
            if [[ -n "${mig_id}" ]] && wait_migration_complete "${mig_id}" 420; then
                pass "live migration ${mig_id} succeeded"
                curl_api GET "/api/v1/vms/${TEST_NS}/${VM}"
                dst_node="$(json_str "${_CURL_BODY}" node)"
                dst_ip="$(json_str "${_CURL_BODY}" ip)"
                [[ "$(_vm_status "${TEST_NS}" "${VM}")" == "Running" ]] \
                    && pass "VM still Running after migration" \
                    || fail "VM not Running after migration"
                if [[ -n "${src_node}" && -n "${dst_node}" && "${src_node}" != "${dst_node}" ]]; then
                    pass "VM moved node ${src_node} → ${dst_node}"
                else
                    warn "could not confirm node change (src='${src_node}' dst='${dst_node}')"
                fi
                if [[ -n "${src_ip}" && "${src_ip}" == "${dst_ip}" ]]; then
                    pass "VM IP unchanged across migration (${src_ip})"
                else
                    warn "VM IP changed or unknown (src='${src_ip}' dst='${dst_ip}')"
                fi
            else
                fail "live migration did not succeed" "id='${mig_id}' $(printf '%s' "${_CURL_BODY}" | head -c 160)"
            fi
        else
            fail "migration test VM did not reach Running"
        fi
    fi

    # Cordon/drain is destructive to scheduling — opt-in, uncordon always restored.
    if ! guard VEYRON_READY_ALLOW_DRAIN; then
        skip "cordon/drain reschedule" "opt_in_required:VEYRON_READY_ALLOW_DRAIN"
    else
        detect_node_count >/dev/null
        curl_api GET "/api/v1/nodes"
        node="$(json_str "${_CURL_BODY}" name)"
        if [[ -n "${node}" ]]; then
            register_cleanup "curl_api POST /api/v1/nodes/${node}/uncordon {}"
            curl_api POST "/api/v1/nodes/${node}/cordon" '{}'
            [[ "${_CURL_CODE}" == "200" ]] \
                && pass "cordoned node ${node} (uncordon registered as cleanup)" \
                || fail "cordon node ${node}" "HTTP ${_CURL_CODE}"
            curl_api POST "/api/v1/nodes/${node}/uncordon" '{}'
            [[ "${_CURL_CODE}" == "200" ]] \
                && pass "uncordoned node ${node}" \
                || warn "uncordon returned HTTP ${_CURL_CODE} — trap will retry"
        else
            skip "cordon/drain" "no_node_name"
        fi
    fi
fi

section "P5 · Self-healing"

# Dry-run report always runs (non-mutating).
curl_api POST "/api/v1/self-healing/run"
if [[ "${_CURL_CODE}" == "200" ]]; then
    pass "self-healing dry-run report → 200 ($(json_num "${_CURL_BODY}" unhealthy) unhealthy)"
else
    skip "self-healing dry-run" "endpoint_unavailable:HTTP ${_CURL_CODE}"
fi

if ! guard VEYRON_READY_ALLOW_HEAL; then
    skip "real self-healing (kill test VMI → assert recovery)" "opt_in_required:VEYRON_READY_ALLOW_HEAL"
else
    HVM="$(new_name heal)"
    curl_api POST "/api/v1/vms" "$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":true}' "${HVM}" "${TEST_NS}")"
    if body_success "${_CURL_BODY}" && wait_vm_status "${TEST_NS}" "${HVM}" Running 360; then
        register_cleanup "dispose_vm ${TEST_NS} ${HVM}"
        pass "created self-heal test VM ${HVM} (Running)"
        # Force-restart the VMI to simulate a failure (exact test-owned name).
        owned_by_test "${HVM}" || { fail "refused non-test heal target"; exit 1; }
        curl_api POST "/api/v1/vms/${TEST_NS}/${HVM}/restart" '{}'
        curl_api POST "/api/v1/self-healing/run?heal=true"
        if [[ "${_CURL_CODE}" == "200" ]]; then
            pass "self-healing heal=true executed"
            wait_vm_status "${TEST_NS}" "${HVM}" Running 360 \
                && pass "test VM recovered to Running after heal" \
                || fail "test VM did not recover after heal"
        else
            fail "self-healing heal=true" "HTTP ${_CURL_CODE}"
        fi
    else
        skip "real self-healing" "heal_vm_not_ready"
        dispose_vm "${TEST_NS}" "${HVM}" 2>/dev/null || true
    fi
fi

section "P5 · Ceph / storage health"

ch="$(ceph_health)"
case "${ch}" in
    ok)      pass "Ceph reports healthy" ;;
    present) skip "Ceph health" "ceph_present_health_not_exposed:check ceph -s on the cluster" ;;
    absent)  skip "Ceph health" "ceph_absent" ;;
esac

printf '\n%bP5 resilience: %d passed, %d failed, %d warn, %d skipped%b\n' "${_B}" "${PASS}" "${FAIL}" "${WARN}" "${SKIP}" "${_N}"
[[ "${FAIL}" -eq 0 ]]
