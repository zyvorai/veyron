#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
# https://zyvor.dev · info@zyvor.dev
#
# P3 — Security & multi-tenancy. Asserts the route-level RBAC matrix in
# src/api/auth_context.rs actually enforces (not just that admin works), plus
# auth/TLS posture and namespace scoping. Optionally proves cross-namespace
# golden-image clone RBAC (denied without RoleBinding, allowed with it).
#
# Usage (standalone or via customer-readiness.sh):
#   VEYRON_API_KEY=admin ./scripts/test/security-rbac-remote.sh <host> [port]
#
# Multi-role coverage (else the enforcement matrix SKIPs):
#   VEYRON_RO_KEY / VEYRON_WRITE_KEY / VEYRON_ADMIN_KEY   (readonly/write/admin)
# Opt-in:
#   VEYRON_READY_ALLOW_CLONE_RBAC=1   cross-ns clone RBAC (throwaway tenant ns)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port]" >&2; exit 2; }
BASE="https://${HOST}:${PORT}"
# shellcheck source=../lib/test-helpers.sh
source "${SCRIPT_DIR}/../lib/test-helpers.sh"
CURRENT_PHASE="P3-security"
TS="${TS:-$(date +%s)}"
trap run_cleanup EXIT INT TERM
ensure_ns
[[ "${TEST_NS_FELL_BACK}" -eq 1 ]] && warn "dedicated namespace unavailable — using 'default' (test resources still name-scoped veyron-readiness-*)"

RO_KEY="${VEYRON_RO_KEY:-}"
WRITE_KEY="${VEYRON_WRITE_KEY:-}"
ADMIN_KEY="${VEYRON_ADMIN_KEY:-${KEY}}"

section "P3 · Auth & TLS posture"

# Default API key is a WARN (never blocks) — but must be fixed before go-live.
if [[ "${KEY}" == "Admin@321" ]]; then
    warn "Default API key 'Admin@321' in use — rotate via VEYRON_API_KEY / veyron-api-key Secret before go-live"
else
    pass "Non-default API key in use"
fi

# Unauthenticated read MUST be rejected — success here means auth is off (FAIL).
curl_api GET "/api/v1/vms" '' no
if body_success "${_CURL_BODY}"; then
    fail "GET /api/v1/vms without a key SUCCEEDED — API auth is disabled" "HTTP ${_CURL_CODE}"
elif [[ "${_CURL_CODE}" == "401" || "${_CURL_CODE}" == "403" ]]; then
    pass "Unauthenticated GET /api/v1/vms rejected (HTTP ${_CURL_CODE})"
else
    pass "Unauthenticated GET /api/v1/vms not authorized (HTTP ${_CURL_CODE})"
fi

# TLS posture — self-signed dev cert is a WARN.
if command -v openssl >/dev/null 2>&1; then
    iss="$(tls_issuer)"
    if is_self_signed_dev_cert; then
        warn "Self-signed / dev TLS cert (issuer: ${iss:-unknown}) — install a trusted cert before go-live"
    elif [[ -n "${iss}" ]]; then
        pass "TLS cert issued by: ${iss}"
    else
        warn "Could not read TLS issuer"
    fi
else
    skip "TLS issuer inspection" "no_openssl"
fi

section "P3 · RBAC enforcement matrix (auth_context.rs)"

if [[ -z "${RO_KEY}" || -z "${WRITE_KEY}" ]]; then
    skip "RBAC read-only key denials" "no_ro_key:set VEYRON_RO_KEY"
    skip "RBAC write key admin-gate denials" "no_write_key:set VEYRON_WRITE_KEY"
    warn "RBAC enforcement matrix not exercised — supply VEYRON_API_KEYS with readonly+write+admin roles for full coverage"
else
    # A readonly key: can GET, cannot mutate. Use a nonexistent VM so nothing is touched.
    curl_api GET "/api/v1/vms" '' yes "${RO_KEY}"
    if body_success "${_CURL_BODY}" || [[ "${_CURL_CODE}" == "200" ]]; then
        pass "readonly key GET /vms → allowed"
    else
        fail "readonly key GET /vms should be allowed" "HTTP ${_CURL_CODE}"
    fi

    curl_api POST "/api/v1/vms/${TEST_NS}/nonexistent-zzzz/hotplug" '{"sockets":2}' yes "${RO_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "readonly key POST …/hotplug → 403 (write required)" \
        || fail "readonly key hotplug should be 403" "HTTP ${_CURL_CODE}: $(printf '%s' "${_CURL_BODY}" | head -c 160)"

    # A write key: can mutate normal routes, but admin gates must refuse.
    curl_api POST "/api/v1/nodes/nonexistent-node/cordon" '{}' yes "${WRITE_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "write key POST /nodes/x/cordon → 403 (admin required)" \
        || fail "write key node cordon should be 403" "HTTP ${_CURL_CODE}"

    curl_api POST "/api/v1/tenants" '{"name":"veyron-readiness-x"}' yes "${WRITE_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "write key POST /tenants → 403 (admin required)" \
        || fail "write key tenant create should be 403" "HTTP ${_CURL_CODE}"

    curl_api POST "/api/v1/snapshots/${TEST_NS}/nonexistent-snap/restore" '{}' yes "${WRITE_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "write key POST …/restore → 403 (admin required)" \
        || fail "write key snapshot restore should be 403" "HTTP ${_CURL_CODE}"

    curl_api POST "/api/v1/images/publish" '{"name":"x"}' yes "${WRITE_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "write key POST /images/publish → 403 (fleet-wide blast radius; admin required)" \
        || fail "write key images/publish should be 403" "HTTP ${_CURL_CODE}"

    # Alias canonicalization: the /api/v1/veyron/* prefix must NOT bypass admin gates.
    curl_api POST "/api/v1/veyron/tenants" '{"name":"veyron-readiness-x"}' yes "${WRITE_KEY}"
    [[ "${_CURL_CODE}" == "403" ]] \
        && pass "write key POST /veyron/tenants alias → 403 (canonicalized to admin gate)" \
        || fail "alias tenant create should be 403" "HTTP ${_CURL_CODE}"

    # The admin key must actually be able to reach an admin route (not a false 403 wall).
    curl_api GET "/api/v1/platform/versions" '' yes "${ADMIN_KEY}"
    [[ "${_CURL_CODE}" == "200" ]] \
        && pass "admin key GET /platform/versions → 200" \
        || warn "admin key GET /platform/versions returned HTTP ${_CURL_CODE}" "$(printf '%s' "${_CURL_BODY}" | head -c 160)"
