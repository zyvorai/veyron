#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# Headless dashboard LAYOUT audit — wrapper for dashboard-layout-audit.cjs.
#
# Complements dashboard-console-check.sh: that one catches JS/console/network
# regressions, this one catches *geometry* regressions — the class of bug where
# the UI renders without erroring but content is unreachable:
#
#   overlaps   content colliding with the fixed menubar
#   clipped    containers whose scrollWidth exceeds clientWidth while
#              overflow is hidden (controls sliced off with no scrollbar and
#              no overflow menu — silent, and invisible to a console check)
#   overflow   whole-page horizontal overflow
#   offscreen  visible elements running past the right viewport edge
#
# Exits non-zero if any page reports a finding, so it works as a post-deploy
# gate. Sweep several widths — the menubar only oversubscribes below ~2000px.
#
# Requires: node, npm, Google Chrome (playwright-core, channel: chrome).
#
# Usage:
#   ./scripts/dashboard-layout-audit.sh --host 80.79.5.173
#   ./scripts/dashboard-layout-audit.sh --host H --width 1280 --tier advanced
#   VEYRON_API_KEY=... ./scripts/dashboard-layout-audit.sh --host H --width 1280,1920
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PW_VERSION="${PW_VERSION:-1.60.0}"
CACHE="${VEYRON_PW_CACHE:-${TMPDIR:-/tmp}/veyron-dashboard-check}"

HOST=""; PORT="30151"; WIDTHS="1280,1920"; TIER="advanced"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)  HOST="$2"; shift 2 ;;
    --port)  PORT="$2"; shift 2 ;;
    --width) WIDTHS="$2"; shift 2 ;;
    --tier)  TIER="$2"; shift 2 ;;
    -h|--help) sed -n '5,26p' "$0"; exit 0 ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$HOST" ]] || { echo "error: --host is required" >&2; exit 2; }

command -v node >/dev/null 2>&1 || { echo "error: node not found" >&2; exit 3; }
command -v npm  >/dev/null 2>&1 || { echo "error: npm not found"  >&2; exit 3; }

if [[ ! -d "${CACHE}/node_modules/playwright-core" ]]; then
  echo "› installing playwright-core@${PW_VERSION} into ${CACHE} (one-time)…" >&2
  mkdir -p "${CACHE}"
  ( cd "${CACHE}" && npm init -y >/dev/null 2>&1 && npm install "playwright-core@${PW_VERSION}" >/dev/null 2>&1 )
fi

rc=0
IFS=',' read -ra WS <<< "$WIDTHS"
for w in "${WS[@]}"; do
  echo "── layout audit ${HOST}:${PORT} @ ${w}px tier=${TIER} ──"
  out="$(PW_CORE="${CACHE}/node_modules/playwright-core" \
    node "${SCRIPT_DIR}/dashboard-layout-audit.cjs" "$HOST" "$PORT" "$w" "$TIER")"
  n="$(printf '%s' "$out" | node -e '
    let s=""; process.stdin.on("data",d=>s+=d).on("end",()=>{
      const d=JSON.parse(s); const f=d.findings||{};
      const pages=Object.keys(f);
      for (const p of pages) {
        const r=f[p];
        for (const k of ["overlaps","clipped","offscreen","overflow"])
          for (const it of (r[k]||[])) console.error(`  ${p}: ${k} ${JSON.stringify(it)}`);
      }
      const ce=Object.entries(d.consoleErrors||{}).filter(([,v])=>v.length);
      for (const [p,v] of ce) console.error(`  ${p}: console ${v[0]}`);
      console.log(pages.length);
    });')"
  if [[ "$n" != "0" ]]; then echo "  ✗ ${n} page(s) with layout findings" >&2; rc=1; else echo "  ✓ clean"; fi
done
exit $rc
