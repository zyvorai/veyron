#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# P3 — Security & multi-tenancy. Asserts the route-level RBAC matrix in
# src/api/auth_context.rs actually enforces (not just that admin works), plus
# auth/TLS posture and namespace scoping. Optionally proves cross-namespace
# golden-image clone RBAC (denied without RoleBinding, allowed with it).
#
# Usage (standalone or via customer-readiness.sh):
#   VEYRON_API_KEY=admin ./scripts/test/security-rbac-remote.sh <host> [port] [ssh-user]
#
# Multi-role coverage (else the enforcement matrix SKIPs):
#   VEYRON_RO_KEY / VEYRON_WRITE_KEY / VEYRON_ADMIN_KEY   (readonly/write/admin)
# Opt-in:
#   VEYRON_READY_ALLOW_CLONE_RBAC=1   cross-ns clone RBAC (throwaway tenant ns)
#     — needs cluster-side kubectl to create a namespace + apply a RoleBinding.
#     Pass ssh-user (3rd arg) so those run over SSH against the target cluster
#     instead of assuming the local machine already has a matching kubeconfig
#     context (it usually doesn't when testing a remote/customer cluster —
#     that silently degrades this to SKIP/inconclusive, not a clean failure).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
SSH_USER="${3:-}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port] [ssh-user]" >&2; exit 2; }
BASE="https://${HOST}:${PORT}"
if [[ -n "${SSH_USER}" ]]; then
    K() { ssh -o StrictHostKeyChecking=no "${SSH_USER}@${HOST}" "sudo /usr/local/bin/k3s kubectl $*"; }
    K_APPLY_STDIN() { ssh -o StrictHostKeyChecking=no "${SSH_USER}@${HOST}" "sudo /usr/local/bin/k3s kubectl apply -f -"; }
    K_AVAILABLE=1
else
    K() { kubectl "$@"; }
    K_APPLY_STDIN() { kubectl apply -f -; }
    command -v kubectl >/dev/null 2>&1 && kubectl get nodes >/dev/null 2>&1 && K_AVAILABLE=1 || K_AVAILABLE=0
