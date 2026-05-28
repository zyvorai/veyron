#!/usr/bin/env bash
# Apply vmrogue-integrations Secret when observability services exist on the cluster.
# Sourced or invoked from deploy-all-remote.sh with K8S_CMD and NAMESPACE set.
set -euo pipefail

bootstrap_vmrogue_integrations() {
    local k8s="${1:?kubectl command}"
    local ns="${2:-vmrogue-system}"

    if [[ "${VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP:-}" == "1" || "${VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP:-}" == "true" ]]; then
        echo "Skipping vmrogue-integrations bootstrap (VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP=1)"
        return 0
    fi

    local prom_url="" am_url="" loki_url="" opencost_url="" trivy_url="" jaeger_url=""
    local prom_svc="" am_svc="" loki_svc="" opencost_svc="" trivy_svc="" jaeger_svc=""

    if ${k8s} get ns monitoring &>/dev/null; then
        prom_svc=$(${k8s} get svc -n monitoring -l app.kubernetes.io/name=prometheus -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
        if [[ -z "${prom_svc}" ]]; then
            prom_svc=$(${k8s} get svc -n monitoring -o name 2>/dev/null | grep -i prometheus | head -1 | sed 's|service/||' || true)
        fi
        if [[ -n "${prom_svc}" ]]; then
            prom_url="http://${prom_svc}.monitoring.svc:9090/api/v1/query"
        fi
        am_svc=$(${k8s} get svc -n monitoring -o name 2>/dev/null | grep -i alertmanager | grep -v operated | head -1 | sed 's|service/||' || true)
        if [[ -n "${am_svc}" ]]; then
            am_url="http://${am_svc}.monitoring.svc:9093"
        fi
    fi

    if ${k8s} get ns loki &>/dev/null; then
        loki_svc=$(${k8s} get svc -n loki -o name 2>/dev/null | grep -E 'loki(-gateway)?$|/loki$' | head -1 | sed 's|service/||' || true)
        if [[ -z "${loki_svc}" ]]; then
            loki_svc=$(${k8s} get svc -n loki -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
        fi
        if [[ -n "${loki_svc}" ]]; then
            loki_url="http://${loki_svc}.loki.svc:3100"
        fi
    fi

    if ${k8s} get ns opencost &>/dev/null; then
        opencost_svc=$(${k8s} get svc -n opencost -o jsonpath='{.items[?(@.metadata.name=="opencost")].metadata.name}' 2>/dev/null || true)
        if [[ -z "${opencost_svc}" ]]; then
            opencost_svc=$(${k8s} get svc -n opencost -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
        fi
        if [[ -n "${opencost_svc}" ]]; then
            opencost_url="http://${opencost_svc}.opencost.svc:9003"
        fi
    fi

    for tns in trivy-system trivy; do
        if ${k8s} get ns "${tns}" &>/dev/null; then
            trivy_svc=$(${k8s} get svc -n "${tns}" -o name 2>/dev/null | grep -i trivy | head -1 | sed 's|service/||' || true)
            if [[ -n "${trivy_svc}" ]]; then
                trivy_url="http://${trivy_svc}.${tns}.svc:4954"
                break
            fi
        fi
    done

    if ${k8s} get ns monitoring &>/dev/null; then
        jaeger_svc=$(${k8s} get svc -n monitoring -o name 2>/dev/null | grep -i jaeger | grep -i query | head -1 | sed 's|service/||' || true)
        if [[ -n "${jaeger_svc}" ]]; then
            jaeger_url="http://${jaeger_svc}.monitoring.svc:16686/api/traces"
        fi
    fi

    if [[ -z "${prom_url}" && -z "${am_url}" && -z "${loki_url}" && -z "${opencost_url}" && -z "${trivy_url}" && -z "${jaeger_url}" ]]; then
        echo "No integration services detected — skip vmrogue-integrations Secret"
        return 0
    fi

    echo "Applying vmrogue-integrations Secret in ${ns}…"
    ${k8s} create namespace "${ns}" --dry-run=client -o yaml | ${k8s} apply -f - >/dev/null 2>&1 || true

    local tmp
    tmp=$(mktemp)
    cat >"${tmp}" <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: vmrogue-integrations
  namespace: ${ns}
  labels:
    app.kubernetes.io/name: vmrogue
    app.kubernetes.io/component: api
type: Opaque
stringData:
EOF
    [[ -n "${prom_url}" ]] && echo "  VMROGUE_PROMETHEUS_URL: \"${prom_url}\"" >>"${tmp}"
    [[ -n "${am_url}" ]] && echo "  VMROGUE_ALERTMANAGER_URL: \"${am_url}\"" >>"${tmp}"
    [[ -n "${loki_url}" ]] && echo "  VMROGUE_LOKI_URL: \"${loki_url}\"" >>"${tmp}"
    [[ -n "${opencost_url}" ]] && echo "  VMROGUE_OPENCOST_URL: \"${opencost_url}\"" >>"${tmp}"
    [[ -n "${trivy_url}" ]] && echo "  VMROGUE_TRIVY_URL: \"${trivy_url}\"" >>"${tmp}"
    [[ -n "${jaeger_url}" ]] && echo "  VMROGUE_JAEGER_QUERY_URL: \"${jaeger_url}\"" >>"${tmp}"

    ${k8s} apply -f "${tmp}"
    rm -f "${tmp}"
    echo "vmrogue-integrations applied (restart vmrogue-api to pick up env if already running)"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    bootstrap_vmrogue_integrations "${1:?kubectl}" "${2:-vmrogue-system}"
fi
