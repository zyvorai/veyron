#!/usr/bin/env bash
# VMRogue — one-command client install (run inside extracted tarball directory).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  VMRogue client install                                  ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

echo "► Step 1/4 — System dependencies (optional kubectl)…"
if [ -x ./install-client-deps.sh ]; then
  ./install-client-deps.sh || echo "  (continuing — fix deps manually if needed)"
else
  echo "  install-client-deps.sh not found — skip"
fi

echo ""
echo "► Step 2/4 — Configuration…"
if [ ! -f vmrogue.env ] && [ -f vmrogue.env.example ]; then
  cp vmrogue.env.example vmrogue.env
  echo "  Created vmrogue.env — edit KUBECONFIG and VMROGUE_API_KEY before starting."
elif [ -f vmrogue.env ]; then
  echo "  vmrogue.env already exists — not overwritten."
else
  echo "  WARNING: vmrogue.env.example missing"
fi

echo ""
echo "► Step 3/4 — Verify binaries…"
test -x ./vmrogue || { echo "ERROR: ./vmrogue missing"; exit 1; }
./vmrogue --help >/dev/null && echo "  OK: vmrogue binary"

echo ""
echo "► Step 4/4 — Smoke test…"
if [ -x ./test-package.sh ]; then
  ./test-package.sh || true
fi

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  Install complete."
echo ""
echo "  Start API + dashboard:"
echo "    set -a && source vmrogue.env && set +a"
echo "    ./vmrogue api-serve --host 0.0.0.0 --port 5151"
echo ""
echo "  Open:  http://<this-server>:5151/dashboard"
echo ""
echo "  Cluster (once per K8s cluster — before VMs work):"
echo "    export KUBECONFIG=...   # same path as in vmrogue.env"
echo "    ./install-cluster.sh    # Cilium + KubeVirt + CDI"
echo "    ./apply-cluster-network.sh   # after in-cluster deploy (Cilium)"
echo "    ./test-cluster.sh"
echo "  Docs: CLUSTER_SETUP.txt  PREREQUISITES.txt"
echo "  Test:  ./test-package.sh"
echo "  Remove: ./uninstall.sh --yes  (add --remove-dir to delete this folder)"
echo "══════════════════════════════════════════════════════════"
