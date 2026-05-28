#!/usr/bin/env bash
# Propagate ZyvorBrand.tsx + move copyright to footer across tt/* dashboards.
set -euo pipefail
KIT="$(cd "$(dirname "$0")" && pwd)"
TT="$(cd "${KIT}/../../.." && pwd)"

REPOS=(
  v9s machina guestkit hypersdk- hyper2kvm- packetwolf ragnarok Aether IronWolf forge
  vmspawn nightforge hypersdk-web hypercluster
)

declare -A WEB_COMP=(
  [v9s]="ui/src/components"
  [machina]="web/src/components"
  [hypersdk-]="web/dashboard-react/src/components"
  [hyper2kvm-]="web/dashboard/src/components"
  [packetwolf]="web-ui/src/components"
  [ragnarok]="frontend/src/components"
  [Aether]="web/dashboard/src/components"
  [IronWolf]="web/dashboard/src/components"
  [forge]="web-ui/src/components"
  [vmspawn]=".web/src/components"
)

BRAND_LINE='zyvor.dev · HyperSDK · © 2026'
OLD_PAT='zyvor\.dev · © 2026'

for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  comp="${WEB_COMP[$repo]:-}"
  [[ -n "${comp}" && -d "${root}/${comp}" ]] || continue
  cp -f "${KIT}/ZyvorBrand.tsx" "${root}/${comp}/ZyvorBrand.tsx"
  echo "ZyvorBrand → ${repo}/${comp}"
  if [[ -f "${KIT}/ZyvorAboutModal.tsx" ]]; then
    cp -f "${KIT}/ZyvorAboutModal.tsx" "${root}/${comp}/ZyvorAboutModal.tsx"
    echo "ZyvorAboutModal → ${repo}/${comp}"
  fi
done

# scripts/zyvor-branding copy
for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  [[ -d "${root}/scripts/zyvor-branding" ]] || continue
  [[ "${root}/scripts/zyvor-branding/ZyvorBrand.tsx" == "${KIT}/ZyvorBrand.tsx" ]] && continue
  cp -f "${KIT}/ZyvorBrand.tsx" "${root}/scripts/zyvor-branding/ZyvorBrand.tsx"
done

# Text bundles: START_HERE, package-ui, etc.
for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  [[ -d "${root}/scripts/lib" ]] || continue
  while IFS= read -r -d '' f; do
    if grep -q '@zyvor' "${f}" 2>/dev/null || grep -q '© @zyvor' "${f}" 2>/dev/null; then
      sed -i '' "s|zyvor.dev · HyperSDK · © 2026|${BRAND_LINE}|g" "${f}" 2>/dev/null || \
        sed -i "s|zyvor.dev · HyperSDK · © 2026|${BRAND_LINE}|g" "${f}"
      sed -i '' "s|${OLD_PAT}|${BRAND_LINE}|g" "${f}" 2>/dev/null || \
        sed -i "s|${OLD_PAT}|${BRAND_LINE}|g" "${f}"
    fi
  done < <(find "${root}/scripts" -type f \( -name '*.sh' -o -name '*.txt' \) -print0 2>/dev/null)
done

# hypersdk-web docusaurus footer
for f in "${TT}/hypersdk-web/i18n"/*/docusaurus-theme-classic/footer.json; do
  [[ -f "${f}" ]] || continue
  sed -i '' 's|HyperSDK · © 2026 Platform|HyperSDK · © 2026|g' "${f}" 2>/dev/null || \
    sed -i 's|HyperSDK · © 2026 Platform|HyperSDK · © 2026|g' "${f}"
done
if [[ -f "${TT}/hypersdk-web/docusaurus.config.ts" ]]; then
  sed -i '' 's|HyperSDK · © 2026 Platform|HyperSDK · © 2026|g' "${TT}/hypersdk-web/docusaurus.config.ts" 2>/dev/null || \
    sed -i 's|HyperSDK · © 2026 Platform|HyperSDK · © 2026|g' "${TT}/hypersdk-web/docusaurus.config.ts"
fi

# TSX: strip header ZyvorInline (with product), remove empty wrapper divs
fix_tsx() {
  local f="$1"
  [[ -f "${f}" ]] || return 0
  perl -i -0777 -pe '
    s/\s*<div[^>]*hidden lg:flex[^>]*>\s*<ZyvorInline[^/]*\/>\s*<\/div>\s*//gs;
    s/\s*<ZyvorInline[^/]*\/>\s*//g;
    s/\s*<p[^>]*>\s*<\/p>\s*//gs;
  ' "${f}" 2>/dev/null || true
}

for repo in "${REPOS[@]}"; do
  comp="${WEB_COMP[$repo]:-}"
  [[ -n "${comp}" ]] || continue
  while IFS= read -r -d '' f; do
    fix_tsx "${f}"
  done < <(find "${TT}/${repo}/${comp}" -name '*.tsx' -print0 2>/dev/null)
  # App / Layout shells
  while IFS= read -r -d '' f; do
    fix_tsx "${f}"
    if grep -q '</main>' "${f}" && ! grep -q 'ZyvorFooter' "${f}"; then
      if grep -q "from './components/ZyvorBrand'" "${f}" 2>/dev/null; then
        :
      elif grep -q 'ZyvorBrand' "${f}" 2>/dev/null; then
        :
      else
        # add import + footer after </main> when file is a layout shell
        if grep -qE '(MainLayout|function Layout|export default function Layout|flex flex-col min-h-screen)' "${f}"; then
          perl -i -pe '
            if (!$done++ && /from .react./) { $_ .= "import { ZyvorFooter } from '\''./components/ZyvorBrand'\'';\n" }
          ' "${f}" 2>/dev/null || true
        fi
      fi
      if grep -q 'ZyvorFooter' "${f}"; then
        :
      elif grep -qE '(MainLayout|export default function Layout)' "${f}" && grep -q '</main>' "${f}"; then
        perl -i -0777 -pe 's/(<\/main>)/$1\n      <ZyvorFooter \/>/s unless /ZyvorFooter/' "${f}" 2>/dev/null || true
      fi
    fi
  done < <(find "${TT}/${repo}" \( -name 'App.tsx' -o -name 'Layout.tsx' -o -name 'MainLayout.tsx' -o -name 'EnhancedLayout.tsx' -o -name 'HyperPageTabs.tsx' \) -print0 2>/dev/null)
done

# Update apply-zyvor-packaging.sh branding strings
if [[ -f "${KIT}/apply-zyvor-packaging.sh" ]]; then
  sed -i '' 's|zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' "${KIT}/apply-zyvor-packaging.sh" 2>/dev/null || \
    sed -i 's|zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' "${KIT}/apply-zyvor-packaging.sh"
fi

echo "Done — UI branding applied under ${TT}"
