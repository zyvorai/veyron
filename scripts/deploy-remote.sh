#!/usr/bin/env bash
# ============================================================================
# deploy-remote.sh — Entry point for full remote K8s deploy (API + operator)
# ============================================================================
# Delegates to deploy-all-remote.sh (rsync → build → import → CRDs → rollout).
#
# Usage:
#   ./scripts/deploy-remote.sh [host] [user] [--quick] [--no-preflight]
#
# Arguments:
#   host            SSH target (default: $DEPLOY_HOST, then HOST)
#   user            SSH user   (default: $DEPLOY_USER, then sus)
#   --quick         Skip image build and import (manifests only)
#   --no-preflight  Skip SSH connectivity check before starting
#
# Environment:
#   DEPLOY_HOST / DEPLOY_USER     Defaults when host/user omitted
#   DEPLOY_REMOTE_SKIP_CHECK=1    Skip SSH preflight (same as --no-preflight)
#   DEPLOY_SSH_TIMEOUT            Seconds for SSH connect (default: 20)
#   VMROGUE_SKIP_CDI, ...         See deploy-all-remote.sh / ensure-cdi-remote.sh
#   VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1  Skip apply of deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml
#   VMROGUE_REQUIRE_KUBEVIRT=1               deploy-k8s-remote.sh: fail if KubeVirt CRD missing
#
# Examples:
#   ./scripts/deploy-remote.sh 192.0.2.1 ubuntu
#   ./scripts/deploy-remote.sh --quick
#   DEPLOY_REMOTE_SKIP_CHECK=1 ./scripts/deploy-remote.sh myhost deploy
#
# After deploy: ./scripts/verify-vmrogue-remote.sh <host> [https_node_port]
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ALL_REMOTE="${SCRIPT_DIR}/deploy-all-remote.sh"
# shellcheck source=lib/deploy-remote-ui.sh
source "${SCRIPT_DIR}/lib/deploy-remote-ui.sh"

usage() {
    cat <<'EOF'
deploy-remote.sh — full remote K8s deploy (wraps deploy-all-remote.sh)

Usage:
  ./scripts/deploy-remote.sh [host] [user] [--quick] [--no-preflight]

  host, user    Optional; default from DEPLOY_HOST / DEPLOY_USER, else
                HOST / sus (same as deploy-all-remote.sh).
  --quick       Skip image build/import; re-apply manifests only.
  --no-preflight
                Skip SSH connectivity check (large builds still run remotely).

Environment:
  DEPLOY_REMOTE_SKIP_CHECK=1   Skip SSH preflight (same as --no-preflight)
  DEPLOY_SSH_TIMEOUT           SSH ConnectTimeout in seconds (default: 20)

EOF
    "${ALL_REMOTE}" --help
}

SKIP_PREFLIGHT=false
FORWARD=()
for arg in "$@"; do
    case "$arg" in
        --no-preflight) SKIP_PREFLIGHT=true ;;
        -h|--help)
            usage
            exit 0
            ;;
        *) FORWARD+=("$arg") ;;
    esac
done

POSITIONAL=()
for arg in "${FORWARD[@]}"; do
    case "$arg" in
        --quick) ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-HOST}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
REMOTE="${USER}@${HOST}"
SSH_TIMEOUT="${DEPLOY_SSH_TIMEOUT:-20}"

if [[ "${DEPLOY_REMOTE_SKIP_CHECK:-0}" == "1" ]] || [[ "${SKIP_PREFLIGHT}" == true ]]; then
    deploy_skip_preflight_note
    exec "${ALL_REMOTE}" "${FORWARD[@]}"
fi

deploy_preflight_banner "${REMOTE}" "${SSH_TIMEOUT}"

if ! ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=accept-new \
    "${REMOTE}" "true" 2>/dev/null; then
    deploy_preflight_fail "${REMOTE}" "${SSH_TIMEOUT}" "${0##*/} $(printf '%q ' "$@")"
fi

deploy_preflight_ok

exec "${ALL_REMOTE}" "${FORWARD[@]}"
