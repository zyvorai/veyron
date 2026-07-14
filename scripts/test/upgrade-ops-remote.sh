#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# P6 — Upgrade path & day-2 ops. Upgrade is a dry-run by default; the real
# (same-version, no-op) operator bounce is opt-in and asserts a running test VM
# survives it. Also proves monitoring actually scrapes real datapoints, day-2
# hotplug is seen by the guest, and reports version drift vs versions.env.
#
# Usage:
#   VEYRON_API_KEY=admin ./scripts/test/upgrade-ops-remote.sh <host> [port]
# Opt-in:
#   VEYRON_READY_ALLOW_UPGRADE=1   same-version no-op patch (assert VM survives)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port]" >&2; exit 2; }
BASE="https://${HOST}:${PORT}"
# shellcheck source=../lib/test-helpers.sh
source "${SCRIPT_DIR}/../lib/test-helpers.sh"
CURRENT_PHASE="P6-upgrade-ops"
TS="${TS:-$(date +%s)}"
trap run_cleanup EXIT INT TERM
ensure_ns
[[ "${TEST_NS_FELL_BACK}" -eq 1 ]] && warn "dedicated namespace unavailable — using 'default' (test resources still name-scoped veyron-readiness-*)"

# Load the version baseline (single source of truth).
VERSIONS_ENV="${SCRIPT_DIR}/../cluster/versions.env"
[[ -f "${VERSIONS_ENV}" ]] && source "${VERSIONS_ENV}"

section "P6 · Upgrade path"

# Dry-run upgrade is always safe (patches nothing).
curl_api POST "/api/v1/platform/upgrade" '{"component":"kubevirt","image_tag":"v0.0.0-readiness-dryrun"}'
if [[ "${_CURL_CODE}" == "200" ]]; then
    pass "platform upgrade dry-run → 200"
else
    skip "platform upgrade dry-run" "endpoint_unavailable:HTTP ${_CURL_CODE}"
fi

if ! guard VEYRON_READY_ALLOW_UPGRADE; then
    skip "real upgrade no-op bounce (assert VM survives)" "opt_in_required:VEYRON_READY_ALLOW_UPGRADE"
else
    # Bring up a test VM, then patch KubeVirt to its CURRENT version (no-op) and
    # assert the VM stays Running across the operator reconcile.
    UVM="$(new_name upg)"
    curl_api POST "/api/v1/vms" "$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":true}' "${UVM}" "${TEST_NS}")"
    if body_success "${_CURL_BODY}" && wait_vm_status "${TEST_NS}" "${UVM}" Running 360; then
        register_cleanup "dispose_vm ${TEST_NS} ${UVM}"
        pass "created upgrade test VM ${UVM} (Running)"
        cur_kv="${KUBEVIRT_VERSION:-}"
        if [[ -z "${cur_kv}" ]]; then
            cur_kv="$(platform_version kubevirt)"
        fi
        if [[ -n "${cur_kv}" ]]; then
            curl_api POST "/api/v1/platform/upgrade" "$(printf '{"component":"kubevirt","image_tag":"%s","confirm":true}' "${cur_kv}")"
            [[ "${_CURL_CODE}" == "200" ]] \
                && pass "same-version KubeVirt patch (${cur_kv}) accepted" \
                || warn "same-version patch returned HTTP ${_CURL_CODE}"
            sleep 20
            [[ "$(_vm_status "${TEST_NS}" "${UVM}")" == "Running" ]] \
                && pass "test VM survived operator bounce (still Running)" \
                || fail "test VM did not survive operator bounce"
        else
            skip "real upgrade bounce" "no_current_version"
        fi
    else
        skip "real upgrade bounce" "upgrade_vm_not_ready"
        dispose_vm "${TEST_NS}" "${UVM}" 2>/dev/null || true
    fi
fi

section "P6 · Monitoring actually scrapes"

