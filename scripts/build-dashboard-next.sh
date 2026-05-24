#!/usr/bin/env bash
# Build dashboard-next (Vite) and stage static files for rust-embed under src/api/web/dashboard-next/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UI="${ROOT}/web/dashboard-next"
OUT="${ROOT}/src/api/web/dashboard-next"

cd "${UI}"
if [[ -f package-lock.json ]]; then
  npm ci
else
  npm install
fi
npm run build

rm -rf "${OUT}"
mkdir -p "${OUT}"
cp -a dist/. "${OUT}/"
echo "Staged dashboard-next → ${OUT} ($(find "${OUT}" -type f | wc -l | tr -d ' ') files)"
