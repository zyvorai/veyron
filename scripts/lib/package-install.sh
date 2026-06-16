#!/usr/bin/env bash
# Veyron — automatic client install (run inside extracted tarball directory).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
export PKG_INSTALL_ROOT="${ROOT}"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

pkg_parse_install_args "$@"

_PKG_SESSION_START=${SECONDS}
pkg_counters_reset
pkg_install_welcome "VMRogue"
pkg_bundle_sanity_check || true
pkg_banner "VMRogue" "Kubernetes VM management · client bundle"
pkg_step_init 5

pkg_step "System dependencies"
if [[ -x ./install-client-deps.sh ]]; then
  ./install-client-deps.sh && pkg_step_done || { pkg_warn "install-client-deps.sh had issues"; pkg_step_done; }
else
  pkg_skip "install-client-deps.sh not found"
  pkg_step_done
fi

pkg_step "Configuration, admin login & Kubernetes access"
pkg_k8s_env_configure vmrogue.env.example vmrogue.env "VMRogue"
pkg_step_done

pkg_step "Verify binaries"
if [[ -x ./vmrogue ]] && ./vmrogue --help >/dev/null 2>&1; then
  pkg_ok "vmrogue ($(./vmrogue --version 2>/dev/null | head -1 || echo ready))"
else
  pkg_fail "./vmrogue missing or not executable"
  exit 1
fi
[[ -x ./virtctl ]] && pkg_ok "virtctl included (optional)"
pkg_step_done

pkg_step "Cluster smoke test (optional)"
if [[ -f vmrogue.env ]]; then
  pkg_load_env_file vmrogue.env 2>/dev/null || true
fi
if [[ -n "${KUBECONFIG:-}" ]] && [[ -f "${KUBECONFIG}" ]] && [[ -x ./test-cluster.sh ]]; then
  ./test-cluster.sh && pkg_ok "test-cluster.sh" || pkg_warn "test-cluster.sh — see CLUSTER_SETUP.txt"
else
  pkg_skip "test-cluster.sh (needs KUBECONFIG in vmrogue.env)"
fi
pkg_step_done

pkg_step "Package smoke test"
[[ -x ./test-package.sh ]] && ./test-package.sh || pkg_warn "test-package.sh reported issues"
pkg_step_done

pkg_install_finish "VMRogue" http 5151 "/dashboard" \
  "Cluster (once): ./install-cluster.sh → deploy VMRogue in cluster" \
  "Start API: set -a && source vmrogue.env && set +a && ./vmrogue api-serve --host 0.0.0.0 --port 5151" \
  "Help: cat HELP.txt · ./install.sh --help" \
  "Override kubeconfig: ./install.sh --kubeconfig /path/to/config" \
  "Docs: CLUSTER_SETUP.txt · PREREQUISITES.txt"
