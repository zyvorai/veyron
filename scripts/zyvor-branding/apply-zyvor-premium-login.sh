#!/usr/bin/env bash
# Propagate PremiumLoginShell.tsx + zyvor-premium-login.css across tt/* dashboards.
set -euo pipefail
KIT="$(cd "$(dirname "$0")" && pwd)"
TT="$(cd "${KIT}/../../.." && pwd)"

REPOS=(
  v9s machina guestkit transiva- h2kvm- packetwolf ragnarok Axiom IronWolf forge vmspawn
)

declare -A WEB_COMP=(
  [v9s]="ui/src/components"
  [machina]="web/src/components"
  [transiva-]="web/dashboard-react/src/components"
  [h2kvm-]="web/dashboard/src/components"
  [packetwolf]="web-ui/src/components"
  [ragnarok]="frontend/src/components"
  [Axiom]="web/dashboard/src/components"
  [IronWolf]="web/dashboard/src/components"
  [forge]="web-ui/src/components"
  [vmspawn]=".web/src/components"
)

declare -A CSS_ENTRY=(
  [v9s]="ui/src/index.css"
  [machina]="web/src/styles/main.css"
  [transiva-]="web/dashboard-react/src/index.css"
  [h2kvm-]="web/dashboard/src/index.css"
  [packetwolf]="web-ui/src/index.css"
  [ragnarok]="frontend/src/index.css"
  [Axiom]="web/dashboard/src/index.css"
  [IronWolf]="web/dashboard/src/index.css"
  [forge]="web-ui/src/index.css"
  [vmspawn]=".web/src/index.css"
)

IMPORT_LINE="@import './zyvor-premium-login.css';"

for repo in "${REPOS[@]}"; do
  comp="${WEB_COMP[$repo]:-}"
  css="${CSS_ENTRY[$repo]:-}"
  root="${TT}/${repo}"
  [[ -n "${comp}" && -d "${root}/${comp}" ]] || continue

  cp -f "${KIT}/PremiumLoginShell.tsx" "${root}/${comp}/PremiumLoginShell.tsx"
  echo "PremiumLoginShell → ${repo}/${comp}"
  if [[ -f "${KIT}/ZyvorAboutModal.tsx" ]]; then
    cp -f "${KIT}/ZyvorAboutModal.tsx" "${root}/${comp}/ZyvorAboutModal.tsx"
    echo "ZyvorAboutModal → ${repo}/${comp}"
  fi
  if [[ -f "${KIT}/ZyvorBrand.tsx" ]]; then
    cp -f "${KIT}/ZyvorBrand.tsx" "${root}/${comp}/ZyvorBrand.tsx"
    echo "ZyvorBrand → ${repo}/${comp}"
  fi

  css_dir="$(dirname "${root}/${css}")"
  cp -f "${KIT}/zyvor-premium-login.css" "${css_dir}/zyvor-premium-login.css"

  if [[ "${repo}" == "machina" ]]; then
  echo "  (machina keeps login CSS in main.css — skip import)"
    continue
  fi

  if [[ -f "${root}/${css}" ]] && ! grep -q 'zyvor-premium-login.css' "${root}/${css}"; then
    # shellcheck disable=SC2016
    perl -i -pe 'print "$ENV{IMPORT_LINE}\n" if $. == 1 && !/zyvor-premium-login/' "${root}/${css}" 2>/dev/null || \
      sed -i '' "1i\\
${IMPORT_LINE}
" "${root}/${css}" 2>/dev/null || \
      sed -i "1i ${IMPORT_LINE}" "${root}/${css}"
    echo "  CSS import → ${repo}/${css}"
  fi
done

# tt/transiva mirror
if [[ -d "${TT}/tt/zyvor-web/dashboard-react/src/components" ]]; then
  cp -f "${KIT}/PremiumLoginShell.tsx" "${TT}/tt/zyvor-web/dashboard-react/src/components/PremiumLoginShell.tsx"
  cp -f "${KIT}/zyvor-premium-login.css" "${TT}/tt/zyvor-web/dashboard-react/src/zyvor-premium-login.css"
  css="${TT}/tt/zyvor-web/dashboard-react/src/index.css"
  if [[ -f "${css}" ]] && ! grep -q 'zyvor-premium-login.css' "${css}"; then
    sed -i '' "1i\\
@import './zyvor-premium-login.css';
" "${css}" 2>/dev/null || sed -i "1i @import './zyvor-premium-login.css';" "${css}"
  fi
  echo "PremiumLoginShell → tt/transiva"
fi

echo "Done — premium login kit under ${TT}"
