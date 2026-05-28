#!/usr/bin/env bash
# Apply vmrogue-integrations Secret when observability / GitOps services exist on the cluster.
# Sourced or invoked from deploy-all-remote.sh with K8S_CMD and NAMESPACE set.
set -euo pipefail

# Obtain Argo CD session token via cluster Service ClusterIP (reachable from the deploy node on k3s).
bootstrap_argocd_credentials() {
    local k8s="${1:?kubectl}"
    local argocd_ns="" argocd_svc="" argocd_url="" argocd_token="" argocd_default_app=""

    for cand in argocd argo-cd; do
        if ${k8s} get ns "${cand}" &>/dev/null; then
            argocd_ns="${cand}"
            break
        fi
    done
    if [[ -z "${argocd_ns}" ]]; then
        return 0
    fi

    argocd_svc=$(${k8s} get svc -n "${argocd_ns}" -o name 2>/dev/null | grep -E 'argocd-server|/server$' | head -1 | sed 's|service/||' || true)
    if [[ -z "${argocd_svc}" ]]; then
        argocd_svc="argocd-server"
    fi
    if ! ${k8s} get svc "${argocd_svc}" -n "${argocd_ns}" &>/dev/null; then
        echo "Argo CD namespace ${argocd_ns} found but no server Service — skip Argo wiring"
        return 0
    fi

    local port cluster_ip session_base
    port=$(${k8s} get svc "${argocd_svc}" -n "${argocd_ns}" -o jsonpath='{.spec.ports[?(@.name=="https")].port}' 2>/dev/null || true)
    if [[ -z "${port}" ]]; then
        port=$(${k8s} get svc "${argocd_svc}" -n "${argocd_ns}" -o jsonpath='{.spec.ports[0].port}' 2>/dev/null || true)
    fi
    [[ -z "${port}" ]] && port=443
    cluster_ip=$(${k8s} get svc "${argocd_svc}" -n "${argocd_ns}" -o jsonpath='{.spec.clusterIP}' 2>/dev/null || true)
    argocd_url="https://${argocd_svc}.${argocd_ns}.svc:${port}"
    if [[ -n "${cluster_ip}" && "${cluster_ip}" != "None" ]]; then
        session_base="https://${cluster_ip}:${port}"
    else
        session_base="${argocd_url}"
    fi

    if [[ "${VMROGUE_SKIP_ARGOCD_TOKEN_BOOTSTRAP:-}" != "1" && "${VMROGUE_SKIP_ARGOCD_TOKEN_BOOTSTRAP:-}" != "true" ]]; then
        if ${k8s} get secret argocd-initial-admin-secret -n "${argocd_ns}" &>/dev/null; then
            local pw_b64 pw body
            pw_b64=$(${k8s} get secret argocd-initial-admin-secret -n "${argocd_ns}" -o jsonpath='{.data.password}' 2>/dev/null || true)
            if [[ -n "${pw_b64}" ]]; then
                pw=$(printf '%s' "${pw_b64}" | base64 -d 2>/dev/null || true)
                if [[ -n "${pw}" ]]; then
                    if command -v jq &>/dev/null; then
                        body=$(jq -n --arg p "${pw}" '{username:"admin",password:$p}')
                        argocd_token=$(curl -skS --connect-timeout 12 --max-time 25 -X POST "${session_base}/api/v1/session" \
                            -H "Content-Type: application/json" \
                            -d "${body}" 2>/dev/null | jq -r '.token // empty' || true)
                    else
                        argocd_token=$(curl -skS --connect-timeout 12 --max-time 25 -X POST "${session_base}/api/v1/session" \
                            -H "Content-Type: application/json" \
                            -d "{\"username\":\"admin\",\"password\":\"${pw//\"/\\\"}\"}" 2>/dev/null \
                            | sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1 || true)
                    fi
                    if [[ -n "${argocd_token}" ]]; then
                        echo "Argo CD session token obtained for ${argocd_ns}"
                    else
                        echo "Argo CD: could not obtain session token (set VMROGUE_ARGOCD_TOKEN manually)"
                    fi
                fi
            fi
        else
            echo "Argo CD: argocd-initial-admin-secret not found — URL only; add VMROGUE_ARGOCD_TOKEN manually"
        fi
    fi

    argocd_default_app=$(${k8s} get applications.argoproj.io -A -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)

    printf '%s\n%s\n%s\n%s' "${argocd_url}" "${argocd_token}" "${argocd_default_app}" "${argocd_ns}"
}

