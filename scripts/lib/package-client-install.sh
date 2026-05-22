#!/usr/bin/env bash
# Client-side runtime dependencies for VMRogue binary bundle.
# Run on the target Linux host after extracting the tarball (sudo if installing kubectl).
set -euo pipefail
echo "== VMRogue client dependencies =="
if command -v kubectl &>/dev/null; then
  echo "  kubectl: $(kubectl version --client -o yaml 2>/dev/null | head -1 || kubectl version --client)"
else
  echo "  Installing kubectl (optional, for cluster checks)..."
  if command -v dnf &>/dev/null; then
    sudo dnf install -y kubectl 2>/dev/null || true
  elif command -v apt-get &>/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y kubectl 2>/dev/null || true
  fi
fi
echo "  Cluster: Kubernetes + KubeVirt required (see README.txt)"
echo "  Binary: static — no extra runtime libs for ./vmrogue"
echo "Done."
