#!/usr/bin/env bash
# Commit branding alignment in every git repo under tt/ that has changes.
set -euo pipefail
TT="$(cd "$(dirname "$0")/../../.." && pwd)"
MSG="$(cat <<'EOF'
chore(ui): align Zyvor branding — footer-only zyvor.dev · HyperSDK · © 2026

Remove redundant product name from headers; copyright only in footer.
Sync ZyvorBrand.tsx and client bundle script strings.
EOF
)"

REPOS=(
  Aether cockpit forge guestkit hyper2kvm- hypercluster hypersdk- hypersdk-web
  IronWolf machina mkosi-kernel nightforge packetwolf pixie ragnarok tetragon v9s VMRogue vmspawn
)

for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  [[ -d "${root}/.git" ]] || continue
  if git -C "${root}" diff --quiet && git -C "${root}" diff --cached --quiet; then
    [[ -z "$(git -C "${root}" status -u --porcelain)" ]] && continue
  fi
  echo "==> commit ${repo}"
  git -C "${root}" add -A
  git -C "${root}" commit -m "${MSG}" || echo "  (skip: nothing to commit)"
done

# Nested tt/ repos
for root in "${TT}"/tt/*/; do
  [[ -d "${root}/.git" ]] || continue
  name="$(basename "${root}")"
  if git -C "${root}" diff --quiet 2>/dev/null && [[ -z "$(git -C "${root}" status -u --porcelain 2>/dev/null)" ]]; then
    continue
  fi
  echo "==> commit tt/${name}"
  git -C "${root}" add -A
  git -C "${root}" commit -m "${MSG}" || echo "  (skip)"
done

echo "Done."