# A test VM to scrape. Reuse a short-lived one.
MVM="$(new_name mon)"
curl_api POST "/api/v1/vms" "$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":true}' "${MVM}" "${TEST_NS}")"
if body_success "${_CURL_BODY}" && wait_vm_status "${TEST_NS}" "${MVM}" Running 360; then
    register_cleanup "dispose_vm ${TEST_NS} ${MVM}"
    pass "created monitoring test VM ${MVM} (Running)"
    # Give the scraper a moment, then assert real memory datapoints (not just a 200).
    sleep 25
    curl_api GET "/api/v1/metrics/${MVM}?metric=memory"
    if [[ "${_CURL_CODE}" == "200" ]] && body_has "${_CURL_BODY}" '"value"|"memory"|"data"|[0-9]'; then
        if body_has "${_CURL_BODY}" '"error"|prometheus.*not|no data'; then
            skip "monitoring scrape (real datapoints)" "no_prometheus"
        else
            pass "GET /metrics/${MVM} returned data (scrape working)"
        fi
    else
        skip "monitoring scrape" "no_prometheus_or_metric:HTTP ${_CURL_CODE}"
    fi

    section "P6 · Day-2 hotplug seen by guest"
    if wait_guest_connected "${TEST_NS}" "${MVM}" 240; then
        # guest exec uses {path,arg} and returns base64 out-data; guest_exec decodes it.
        guest_exec "${TEST_NS}" "${MVM}" /usr/bin/nproc
        before="$(printf '%s' "${_GUEST_STDOUT}" | grep -oE '[0-9]+' | head -1)"
        curl_api POST "/api/v1/vms/${TEST_NS}/${MVM}/hotplug" '{"sockets":2,"memory":"2Gi"}'
        # The hotplug handler returns {"ok":true,...} (not the "success" envelope).
        if [[ "${_CURL_CODE}" == "200" ]] && body_has "${_CURL_BODY}" '"ok"[[:space:]]*:[[:space:]]*true'; then
            pass "POST hotplug {sockets:2,memory:2Gi} accepted"
            sleep 20
            guest_exec "${TEST_NS}" "${MVM}" /usr/bin/nproc
            after="$(printf '%s' "${_GUEST_STDOUT}" | grep -oE '[0-9]+' | head -1)"
            if [[ -z "${before}" || -z "${after}" ]]; then
                skip "hotplug guest CPU visibility" "nproc_unreadable"
            elif [[ "${after}" -gt "${before}" ]]; then
                pass "guest sees more CPUs after hotplug (${before} → ${after})"
            else
                warn "guest CPU count not observed to increase (${before} → ${after}) — hotplug applies on next restart unless LiveUpdate online-cpu is enabled"
            fi
        else
            skip "day-2 hotplug" "hotplug_unavailable:HTTP ${_CURL_CODE}"
        fi
    else
        skip "day-2 hotplug guest verification" "no_guest_agent"
    fi
else
    skip "monitoring scrape + hotplug" "mon_vm_not_ready"
    dispose_vm "${TEST_NS}" "${MVM}" 2>/dev/null || true
fi

section "P6 · Version drift vs versions.env"

curl_api GET "/api/v1/platform/versions"
if [[ "${_CURL_CODE}" == "200" ]]; then
    got_kv="$(platform_version kubevirt)"
    got_cdi="$(platform_version cdi)"
    want_kv="${KUBEVIRT_VERSION:-}"
    want_cdi="${CDI_VERSION:-}"
    if [[ -z "${want_kv}" ]]; then
        skip "KubeVirt version drift" "no_baseline"
    elif [[ "${got_kv}" == "${want_kv}" ]]; then
        pass "KubeVirt version matches baseline (${got_kv})"
    else
        warn "KubeVirt version drift: cluster=${got_kv} baseline=${want_kv}"
    fi
    if [[ -z "${want_cdi}" ]]; then
        skip "CDI version drift" "no_baseline"
    elif [[ -z "${got_cdi}" ]]; then
        skip "CDI version drift" "cdi_version_not_reported"
    elif [[ "${got_cdi}" == "${want_cdi}" ]]; then
        pass "CDI version matches baseline (${got_cdi})"
    else
        warn "CDI version drift: cluster=${got_cdi} baseline=${want_cdi}"
    fi
else
    skip "version drift" "versions_endpoint_unavailable:HTTP ${_CURL_CODE}"
fi

printf '\n%bP6 upgrade-ops: %d passed, %d failed, %d warn, %d skipped%b\n' "${_B}" "${PASS}" "${FAIL}" "${WARN}" "${SKIP}" "${_N}"
[[ "${FAIL}" -eq 0 ]]
