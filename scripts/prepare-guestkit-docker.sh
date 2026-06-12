#!/usr/bin/env bash
# Copy sibling guestkit repo into Docker build context (../guestkit -> ./guestkit).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${GUESTKIT_SRC:-$(cd "$ROOT/../guestkit" 2>/dev/null && pwd || true)}"
DEST="$ROOT/guestkit"
if [[ -z "$SRC" || ! -d "$SRC" ]]; then
  echo "guestkit source not found — set GUESTKIT_SRC or place repo at ../guestkit" >&2
  exit 1
fi
mkdir -p "$DEST"
rsync -a --delete --exclude target --exclude .git "$SRC/" "$DEST/"
echo "Prepared $DEST from $SRC"