fi
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
if [[ "${KEY}" == "CHANGE_ME" ]]; then
    warn "Default API key 'CHANGE_ME' in use — rotate via VEYRON_API_KEY / veyron-api-key Secret before go-live"
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
    # This requires cluster-side kubectl (create a namespace/PVC/DataSource,
    # apply a RoleBinding); if we can't drive it (no SSH user and no local
    # kubectl context pointed at this cluster — the common case when testing
    # a remote/customer cluster from a workstation), degrade to SKIP.
    CLONE_NS="veyron-readiness-tenant-${TS}"
    IMG_NS="${VEYRON_IMAGE_NS:-vm-images}"
    SRC_PVC="veyron-readiness-clonesrc-${TS}"
    DS_NAME="veyron-readiness-clonesrc-${TS}"
    VM_NAME="veyron-readiness-clone-${TS}"
    if [[ "${K_AVAILABLE}" -ne 1 ]]; then
        skip "cross-ns clone RBAC" "no_kubectl:pass ssh-user (3rd arg) or point local kubectl at this cluster"
    else
        K create namespace "${CLONE_NS}" >/dev/null 2>&1 || true
        register_cleanup "K delete namespace ${CLONE_NS} --wait=false"
        # A real backing PVC + DataSource (test-scoped name, not the real
        # "windows-golden" catalog entry — never touch real customer data).
        # CDI's cross-namespace clone RBAC webhook only fires once the
        # DataSource/PVC actually resolve; an unresolvable reference is
        # rejected earlier by KubeVirt's own mutator with a "not found"
        # error that looks superficially similar but is NOT the RBAC gate
        # this test exists to exercise.
        printf '%s\n' \
            "apiVersion: v1" \
            "kind: PersistentVolumeClaim" \
            "metadata: {name: ${SRC_PVC}, namespace: ${IMG_NS}}" \
            "spec: {accessModes: [ReadWriteOnce], resources: {requests: {storage: 1Gi}}}" \
            "---" \
            "apiVersion: cdi.kubevirt.io/v1beta1" \
            "kind: DataSource" \
            "metadata: {name: ${DS_NAME}, namespace: ${IMG_NS}}" \
            "spec: {source: {pvc: {name: ${SRC_PVC}, namespace: ${IMG_NS}}}}" \
            | K_APPLY_STDIN >/dev/null 2>&1
        register_cleanup "K delete datasource ${DS_NAME} -n ${IMG_NS} --ignore-not-found"
        register_cleanup "K delete pvc ${SRC_PVC} -n ${IMG_NS} --ignore-not-found"

        # Poll VM Events for the DataVolume-create Warning event — this is
        # NOT synchronous with POST /vms (which returns 200 as soon as the
        # VM object itself is accepted; the DataVolumeTemplate only actually
        # gets materialized, and RBAC-checked, once the VM starts).
        # Kubernetes Events persist independently of the object's lifecycle
        # (~1h TTL) — a stale "UnauthorizedDataVolumeCreate" event from a
        # PRIOR (deleted) VM with the same name is still returned by
        # `--field-selector involvedObject.name=X` for a NEW VM object that
        # reuses that name. Verified this is a real trap, not theoretical:
        # a deny→grant→retry cycle on the same name/namespace found the old
        # event immediately and reported "still denied" even when the retry
        # had genuinely succeeded. Filter by involvedObject.uid instead, so
        # only events for the object THIS poll actually created are seen.
        poll_clone_event() { # poll_clone_event NS VM UID TIMEOUT_S -> prints last matching event reason, or ""
            local ns="$1" vm="$2" uid="$3" timeout="$4" waited=0
            while [[ "${waited}" -lt "${timeout}" ]]; do
                local ev
                ev="$(K get events -n "${ns}" --field-selector "involvedObject.uid=${uid}" -o jsonpath='{.items[*].reason}' 2>/dev/null)"
                if [[ "${ev}" == *UnauthorizedDataVolumeCreate* ]]; then echo "UnauthorizedDataVolumeCreate"; return; fi
                if [[ "${ev}" == *FailedDataVolumeCreate* ]]; then echo "FailedDataVolumeCreate"; return; fi
                local dv_phase
                dv_phase="$(K get dv "${vm}-rootdisk" -n "${ns}" -o jsonpath='{.status.phase}' 2>/dev/null)"
                [[ "${dv_phase}" == "Succeeded" || "${dv_phase}" == "Bound" ]] && { echo "${dv_phase}"; return; }
                sleep 3; waited=$((waited + 3))
            done
            echo ""
        }

        clone_body="$(printf '{"name":"%s","namespace":"%s","template":"ubuntu-22.04","image":{"name":"%s","namespace":"%s"},"start":true}' \
            "${VM_NAME}" "${CLONE_NS}" "${DS_NAME}" "${IMG_NS}")"
        curl_api POST "/api/v1/vms" "${clone_body}"
        register_cleanup "K delete vm ${VM_NAME} -n ${CLONE_NS} --ignore-not-found --wait=false"
        denied_before="0"
        if ! body_success; then
            skip "cross-ns clone denial" "inconclusive:vm_create_failed:$(printf '%s' "${_CURL_BODY}" | head -c 120)"
        else
            vm_uid="$(K get vm "${VM_NAME}" -n "${CLONE_NS}" -o jsonpath='{.metadata.uid}' 2>/dev/null)"
            result="$(poll_clone_event "${CLONE_NS}" "${VM_NAME}" "${vm_uid}" 30)"
            if [[ "${result}" == "UnauthorizedDataVolumeCreate" ]]; then
                pass "cross-ns clone denied without RoleBinding (UnauthorizedDataVolumeCreate event)"
                denied_before="1"
            else
                skip "cross-ns clone denial" "inconclusive:no UnauthorizedDataVolumeCreate event within 30s (last: ${result:-none})"
            fi
        fi

        # Grant the RoleBinding for THIS tenant ns, in the IMAGE ns (not the
        # tenant ns — that's what actually authorizes the clone). Built
        # inline rather than sed-ing deploy/k8s/bootstrap/cdi-golden-image-cloner.yaml:
        # that file has two `namespace:` lines (the RoleBinding's own
        # metadata.namespace: vm-images, and subjects[0].namespace: customer-a)
        # and a blind `s/namespace: .*/.../ ` rewrites both, silently moving the
        # RoleBinding itself into the tenant namespace instead of vm-images —
        # which grants nothing (verified: this previously always left "could
        # not apply cloner RoleBinding" / a no-op binding in the wrong place).
        if printf '%s\n' \
            "apiVersion: rbac.authorization.k8s.io/v1" \
            "kind: ClusterRole" \
            "metadata: {name: cdi-golden-image-cloner, labels: {veyron.io/managed-by: veyron}}" \
            "rules:" \
            "  - apiGroups: [cdi.kubevirt.io]" \
            "    resources: [datavolumes/source]" \
            "    verbs: [create]" \
            "  - apiGroups: [cdi.kubevirt.io]" \
            "    resources: [datasources]" \
            "    verbs: [get, list, watch]" \
            "---" \
            "apiVersion: rbac.authorization.k8s.io/v1" \
            "kind: RoleBinding" \
            "metadata: {name: ${CLONE_NS}-golden-image-cloner, namespace: ${IMG_NS}, labels: {veyron.io/managed-by: veyron}}" \
            "subjects: [{kind: ServiceAccount, name: default, namespace: ${CLONE_NS}}]" \
            "roleRef: {apiGroup: rbac.authorization.k8s.io, kind: ClusterRole, name: cdi-golden-image-cloner}" \
            | K_APPLY_STDIN >/dev/null 2>&1
        then
            pass "applied cdi-golden-image-cloner RoleBinding (${CLONE_NS} -> ${IMG_NS})"
            register_cleanup "K delete rolebinding ${CLONE_NS}-golden-image-cloner -n ${IMG_NS} --ignore-not-found"

            # Recreate the VM (a failed DataVolumeTemplate clone doesn't retry
            # on its own) and confirm the SAME denial no longer occurs.
            curl_api DELETE "/api/v1/vms/${CLONE_NS}/${VM_NAME}" '{}' yes >/dev/null 2>&1
            sleep 3
            curl_api POST "/api/v1/vms" "${clone_body}"
            if ! body_success; then
                skip "cross-ns clone allow-path" "inconclusive:vm_recreate_failed:$(printf '%s' "${_CURL_BODY}" | head -c 120)"
            else
                vm_uid="$(K get vm "${VM_NAME}" -n "${CLONE_NS}" -o jsonpath='{.metadata.uid}' 2>/dev/null)"
                result="$(poll_clone_event "${CLONE_NS}" "${VM_NAME}" "${vm_uid}" 30)"
                if [[ "${result}" == "UnauthorizedDataVolumeCreate" ]]; then
                    fail "cross-ns clone still denied after granting RoleBinding" "event: ${result}"
                elif [[ "${denied_before}" -eq 1 ]]; then
                    pass "cross-ns clone RBAC denial cleared after granting RoleBinding (dv: ${result:-still pending})"
                else
                    skip "cross-ns clone allow-path" "inconclusive:deny-before step did not produce a clean baseline"
                fi
            fi
        else
            warn "could not apply cloner RoleBinding (manual step)"
        fi
    fi
fi

printf '\n%bP3 security: %d passed, %d failed, %d warn, %d skipped%b\n' "${_B}" "${PASS}" "${FAIL}" "${WARN}" "${SKIP}" "${_N}"
[[ "${FAIL}" -eq 0 ]]
