#!/usr/bin/env bash
# VMRogue — one-command client install (run inside extracted tarball directory).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

_PKG_SESSION_START=${SECONDS}
pkg_banner "VMRogue client install" "Kubernetes VM management · client bundle"
pkg_step_init 4

pkg_step "System dependencies"
if [[ -x ./install-client-deps.sh ]]; then
  if ./install-client-deps.sh; then
    pkg_step_done
  else
    pkg_warn "install-client-deps.sh had issues — continuing"
    pkg_step_done
  fi
else
  pkg_skip "install-client-deps.sh not found"
  pkg_step_done
fi

pkg_step "Configuration"
if [[ ! -f vmrogue.env ]] && [[ -f vmrogue.env.example ]]; then
  cp vmrogue.env.example vmrogue.env
  pkg_ok "Created vmrogue.env from example"
  pkg_detail "Edit KUBECONFIG and VMROGUE_API_KEY before starting the API"
elif [[ -f vmrogue.env ]]; then
  pkg_ok "vmrogue.env already present (not overwritten)"
else
  pkg_warn "vmrogue.env.example missing — create vmrogue.env manually"
fi
pkg_step_done

pkg_step "Verify binaries"
if [[ -x ./vmrogue ]]; then
  if ./vmrogue --help >/dev/null 2>&1; then
    pkg_ok "vmrogue binary ($(./vmrogue --version 2>/dev/null | head -1 || echo ready))"
  else
    pkg_fail "vmrogue --help failed"
    exit 1
  fi
else
  pkg_fail "./vmrogue missing or not executable"
  exit 1
fi
[[ -x ./virtctl ]] && pkg_ok "virtctl included (optional)"
pkg_step_done

pkg_step "Smoke test"
if [[ -x ./test-package.sh ]]; then
  ./test-package.sh || pkg_warn "test-package.sh reported issues"
else
  pkg_skip "test-package.sh not found"
fi
pkg_step_done

pkg_summary "Install complete"
pkg_next_steps \
  "Cluster (once): export KUBECONFIG=… → ./install-cluster.sh → ./apply-cluster-network.sh" \
  "Start client: set -a && source vmrogue.env && set +a" \
  "Run API: ./vmrogue api-serve --host 0.0.0.0 --port 5151" \
  "Dashboard: http://<this-server>:5151/dashboard" \
  "Docs: CLUSTER_SETUP.txt · PREREQUISITES.txt" \
  "Remove: ./uninstall.sh --yes [--remove-dir]"
