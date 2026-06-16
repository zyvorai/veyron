#!/usr/bin/env bash
# Smoke-test Veyron client bundle after install.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
export PKG_INSTALL_ROOT="${ROOT}"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && {
  pkg_script_help "test-package.sh"
  exit 0
}

[[ -f veyron.env ]] && pkg_load_env_file veyron.env 2>/dev/null || true

_PKG_SESSION_START=${SECONDS}
pkg_counters_reset
pkg_banner "Veyron package test" "Client binary · optional cluster · optional API"

if [[ -x ./veyron ]]; then
  ./veyron --help >/dev/null 2>&1 && pkg_ok "veyron --help" || pkg_fail "veyron --help"
else
  pkg_fail "./veyron missing"
fi

if [[ -x ./test-cluster.sh ]] && [[ -n "${KUBECONFIG:-}" ]] && [[ -f "${KUBECONFIG}" ]]; then
  if ./test-cluster.sh; then
    pkg_ok "test-cluster.sh"
  else
    pkg_warn "test-cluster.sh — see CLUSTER_SETUP.txt"
  fi
elif [[ -x ./test-cluster.sh ]]; then
  pkg_skip "test-cluster.sh (set KUBECONFIG in veyron.env)"
else
  pkg_skip "test-cluster.sh not in bundle"
fi

if [[ -n "${KUBECONFIG:-}" ]] && [[ -f "${KUBECONFIG}" ]]; then
  if ./veyron doctor 2>&1 | head -15 | while read -r line; do pkg_detail "${line}"; done; then
    pkg_ok "veyron doctor (see lines above)"
  else
    pkg_warn "veyron doctor"
  fi
else
  pkg_skip "veyron doctor (set KUBECONFIG)"
fi

if [[ -n "${VEYRON_API_KEY:-}" ]]; then
  if curl -sf "http://127.0.0.1:${VMROGUE_PORT:-5151}/api/v1/health" >/dev/null 2>&1; then
    pkg_ok "API health :${VMROGUE_PORT:-5151}"
  else
    pkg_skip "API not listening — start: ./veyron api-serve --host 0.0.0.0 --port ${VMROGUE_PORT:-5151}"
  fi
else
  pkg_skip "HTTP health (set VEYRON_API_KEY and start api-serve)"
fi

pkg_summary "Package test"
[[ "${_PKG_COUNTERS_FAIL}" -eq 0 ]]
