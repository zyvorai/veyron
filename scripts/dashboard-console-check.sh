#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Headless dashboard console-error sweep — wrapper for dashboard-console-check.cjs.
# Drives real Chrome against a deployed Veyron dashboard and reports per-page
# console errors / uncaught page errors / failed requests. Exits non-zero on any
# error, so it works as a post-deploy smoke test or CI regression gate.
#
# Requires: node, npm, and Google Chrome (uses playwright-core's `channel: chrome`,
# so no browser download). playwright-core is installed once into a temp cache.
#
# Usage:
#   ./scripts/dashboard-console-check.sh --host HOST
#   ./scripts/dashboard-console-check.sh --host HOST --screenshot ./out
#   VEYRON_API_KEY=... ./scripts/dashboard-console-check.sh --host H --port 30151
#   ./scripts/dashboard-console-check.sh --host H --page snapshots --screenshot .
#
# All flags are forwarded to the .cjs (see its header): --host --port --key
# --pages --page --screenshot --scheme.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PW_VERSION="${PW_VERSION:-1.60.0}"
CACHE="${VEYRON_PW_CACHE:-${TMPDIR:-/tmp}/veyron-dashboard-check}"

command -v node >/dev/null 2>&1 || { echo "error: node not found" >&2; exit 3; }
command -v npm  >/dev/null 2>&1 || { echo "error: npm not found"  >&2; exit 3; }

if [[ ! -d "${CACHE}/node_modules/playwright-core" ]]; then
  echo "› installing playwright-core@${PW_VERSION} into ${CACHE} (one-time)…" >&2
  mkdir -p "${CACHE}"
  ( cd "${CACHE}" && npm init -y >/dev/null 2>&1 && npm install "playwright-core@${PW_VERSION}" >/dev/null 2>&1 )
fi

PW_CORE="${CACHE}/node_modules/playwright-core" \
  exec node "${SCRIPT_DIR}/dashboard-console-check.cjs" "$@"
