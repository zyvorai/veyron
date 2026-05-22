#!/usr/bin/env bash
# Propagate Zyvor install UX + branding assets to all product repos.
set -euo pipefail
KIT="$(cd "$(dirname "$0")" && pwd)"
TT="$(cd "${KIT}/../../.." && pwd)"  # …/VMRogue/scripts/zyvor-branding → repo root's parent (tt)

# All ssahani + hypersdk products under ~/tt (see sync-zyvor-branding.sh for web paths).
REPOS=(
  VMRogue v9s machina guestkit hypersdk- hyper2kvm- packetwolf ragnarok Aether IronWolf forge
  vmspawn nightforge hypersdk-web hypercluster
)

vmrogue_root="$(cd "${KIT}/../.." && pwd)"
for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  [[ -d "${root}" ]] || continue
  mkdir -p "${root}/scripts/zyvor-branding"
  if [[ "${root}" != "${vmrogue_root}" ]]; then
    cp -f "${KIT}/ZYVOR_INSTALL.txt" "${KIT}/ZyvorBrand.tsx" "${KIT}/zyvor-logo.png" \
      "${root}/scripts/zyvor-branding/"
  fi
  cp -f "${KIT}/../lib/package-ui.sh" "${root}/scripts/lib/package-ui.sh" 2>/dev/null || true
  echo "assets → ${repo}"
done

ZYVOR_STEP='  "https://zyvor.dev · © @zyvor 2026" \'
for repo in "${REPOS[@]}"; do
  f="${TT}/${repo}/scripts/lib/package-install.sh"
  [[ -f "${f}" ]] || continue
  if grep -q 'zyvor.dev' "${f}"; then
    continue
  fi
  # shellcheck disable=SC2016
  perl -i -pe 'if (/^pkg_next_steps \\$/ && !$seen++) { $_ .= "  \"https://zyvor.dev · © @zyvor 2026\" \\\n" }' "${f}"
  echo "install → ${repo}"
done

INSTALL_CP=$'cp "\\${BUILD_DIR}/scripts/zyvor-branding/ZYVOR_INSTALL.txt" "\\${STAGE}/ZYVOR_INSTALL.txt" 2>/dev/null || true'
ZYVOR_QS=$'\nPackaged by Zyvor — https://zyvor.dev · © @zyvor 2026'
for repo in "${REPOS[@]}"; do
  f="${TT}/${repo}/scripts/package-binary-remote.sh"
  [[ -f "${f}" ]] || continue
  if grep -q 'ZYVOR_INSTALL.txt' "${f}"; then
    continue
  fi
  if grep -q '^QEOF$' "${f}"; then
    perl -i -0pe "s/\\nQEOF\\n/${ZYVOR_QS}\\nQEOF\\n\\n${INSTALL_CP}\\n/s" "${f}"
  elif grep -q "cat > \"\\\${STAGE}/QUICKSTART.txt\" <<'Q'" "${f}"; then
    perl -i -0pe "s/\\nQ\\n/${ZYVOR_QS}\\nQ\\n\\n${INSTALL_CP}\\n/s" "${f}"
  fi
  echo "package → ${repo}"
done

echo "Done."
