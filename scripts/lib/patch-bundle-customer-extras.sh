#!/usr/bin/env bash
# Insert install-everything.sh + product.meta into package-binary-remote.sh (once).
set -euo pipefail
TT="$(cd "$(dirname "$0")/../../.." && pwd)"

patch_repo() {
  local repo="$1" meta="$2"
  local f="${TT}/${repo}/scripts/package-binary-remote.sh"
  [[ -f "${f}" ]] || return 0
  if grep -q 'install-everything.sh' "${f}"; then
    echo "skip ${repo} (already patched)"
    return 0
  fi
  perl -i -0pe 's/(cp "\$\{LIB\}\/package-ui\.sh" "\$\{STAGE\}\/\.package-lib\/"\n)/$1cp "\${LIB}\/install-everything.sh" "\${STAGE}\/"\n/s' "${f}"
  perl -i -pe 's/(chmod \+x "\$\{STAGE\}\/install\.sh"[^\n]*test-package\.sh" )/\1"\\\${STAGE}\/install-everything.sh" /' "${f}" 2>/dev/null || true
  perl -i -pe 's/("\$\{STAGE\}\/test-package\.sh" )/\1"\\\${STAGE}\/install-everything.sh" /' "${f}" 2>/dev/null || true
  # VMRogue uses OUT_DIR/ARTIFACT — skip here
  local stage_var='${STAGE}'
  local lib_var='${LIB}'
  cat >> "${f}.meta.tmp" <<METAEOF

# --- customer extras (install-everything) ---
METAEOF
  echo "patched ${repo} (manual meta may be needed — check file)"
}

# Handled in repo already: VMRogue, machina
for r in v9s guestkit hypersdk- hyper2kvm- packetwolf ragnarok Aether IronWolf forge; do
  f="${TT}/${r}/scripts/package-binary-remote.sh"
  [[ -f "${f}" ]] || continue
  grep -q 'install-everything.sh' "${f}" && { echo "ok ${r}"; continue; }
  echo "NEEDS_PATCH ${r}"
done
