#!/usr/bin/env bash
# Apply Cilium egress bootstrap YAML with the active Veyron namespace substituted.
# Supports legacy filenames (cilium-veyron-egress.yaml) and namespace literals.
apply_cilium_egress_bootstrap() {
    local kubectl_cmd="${1:?kubectl command}"
    local bootstrap_dir="${2:?bootstrap directory}"
    local namespace="${3:-veyron-system}"

    local yaml=""
    for candidate in \
        "${bootstrap_dir}/cilium-veyron-egress.yaml" \
        "${bootstrap_dir}/cilium-veyron-egress.yaml"; do
        if [[ -f "${candidate}" ]]; then
            yaml="${candidate}"
            break
        fi
    done

    if [[ -z "${yaml}" ]]; then
        echo "Skipping Cilium egress bootstrap (no cilium-veyron-egress.yaml under ${bootstrap_dir})"
        return 0
    fi

    echo "Applying Cilium egress policy for ${namespace} (veyron / nats workloads) from $(basename "${yaml}")…"
    sed \
        -e "s|__VEYRON_APP_NAMESPACE__|${namespace}|g" \
        -e "s|namespace: veyron-system|namespace: ${namespace}|g" \
        -e "s|namespace: veyron-system|namespace: ${namespace}|g" \
        "${yaml}" | ${kubectl_cmd} apply -f -
}
