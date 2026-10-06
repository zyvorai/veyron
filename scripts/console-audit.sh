#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Headless audit of the React console (/console) — wrapper for console-audit.cjs.
#
# Signs in with VEYRON_API_KEY, visits every page in the console menu and reports:
#   console    JavaScript errors and uncaught exceptions
#   api        API responses with status >= 500
#   empty      pages that render no content
#   overflow   whole-page horizontal overflow
#   offscreen  controls running past the right viewport edge
#   overlaps   content under the fixed top nav
#   clipped    overflow-hidden containers slicing controls off with no scrollbar
#
# Exits non-zero if any page reports a finding, so it works as a post-deploy gate.
#
# Requires: node, npm, Google Chrome (playwright-core, channel: chrome).
#
# Usage:
#   VEYRON_API_KEY=... ./scripts/console-audit.sh --host 203.0.113.10
#   ./scripts/console-audit.sh --host H --width 1280,1920 --pages vms,ai,proposals
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PW_VERSION="${PW_VERSION:-1.60.0}"
CACHE="${VEYRON_PW_CACHE:-${TMPDIR:-/tmp}/veyron-console-audit}"

HOST=""; PORT="30151"; WIDTHS="1280,1920"; PAGES=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)  HOST="$2"; shift 2 ;;
    --port)  PORT="$2"; shift 2 ;;
    --width) WIDTHS="$2"; shift 2 ;;
    --pages) PAGES="$2"; shift 2 ;;
    -h|--help) sed -n '5,22p' "$0"; exit 0 ;;
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
  echo "── console audit https://${HOST}:${PORT}/console @ ${w}px ──"
  out="$(PW_CORE="${CACHE}/node_modules/playwright-core" \
    node "${SCRIPT_DIR}/console-audit.cjs" "$HOST" "$PORT" "$w" "$PAGES")" || { echo "  ✗ audit run failed" >&2; rc=1; continue; }
  n="$(printf '%s' "$out" | node -e '
    let s=""; process.stdin.on("data",d=>s+=d).on("end",()=>{
      const d=JSON.parse(s); const f=d.findings||{}; const bad=new Set(Object.keys(f));
      for (const [p,r] of Object.entries(f))
        for (const [k,items] of Object.entries(r)) for (const it of items) console.error(`  ${p}: ${k} ${it}`);
      for (const [p,v] of Object.entries(d.consoleErrors||{})) {
        if (!v.length) continue; bad.add(p);
        for (const m of v) console.error(`  ${p}: console ${m}`);
      }
      console.error(`  ${d.pages.length} pages checked`);
      console.log(bad.size);
    });')"
  if [[ "$n" != "0" ]]; then echo "  ✗ ${n} page(s) with findings" >&2; rc=1; else echo "  ✓ clean"; fi
done
exit $rc
