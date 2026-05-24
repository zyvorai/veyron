#!/usr/bin/env bash
# Refresh scripts/zyvor-branding kit from VMRogue dashboard-next (canonical Zyvor UI).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
KIT="${ROOT}/scripts/zyvor-branding"
SRC="${ROOT}/web/dashboard-next/src/sdk"

cp -f "${SRC}/components/ZyvorAboutModal.tsx" "${KIT}/ZyvorAboutModal.tsx"
cp -f "${SRC}/components/ZyvorBrand.tsx" "${KIT}/ZyvorBrand.tsx"
cp -f "${SRC}/components/PremiumLoginShell.tsx" "${KIT}/PremiumLoginShell.tsx"
cp -f "${SRC}/zyvor-premium-login.css" "${KIT}/zyvor-premium-login.css"

echo "Synced Zyvor kit from VMRogue dashboard-next → ${KIT}"