bootstrap_vmrogue_integrations() {
    local k8s="${1:?kubectl command}"
    local ns="${2:-vmrogue-system}"

    if [[ "${VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP:-}" == "1" || "${VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP:-}" == "true" ]]; then
        echo "Skipping vmrogue-integrations bootstrap (VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP=1)"
        return 0
    fi

    local prom_url="" am_url="" loki_url="" opencost_url="" trivy_url="" jaeger_url="" grafana_url=""
    local argocd_url="" argocd_token="" argocd_default_app=""
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
        grafana_svc=$(${k8s} get svc -n monitoring -o name 2>/dev/null | grep -i grafana | grep -v headless | head -1 | sed 's|service/||' || true)
        if [[ -n "${grafana_svc}" ]]; then
            grafana_port=$(${k8s} get svc "${grafana_svc}" -n monitoring -o jsonpath='{.spec.ports[?(@.name=="service")].port}' 2>/dev/null || true)
            [[ -z "${grafana_port}" ]] && grafana_port=$(${k8s} get svc "${grafana_svc}" -n monitoring -o jsonpath='{.spec.ports[0].port}' 2>/dev/null || true)
            [[ -z "${grafana_port}" ]] && grafana_port=3000
            grafana_url="http://${grafana_svc}.monitoring.svc:${grafana_port}"
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

    if ${k8s} get crd backups.velero.io &>/dev/null; then
        echo "Velero CRDs detected — cluster backups available in dashboard Backups page"
    fi

    local argo_lines argo_ns_discovered=""
    argo_lines=$(bootstrap_argocd_credentials "${k8s}" || true)
    if [[ -n "${argo_lines}" ]]; then
        argocd_url=$(echo "${argo_lines}" | sed -n '1p')
        argocd_token=$(echo "${argo_lines}" | sed -n '2p')
        argocd_default_app=$(echo "${argo_lines}" | sed -n '3p')
        argo_ns_discovered=$(echo "${argo_lines}" | sed -n '4p')
        if [[ -n "${argocd_url}" ]]; then
            echo "Argo CD API URL: ${argocd_url} (namespace ${argo_ns_discovered})"
        fi
    fi

    if [[ -z "${prom_url}" && -z "${am_url}" && -z "${loki_url}" && -z "${opencost_url}" && -z "${trivy_url}" && -z "${jaeger_url}" && -z "${grafana_url}" && -z "${argocd_url}" ]]; then
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
    [[ -n "${grafana_url}" ]] && echo "  VMROGUE_GRAFANA_URL: \"${grafana_url}\"" >>"${tmp}"
    [[ -n "${argocd_url}" ]] && echo "  VMROGUE_ARGOCD_URL: \"${argocd_url}\"" >>"${tmp}"
    [[ -n "${argocd_token}" ]] && echo "  VMROGUE_ARGOCD_TOKEN: \"${argocd_token}\"" >>"${tmp}"
    [[ -n "${argocd_default_app}" ]] && echo "  VMROGUE_ARGOCD_DEFAULT_APP: \"${argocd_default_app}\"" >>"${tmp}"

    ${k8s} apply -f "${tmp}"
    rm -f "${tmp}"
    echo "vmrogue-integrations applied (restart vmrogue-api to pick up env if already running)"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    bootstrap_vmrogue_integrations "${1:?kubectl}" "${2:-vmrogue-system}"
fi
