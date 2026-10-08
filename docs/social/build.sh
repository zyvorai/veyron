#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Render the README and social cards (docs/social/*.html) into docs/assets/.
# Needs Google Chrome and macOS `sips`.
#   ./docs/social/build.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="$HERE/../assets"
mkdir -p "$OUT"
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
[[ -x "$CHROME" ]] || { echo "Google Chrome not found (set CHROME=...)" >&2; exit 1; }
TMP="$(mktemp -d "${TMPDIR:-/tmp}/veyron-social.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

render() { # <html> <output> <width> <height>
  local png="$TMP/${2%.*}.png"
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --allow-file-access-from-files --window-size="$3,$4" --screenshot="$png" "file://$HERE/$1" >/dev/null 2>&1
  case "$2" in
    *.jpg) sips -s format jpeg -s formatOptions 90 "$png" --out "$OUT/$2" >/dev/null ;;
    *) cp "$png" "$OUT/$2" ;;
  esac
  echo "wrote docs/assets/$2 ($(du -k "$OUT/$2" | cut -f1) KB)"
}

[[ -f "$OUT/console-mission.png" ]] || echo "warn: docs/assets/console-mission.png missing; the hero frame will be empty" >&2

render share-card.html veyron-share-card.png 1200 630
render hero.html readme-hero.jpg 1600 800
render capabilities.html readme-capabilities.jpg 1600 600
render architecture.html readme-architecture.jpg 1600 700
render stack.html readme-stack.jpg 1600 900
render path.html readme-path.jpg 1600 900
render benchmark.html readme-benchmark.jpg 1600 900
render ebpf.html readme-ebpf.jpg 1600 820
