#!/usr/bin/env bash
# shellcheck shell=bash
# Rich terminal UX for deploy-remote.sh and deploy-all-remote.sh (sources package-ui.sh).

[[ -n "${_DEPLOY_UI_LOADED:-}" ]] && return 0
_DEPLOY_UI_LOADED=1

_DEPLOY_UI_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=package-ui.sh
source "${_DEPLOY_UI_DIR}/package-ui.sh"

DEPLOY_RUN_STARTED=${SECONDS:-0}
_DEPLOY_PHASE_START=0
_DEPLOY_PHASE_NAME=""

deploy_format_duration() {
    local total="$1"
    local mins=$((total / 60))
    local secs=$((total % 60))
    printf "%02dm %02ds" "${mins}" "${secs}"
}

deploy_timestamp() {
    date +"%H:%M:%S"
}

deploy_preflight_banner() {
    local remote="$1"
    local timeout="${2:-20}"
    local port="${3:-22}"
    pkg_banner "🚀 VMRogue · deploy-remote" "SSH preflight · ${remote}"
    pkg_box_begin "Target"
    pkg_box_line "Host: ${remote}" "${PKG_C_BOLD}${PKG_C_CYAN}"
    pkg_box_line "Port: ${port} · Timeout: ${timeout}s · BatchMode" "${PKG_C_DIM}"
    pkg_box_end
}

deploy_preflight_ok() {
    pkg_ok "SSH connection verified"
    pkg_info "Handing off to deploy-all-remote.sh…"
    echo ""
}

deploy_preflight_fail() {
    local remote="$1"
    local timeout="$2"
    local script_name="$3"
    local port="${4:-22}"
    local detail="${5:-}"
    echo ""
    pkg_fail "Cannot reach ${remote} on port ${port} (timeout ${timeout}s, BatchMode auth)"
    if [[ -n "${detail}" ]]; then
        pkg_box_begin "SSH error"
        pkg_box_line "${detail}" "${PKG_C_RED}"
        pkg_box_end
    fi
    pkg_next_steps \
        "Test manually: ssh -p ${port} ${remote}" \
        "Console fix: sudo bash scripts/bootstrap-remote-sshd-port22.sh" \
        "Skip preflight: DEPLOY_REMOTE_SKIP_CHECK=1 ${script_name} …" \
        "Or pass: --no-preflight"
    exit 1
}

deploy_skip_preflight_note() {
    pkg_warn "SSH preflight skipped (--no-preflight / DEPLOY_REMOTE_SKIP_CHECK=1)"
    echo ""
}

deploy_main_banner() {
    local remote="$1"
    local ctr="$2"
    local runtime="$3"
    local flavor="$4"
    local k8s_cmd="$5"
    local quick="${6:-false}"

    local mode="🐳 Full image build + cluster rollout"
    if [[ "${quick}" == "true" ]]; then
        mode="⚡ Quick redeploy — manifests only (--quick)"
    fi

    pkg_banner "🔥 VMRogue · remote Kubernetes deploy" "${remote}"
    pkg_box_begin "Pipeline"
    pkg_box_line "${mode}" "${PKG_C_BOLD}${PKG_C_MAGENTA}"
    pkg_box_line "Started $(deploy_timestamp)" "${PKG_C_DIM}"
    pkg_box_end

    pkg_box_begin "Remote environment"
    pkg_box_line "Builder:   ${ctr}" "${PKG_C_CYAN}"
    pkg_box_line "Runtime:   ${runtime}" "${PKG_C_CYAN}"
    pkg_box_line "Cluster:   ${flavor}" "${PKG_C_CYAN}"
    pkg_box_line "kubectl:   ${k8s_cmd}" "${PKG_C_DIM}"
    pkg_box_end
    echo ""
}

deploy_phase_start() {
    _DEPLOY_PHASE_NAME="$1"
    _DEPLOY_PHASE_START=${SECONDS}
    echo ""
    printf "%s%s ▶  %s%s\n" \
        "${PKG_C_BOLD}${PKG_C_BLUE}" "━━" "${_DEPLOY_PHASE_NAME}" "${PKG_C_RESET}"
}

deploy_phase_end() {
    local elapsed=$((SECONDS - _DEPLOY_PHASE_START))
    printf "%s  ✔  %s %s(finished in %s)%s\n" \
        "${PKG_C_GREEN}" "${_DEPLOY_PHASE_NAME}" \
        "${PKG_C_DIM}" "$(deploy_format_duration "${elapsed}")" "${PKG_C_RESET}"
}

deploy_note() {
    printf "%s  › %s%s\n" "${PKG_C_DIM}" "$*" "${PKG_C_RESET}"
}

deploy_substep() {
    printf "%s  %s%s\n" "${PKG_C_CYAN}" "$*" "${PKG_C_RESET}"
}

deploy_skip_phase() {
    printf "%s  ⏭  %s%s\n" "${PKG_C_YELLOW}" "$*" "${PKG_C_RESET}"
}

deploy_stream_prefix() {
    local label="$1"
    if pkg_ui_tty; then
        printf '  %s[%s]%s ' "${PKG_C_DIM}" "${label}" "${PKG_C_RESET}"
    else
        printf '  [%s] ' "${label}"
    fi
}

deploy_build_fail() {
    local host="$1"
    pkg_fail "No container build tool on ${host} (need podman, docker, or nerdctl)"
    exit 1
}

deploy_complete() {
    local host="$1"
    local user="$2"
    local remote="$3"
    local node_port="$4"
    local http_redirect_port="$5"
    local api_key="$6"
    local script_dir="$7"
    local k8s_cmd="$8"
    local namespace="${9:-vmrogue-system}"

    local total=$((SECONDS - DEPLOY_RUN_STARTED))

    echo ""
    pkg_divider "═"
    printf "%s%s  ✅  Deployment complete%s\n" "${PKG_C_BOLD}${PKG_C_GREEN}" "🎉" "${PKG_C_RESET}"
    pkg_divider "═"
    echo ""

    pkg_box_begin "Open in browser"
    pkg_box_line "Dashboard:  https://${host}:${node_port}/dashboard" "${PKG_C_BOLD}${PKG_C_GREEN}"
    pkg_box_line "Health:     https://${host}:${node_port}/api/v1/health" "${PKG_C_CYAN}"
    if [[ -n "${http_redirect_port}" ]]; then
        pkg_box_line "HTTP redirect: node port ${http_redirect_port}" "${PKG_C_DIM}"
    fi
    pkg_box_end

    pkg_box_begin "Credentials & ops"
    pkg_box_line "API key: ${api_key}" "${PKG_C_YELLOW}"
    pkg_box_line "Total time: $(deploy_format_duration "${total}")" "${PKG_C_DIM}"
    pkg_box_line "Container listens on :5151 · use NodePort ${node_port} externally" "${PKG_C_DIM}"
    pkg_box_end

    pkg_next_steps \
        "Smoke test: ${script_dir}/verify-vmrogue-remote.sh ${host} ${node_port}" \
        "Re-deploy:  ${script_dir}/deploy-remote.sh ${host} ${user}" \
        "API logs:   ssh ${remote} \"${k8s_cmd} -n ${namespace} logs -l app.kubernetes.io/component=api -f\"" \
        "Op logs:    ssh ${remote} \"${k8s_cmd} -n ${namespace} logs -l app.kubernetes.io/component=operator -f\""
}
