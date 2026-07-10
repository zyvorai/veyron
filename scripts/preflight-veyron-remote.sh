#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Customer-site readiness preflight for a DEPLOYED Veyron API. Reports which
# Day-2 operations the target cluster actually supports (KubeVirt feature gates
# + installed components), flags dev-grade security defaults, and runs the HTTPS
# smoke test. Run this as the go-live acceptance gate at a customer site.
#
# Usage:
#   VEYRON_API_KEY='...' ./scripts/preflight-veyron-remote.sh <host> [https_node_port]
#
# Exits non-zero if the API is unreachable or the smoke test fails.
set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-30151}"
KEY="${VEYRON_API_KEY:-Admin@321}"
[[ -z "${HOST}" ]] && { echo "usage: VEYRON_API_KEY=... $0 <host> [port]" >&2; exit 2; }
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE="https://${HOST}:${PORT}"

c() { curl -sk -H "X-API-Key: ${KEY}" "$@"; }
hr() { printf '%s\n' "────────────────────────────────────────────────"; }
ok(){ printf '  \033[0;32m✔\033[0m %s\n' "$*"; }
warn(){ printf '  \033[0;33m!\033[0m %s\n' "$*"; }
bad(){ printf '  \033[0;31m✘\033[0m %s\n' "$*"; }

echo "Veyron customer-site preflight → ${BASE}"; hr

# 1) API reachable
if ! c -o /dev/null -w '' "${BASE}/health" 2>/dev/null; then
  bad "API not reachable at ${BASE}/health"; exit 1
fi
ok "API reachable"

# 2) Security posture
if [[ "${KEY}" == "Admin@321" ]]; then
  warn "Using the DEFAULT API key 'Admin@321' — set VEYRON_API_KEY / veyron-api-key Secret before go-live"
else
  ok "Non-default API key in use"
fi
c -o /dev/null -w '' "${BASE}/api/v1/health/ready" 2>/dev/null && ok "Readiness endpoint healthy" || warn "Readiness probe not OK"

# 3) Day-2 capabilities (which ops actually work on THIS cluster)
echo; echo "Day-2 capabilities (target cluster):"; hr
caps="$(c "${BASE}/api/v1/platform/capabilities" 2>/dev/null || true)"
if [[ -z "${caps}" || "${caps}" == *'"error"'* ]]; then
  warn "capabilities endpoint unavailable (redeploy API to enable /platform/capabilities)"
else
  echo "${caps}" | python3 - "${caps}" <<'PY' 2>/dev/null || echo "${caps}" | head -c 400
import sys, json
d = json.loads(sys.stdin.read())
print("  kubevirt_version:", d.get("kubevirt_version"))
ops = d.get("day2_ops", {})
for k, v in ops.items():
    mark = "\033[0;32m✔\033[0m" if v else "\033[0;33m—\033[0m"
    print(f"    {mark} {k}: {v}")
comps = d.get("components", {})
print("  components:", ", ".join(f"{k}={v}" for k, v in comps.items()))
gated = [k for k, v in ops.items() if not v]
if gated:
    print("\n  \033[0;33mNote:\033[0m gated/off ops above are conditional — enable the KubeVirt")
    print("  feature gate (VMLiveUpdateFeatures / VolumesUpdateStrategy) or install the")
    print("  component, or expect those Day-2 ops to be unavailable.")
PY
fi

# 4) Functional smoke test
echo; echo "HTTPS API smoke test:"; hr
if [[ -x "${SCRIPT_DIR}/verify-veyron-remote.sh" ]]; then
  VEYRON_API_KEY="${KEY}" "${SCRIPT_DIR}/verify-veyron-remote.sh" "${HOST}" "${PORT}" || { bad "smoke test failed"; exit 1; }
else
  warn "verify-veyron-remote.sh not found; skipping smoke test"
fi

hr
ok "Preflight complete. Also run: VEYRON_API_KEY=... ./scripts/test-vm-daily-ops-remote.sh ${HOST} ${PORT}"
ok "and: VEYRON_API_KEY=... ./scripts/dashboard-console-check.sh --host ${HOST}"
