#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
#
# Copy the images the landing page uses from docs/assets/ into docs/site/assets/,
# so docs/site/ is a self-contained static site (GitHub Pages artifact).
#   ./docs/site/build.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/../assets"
OUT="$HERE/assets"
mkdir -p "$OUT"
for f in console-mission.png console-vms.png console-dark.png console-login.png veyron-share-card.png; do
  [[ -f "$SRC/$f" ]] || { echo "missing docs/assets/$f" >&2; exit 1; }
  cp "$SRC/$f" "$OUT/$f"
done
cp "$HERE/../social/zyvor-mark.svg" "$OUT/zyvor-mark.svg"
echo "docs/site ready ($(du -sk "$HERE" | cut -f1) KB)"
