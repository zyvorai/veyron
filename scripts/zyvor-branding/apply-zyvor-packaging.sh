#!/usr/bin/env bash
# Propagate Zyvor install UX + branding assets to all product repos.
set -euo pipefail
KIT="$(cd "$(dirname "$0")" && pwd)"
TT="$(cd "${KIT}/../../.." && pwd)"  # …/Veyron/scripts/zyvor-branding → repo root's parent (tt)

REPOS=(
  v9s machina guestkit hypersdk- hyper2kvm- packetwolf ragnarok Aether IronWolf forge
  vmspawn nightforge hypersdk-web hypercluster
)

# repo → "public_dir:components_dir" (either may be empty)
declare -A WEB_PUBLIC=(
  [v9s]="ui/public:ui/src/components"
  [machina]="web/public:web/src/components"
  [hypersdk-]="web/dashboard-react/public:web/dashboard-react/src/components"
  [hyper2kvm-]="web/dashboard/public:web/dashboard/src/components"
  [packetwolf]="web-ui/public:web-ui/src/components"
  [ragnarok]="frontend/public:frontend/src/components"
  [Aether]="web/dashboard/public:web/dashboard/src/components"
  [IronWolf]="web/dashboard/public:web/dashboard/src/components"
  [forge]="web-ui/public:web-ui/src/components"
  [vmspawn]=".web/public:.web/src/components"
  [hypersdk-web]="static/img:"
  [hypercluster]=":"
  [guestkit]=":"
  [nightforge]=":"
)

veyron_root="$(cd "${KIT}/../.." && pwd)"
for repo in "${REPOS[@]}"; do
  root="${TT}/${repo}"
  [[ -d "${root}" ]] || continue
  mkdir -p "${root}/scripts/zyvor-branding"
  if [[ "${root}" != "${veyron_root}" ]]; then
    cp -f "${KIT}/ZYVOR_INSTALL.txt" "${KIT}/ZyvorBrand.tsx" "${KIT}/zyvor-logo.png" \
      "${root}/scripts/zyvor-branding/"
  fi
  LIB_SRC="${KIT}/../lib"
  for libf in package-ui.sh install-everything.sh write-customer-help.sh START_HERE.txt \
    package-bundle-client.sh package-cluster-test.sh package-host-test.sh package-cluster-prereqs-run.sh; do
    [[ -f "${LIB_SRC}/${libf}" ]] && cp -f "${LIB_SRC}/${libf}" "${root}/scripts/lib/${libf}" 2>/dev/null || true
  done
  [[ -f "${LIB_SRC}/write-customer-help.sh" ]] && chmod +x "${root}/scripts/lib/write-customer-help.sh" 2>/dev/null || true

  pair="${WEB_PUBLIC[$repo]:-}"
  if [[ -n "${pair}" ]]; then
    pub="${pair%%:*}"
    comp="${pair#*:}"
    if [[ -n "${pub}" ]]; then
      mkdir -p "${root}/${pub}"
      cp -f "${KIT}/zyvor-logo.png" "${root}/${pub}/zyvor-logo.png"
    fi
    if [[ -n "${comp}" && -d "${root}/$(dirname "${comp}")" ]]; then
      mkdir -p "${root}/${comp}"
      cp -f "${KIT}/ZyvorBrand.tsx" "${root}/${comp}/ZyvorBrand.tsx"
    fi
  fi
  # Embedded API dashboards (same folder as dashboard.html)
  for embed in src/api/web; do
    if [[ -d "${root}/${embed}" ]]; then
      cp -f "${KIT}/zyvor-logo.png" "${root}/${embed}/zyvor-logo.png" 2>/dev/null || true
    fi
  done
  echo "assets → ${repo}"
done

ZYVOR_STEP='  "zyvor.dev · HyperSDK · © 2026" \'
for repo in "${REPOS[@]}"; do
  f="${TT}/${repo}/scripts/lib/package-install.sh"
  [[ -f "${f}" ]] || continue
  if grep -q 'zyvor.dev' "${f}"; then
    continue
  fi
  # shellcheck disable=SC2016
  perl -i -pe 'if (/^pkg_next_steps \\$/ && !$seen++) { $_ .= "  \"zyvor.dev · HyperSDK · © 2026\" \\\n" }' "${f}"
  echo "install → ${repo}"
done

INSTALL_CP=$'cp "\\${BUILD_DIR}/scripts/zyvor-branding/ZYVOR_INSTALL.txt" "\\${STAGE}/ZYVOR_INSTALL.txt" 2>/dev/null || true'
ZYVOR_QS=$'\nPackaged by Zyvor — zyvor.dev · HyperSDK · © 2026'
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

SCRIPTS_SRC="${KIT}/.."
for script in rebuild-all-customer-tarballs-remote.sh test-customer-e2e-remote.sh test-packages-remote-only.sh; do
  [[ -f "${SCRIPTS_SRC}/${script}" ]] || continue
  for repo in Veyron v9s machina guestkit hypersdk- hyper2kvm- packetwolf ragnarok Aether IronWolf forge; do
    root="${TT}/${repo}"
    [[ -d "${root}/scripts" ]] || continue
    cp -f "${SCRIPTS_SRC}/${script}" "${root}/scripts/${script}"
    chmod +x "${root}/scripts/${script}"
  done
  echo "customer-scripts → ${script}"
done

echo "Done."
