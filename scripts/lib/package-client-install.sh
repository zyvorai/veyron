#!/usr/bin/env bash
# Client-side runtime dependencies for Veyron binary bundle.
set -euo pipefail
# shellcheck source=/dev/null
[[ -f "$(dirname "$0")/package-ui.sh" ]] && source "$(dirname "$0")/package-ui.sh"
[[ -f "$(dirname "$0")/.package-lib/package-ui.sh" ]] && source "$(dirname "$0")/.package-lib/package-ui.sh"

pkg_banner "Veyron client dependencies" "Optional kubectl for cluster checks"
if command -v kubectl &>/dev/null; then
  pkg_ok "kubectl: $(kubectl version --client -o yaml 2>/dev/null | head -1 || kubectl version --client 2>/dev/null | head -1)"
else
  pkg_info "Installing kubectl (optional)…"
  if command -v dnf &>/dev/null; then
    sudo dnf install -y kubectl 2>/dev/null && pkg_ok "kubectl installed" || pkg_warn "kubectl install failed"
  elif command -v apt-get &>/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y kubectl 2>/dev/null && pkg_ok "kubectl installed" || pkg_warn "kubectl install failed"
  fi
fi
pkg_info "Cluster needs Kubernetes + KubeVirt (see CLUSTER_SETUP.txt)"
pkg_ok "veyron binary is static — no extra runtime libs"
pkg_summary "Dependencies"
