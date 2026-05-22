#!/usr/bin/env bash
# Smoke-test VMRogue client bundle after install.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
echo "== VMRogue package test =="
test -x ./vmrogue || { echo "FAIL: ./vmrogue missing"; exit 1; }
./vmrogue --help >/dev/null && echo "  OK: vmrogue --help"
if [ -x ./test-cluster.sh ] && [ -n "${KUBECONFIG:-}" ] && [ -f "${KUBECONFIG}" ]; then
  ./test-cluster.sh || echo "  WARN: test-cluster.sh — fix cluster (see CLUSTER_SETUP.txt)"
elif [ -x ./test-cluster.sh ]; then
  echo "  SKIP: set KUBECONFIG in vmrogue.env then ./test-cluster.sh"
fi
if [ -n "${KUBECONFIG:-}" ] && [ -f "${KUBECONFIG}" ]; then
  ./vmrogue doctor 2>&1 | head -20 && echo "  OK: vmrogue doctor (see output above)"
else
  echo "  SKIP: set KUBECONFIG to run vmrogue doctor"
fi
if [ -n "${VMROGUE_API_KEY:-}" ]; then
  if curl -sf "http://127.0.0.1:${VMROGUE_PORT:-5151}/api/v1/health" >/dev/null 2>&1; then
    echo "  OK: API health on :${VMROGUE_PORT:-5151}"
  else
    echo "  SKIP: API not listening (start: ./vmrogue api-serve --host 0.0.0.0 --port ${VMROGUE_PORT:-5151})"
  fi
else
  echo "  SKIP: set VMROGUE_API_KEY and start api-serve to test HTTP health"
fi
echo "Done."
