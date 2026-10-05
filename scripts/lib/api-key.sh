#!/usr/bin/env bash
# shellcheck shell=bash
# Per-host Veyron credentials: no baked-in defaults.
#
# Resolution order for deploys (veyron_resolve_deploy_secrets):
#   1. VEYRON_API_KEY / VEYRON_BOOTSTRAP_ADMIN_PASSWORD / VEYRON_JWT_SECRET env vars
#   2. the existing veyron-api-key Secret on the cluster (redeploys keep working keys)
#   3. freshly generated random values
# The result is saved to ~/.config/veyron/hosts/<host>.env (mode 600) so test scripts
# can pick the key up with veyron_require_api_key.

[[ -n "${_VEYRON_API_KEY_LOADED:-}" ]] && return 0
_VEYRON_API_KEY_LOADED=1

veyron_hosts_dir() {
    printf '%s/veyron/hosts' "${XDG_CONFIG_HOME:-${HOME}/.config}"
}

veyron_random_secret() {
    local len="${1:-32}"
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -base64 $((len * 2)) | tr -dc 'A-Za-z0-9' | head -c "${len}"
    else
        LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c "${len}"
    fi
}

veyron_save_host_secrets() {
    local host="$1" dir file
    dir="$(veyron_hosts_dir)"
    file="${dir}/${host}.env"
    mkdir -p "${dir}"
    (
        umask 077
        {
            printf 'VEYRON_API_KEY=%q\n' "${API_KEY}"
            printf 'VEYRON_BOOTSTRAP_ADMIN_PASSWORD=%q\n' "${ADMIN_PASSWORD}"
        } >"${file}"
    )
}

veyron_load_host_secrets() {
    local host="$1" file
    file="$(veyron_hosts_dir)/${host}.env"
    [[ -f "${file}" ]] || return 1
    # shellcheck disable=SC1090
    source "${file}"
}

# Usage: veyron_resolve_deploy_secrets HOST REMOTE "K8S_CMD" NAMESPACE
# Sets API_KEY, ADMIN_PASSWORD, JWT_SECRET (requires deploy_ssh from deploy-ssh.sh).
veyron_resolve_deploy_secrets() {
    local host="$1" remote="$2" kcmd="$3" ns="$4"
    local existing_key="" existing_pw="" existing_jwt=""
    if declare -F deploy_ssh >/dev/null; then
        existing_key=$(deploy_ssh "${remote}" "${kcmd} -n ${ns} get secret veyron-api-key -o jsonpath='{.data.api-key}' 2>/dev/null | base64 -d 2>/dev/null" 2>/dev/null || true)
        existing_pw=$(deploy_ssh "${remote}" "${kcmd} -n ${ns} get secret veyron-api-key -o jsonpath='{.data.admin-password}' 2>/dev/null | base64 -d 2>/dev/null" 2>/dev/null || true)
        existing_jwt=$(deploy_ssh "${remote}" "${kcmd} -n ${ns} get secret veyron-api-key -o jsonpath='{.data.jwt-secret}' 2>/dev/null | base64 -d 2>/dev/null" 2>/dev/null || true)
    fi
    API_KEY="${VEYRON_API_KEY:-${existing_key:-$(veyron_random_secret 32)}}"
    ADMIN_PASSWORD="${VEYRON_BOOTSTRAP_ADMIN_PASSWORD:-${existing_pw:-${API_KEY}}}"
    JWT_SECRET="${VEYRON_JWT_SECRET:-${existing_jwt:-$(veyron_random_secret 64)}}"
    veyron_save_host_secrets "${host}"
}

# Usage: KEY="$(veyron_require_api_key HOST)" — env var, then the saved host file.
veyron_require_api_key() {
    local host="${1:-}"
    if [[ -n "${VEYRON_API_KEY:-}" ]]; then
        printf '%s' "${VEYRON_API_KEY}"
        return 0
    fi
    if [[ -n "${host}" ]] && veyron_load_host_secrets "${host}" && [[ -n "${VEYRON_API_KEY:-}" ]]; then
        printf '%s' "${VEYRON_API_KEY}"
        return 0
    fi
    echo "VEYRON_API_KEY is not set and no saved key for '${host}' in $(veyron_hosts_dir)/ — export VEYRON_API_KEY (see the deploy summary)" >&2
    return 1
}