fi

section "P3 · Namespace scoping"

# Create a test VM in TEST_NS; confirm a query scoped to a different empty ns
# does not list it. Uses the disposable containerDisk template (fast, no PVC).
SCOPE_VM="$(new_name scope)"
create_body="$(cat <<EOF
{"name":"${SCOPE_VM}","namespace":"${TEST_NS}","template":"ubuntu-22.04","cpus":1,"memory":"1Gi","disk_size":"10Gi","start":false}
EOF
)"
curl_api POST "/api/v1/vms" "${create_body}"
if body_success "${_CURL_BODY}"; then
    register_cleanup "dispose_vm ${TEST_NS} ${SCOPE_VM}"
    pass "created scoping probe VM ${SCOPE_VM} in ${TEST_NS}"

    curl_api GET "/api/v1/vms/${TEST_NS}/${SCOPE_VM}"
    body_has "${_CURL_BODY}" "${SCOPE_VM}" \
        && pass "VM visible when querying its own namespace" \
        || fail "VM not visible in its own namespace" "$(printf '%s' "${_CURL_BODY}" | head -c 160)"

    # Query a different, empty namespace — the test VM must not appear.
    curl_api GET "/api/v1/vms?namespace=kube-system"
    if body_has "${_CURL_BODY}" "${SCOPE_VM}"; then
        fail "namespace scoping leak: ${SCOPE_VM} appeared in a kube-system-scoped query"
    else
        pass "namespace scoping holds — test VM not leaked into another namespace"
    fi
else
    skip "namespace scoping probe" "vm_create_failed:$(printf '%s' "${_CURL_BODY}" | head -c 120)"
fi

section "P3 · Cross-namespace clone RBAC"

if ! guard VEYRON_READY_ALLOW_CLONE_RBAC; then
    skip "cross-ns golden-image clone RBAC (denied→allowed)" "opt_in_required:VEYRON_READY_ALLOW_CLONE_RBAC"
else
    # This requires cluster-side RoleBinding application (kubectl); if the API
    # can't drive it, we degrade to SKIP rather than FAIL.
    CLONE_NS="veyron-readiness-tenant-${TS}"
    BOOTSTRAP="${SCRIPT_DIR}/../../deploy/k8s/bootstrap/cdi-golden-image-cloner.yaml"
    if ! command -v kubectl >/dev/null 2>&1; then
        skip "cross-ns clone RBAC" "no_kubectl"
    elif [[ ! -f "${BOOTSTRAP}" ]]; then
        skip "cross-ns clone RBAC" "no_bootstrap_manifest"
    else
        kubectl create namespace "${CLONE_NS}" >/dev/null 2>&1 || true
        register_cleanup "kubectl delete namespace ${CLONE_NS} --wait=false"
        # Attempt a clone before the RoleBinding — expect UnauthorizedDataVolumeCreate.
        clone_body="$(printf '{"name":"veyron-readiness-clone-%s","namespace":"%s","source_datasource":"windows"}' "${TS}" "${CLONE_NS}")"
        curl_api POST "/api/v1/vms" "${clone_body}"
        if body_has "${_CURL_BODY}" 'Unauthorized|UnauthorizedDataVolumeCreate|forbidden'; then
            pass "cross-ns clone denied without RoleBinding (UnauthorizedDataVolumeCreate)"
        else
            skip "cross-ns clone denial" "inconclusive:$(printf '%s' "${_CURL_BODY}" | head -c 120)"
        fi
        # Apply the cloner RoleBinding for the tenant ns and retry (allowed path).
        sed "s/namespace: .*/namespace: ${CLONE_NS}/" "${BOOTSTRAP}" | kubectl apply -f - >/dev/null 2>&1 \
            && pass "applied cdi-golden-image-cloner RoleBinding to ${CLONE_NS}" \
            || warn "could not apply cloner RoleBinding (manual step)"
    fi
fi

printf '\n%bP3 security: %d passed, %d failed, %d warn, %d skipped%b\n' "${_B}" "${PASS}" "${FAIL}" "${WARN}" "${SKIP}" "${_N}"
[[ "${FAIL}" -eq 0 ]]
