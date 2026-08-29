#!/usr/bin/env bash
# Copy PacketWolf proprietary LICENSE (ZyvorAI Labs / zyvor.dev) to projects under ../
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/zyvor-canonical.sh
source "${SCRIPT_DIR}/lib/zyvor-canonical.sh"
CANONICAL_ROOT="$(zyvor_canonical_root "${SCRIPT_DIR}")"
SOURCE_LICENSE="${CANONICAL_ROOT}/LICENSE"
PARENT="$(cd "${CANONICAL_ROOT}/.." && pwd)"

if [ ! -f "$SOURCE_LICENSE" ]; then
    echo "ERROR: Missing ${SOURCE_LICENSE}" >&2
    exit 1
fi

# Ssahani-owned repos under tt/ (exclude third-party mirrors and guestkit).
REPOS=(
    Axiom
    cockpit
    forge
    h2kvm-
    hypercluster
    transiva-
    zyvor-web
    IronWolf
    machina
    mkosi-kernel
    nightforge
    packetwolf
    ragnarok
    v9s
    Veyron
    vmspawn
)

echo "Source: ${SOURCE_LICENSE}"
echo "Parent: ${PARENT}"
echo "Excluded (keep OSS license files): guestkit (LGPL), tt/cloud-netconfig, tt/h2kvm,"
echo "  tt/transiva, tt/transiva-org-profile, tt/netctl, tt/netevd"
echo ""

for name in "${REPOS[@]}"; do
    dest="${PARENT}/${name}/LICENSE"
    if [ ! -d "${PARENT}/${name}" ]; then
        echo "  skip ${name} (directory missing)"
        continue
    fi
    if [ "$(cd "$(dirname "$SOURCE_LICENSE")" && pwd -P)" = "$(cd "$(dirname "$dest")" && pwd -P)" ] \
        && [ "$(basename "$SOURCE_LICENSE")" = "$(basename "$dest")" ]; then
        echo "  — ${name} (source repo, unchanged)"
        continue
    fi
    cp "$SOURCE_LICENSE" "$dest"
    echo "  ✅ ${dest}"
done

echo ""
echo "Done. Each project should reference LICENSE in releases and customer handoffs."
