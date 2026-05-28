#!/usr/bin/env bash
# Full Zyvor branding alignment under ~/tt — run from any machine with this repo.
set -euo pipefail
KIT="$(cd "$(dirname "$0")" && pwd)"
TT="$(cd "${KIT}/../../.." && pwd)"
BRAND='zyvor.dev · HyperSDK · © 2026'

echo "==> Propagate ZyvorBrand.tsx + bundle scripts"
"${KIT}/apply-zyvor-branding-ui.sh"

echo "==> Global text replace (@zyvor → standard line)"
while IFS= read -r -d '' f; do
  grep -qE '@zyvor|© @zyvor' "${f}" 2>/dev/null || continue
  sed -i '' \
    -e 's|https://zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' \
    -e 's|zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' \
    -e "s|© @zyvor 2026 · HyperSDK|HyperSDK · © 2026|g" \
    -e "s|© @zyvor 2026 · <a href=\"https://zyvor.dev\"|zyvor.dev · HyperSDK · © 2026 · <a href=\"https://zyvor.dev\"|g" \
    -e 's|© @zyvor 2026|© 2026|g' \
    "${f}" 2>/dev/null || sed -i \
    -e 's|https://zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' \
    -e 's|zyvor.dev · HyperSDK · © 2026|zyvor.dev · HyperSDK · © 2026|g' \
    -e "s|© @zyvor 2026 · HyperSDK|HyperSDK · © 2026|g" \
    -e 's|© @zyvor 2026|© 2026|g' \
    "${f}"
done < <(find "${TT}" -type f \( -name '*.txt' -o -name '*.sh' -o -name '*.tsx' -o -name '*.ts' -o -name '*.json' -o -name '*.html' \) \
  ! -path '*/node_modules/*' ! -path '*/dist/*' ! -path '*/.git/*' ! -path '*/target/*' ! -name 'align-all-zyvor-branding.sh' -print0 2>/dev/null)

echo "==> hypersdk (tt/hypersdk) web Footer"
HS="${TT}/tt/hypersdk"
if [[ -d "${HS}/web/dashboard-react/src/components" ]]; then
  cp -f "${KIT}/ZyvorBrand.tsx" "${HS}/web/dashboard-react/src/components/ZyvorBrand.tsx"
  cp -f "${KIT}/ZyvorBrand.tsx" "${HS}/scripts/zyvor-branding/ZyvorBrand.tsx"
  perl -i -pe '
    s/\{\x27 · \x27\}© @zyvor 2026 · HyperSDK/ · HyperSDK · © 2026/g;
    s/© @zyvor 2026 · <a href="https:\/\/zyvor.dev"[^>]*>zyvor.dev<\/a> · HyperSDK/
      <a href="https:\/\/zyvor.dev" target="_blank" rel="noopener noreferrer" style={{ color: '\''#f0583a'\'', textDecoration: '\''none'\'' }}>zyvor.dev<\/a> · HyperSDK · © 2026/g;
  ' "${HS}/web/dashboard-react/src/components/Footer.tsx" 2>/dev/null || true
fi

echo "==> Strip header ZyvorInline + empty wrappers"
fix_tsx() {
  perl -i -0777 -pe '
    s/\s*<div[^>]*hidden lg:flex[^>]*>\s*<ZyvorInline[^>]*\/>\s*<\/div>\s*//gs;
    s/\s*<ZyvorInline[^>]*\/>\s*//g;
    s/^import \{ ZyvorInline \} from [^\n]+\n//gm;
  ' "$1" 2>/dev/null || true
}
while IFS= read -r -d '' f; do
  fix_tsx "${f}"
done < <(find "${TT}" -name '*.tsx' ! -path '*/node_modules/*' ! -path '*/dist/*' -print0 2>/dev/null)

echo "==> Ensure ZyvorFooter on main shells"
add_footer() {
  local f="$1" import_path="$2"
  [[ -f "${f}" ]] || return 0
  grep -q 'ZyvorFooter' "${f}" && return 0
  grep -q '</main>' "${f}" || return 0
  if ! grep -q "from '${import_path}'" "${f}" && ! grep -q "from \"${import_path}\"" "${f}"; then
    perl -i -pe "if (!\$done++ && /^import /) { \$_ .= \"import { ZyvorFooter } from '${import_path}';\n\" }" "${f}"
  fi
  perl -i -0777 -pe 's/(<\/main>)/$1\n      <ZyvorFooter \/>/s unless /ZyvorFooter/' "${f}"
}

add_footer "${TT}/machina/web/src/App.tsx" './components/ZyvorBrand'
add_footer "${TT}/vmspawn/.web/src/App.tsx" './components/ZyvorBrand'
add_footer "${TT}/hypersdk-/web/dashboard-react/src/App.tsx" './components/ZyvorBrand'
add_footer "${TT}/hyper2kvm-/web/dashboard/src/components/Layout.tsx" './ZyvorBrand'

echo "Done."

# hypersdk-web i18n footers
for f in "${TT}/hypersdk-web/i18n"/*/docusaurus-theme-classic/footer.json; do
  [[ -f "${f}" ]] || continue
  perl -i -pe 's|© @zyvor 2026 · HyperSDK Platform|HyperSDK · © 2026|g' "${f}" 2>/dev/null || true
done

echo "==> Done. Standard line: ${BRAND}"
