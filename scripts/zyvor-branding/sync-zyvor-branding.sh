#!/usr/bin/env bash
# Copy zyvor-logo.png into each ssahani / hypersdk product web UI public/ directory.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
LOGO="${ROOT}/zyvor-logo.png"
[[ -f "${LOGO}" ]] || { echo "Missing ${LOGO}" >&2; exit 1; }

declare -a TARGETS=(
  # ssahani suite (tt/)
  "/Users/ssahani/tt/VMRogue/web/dashboard-next/public"
  "/Users/ssahani/tt/v9s/ui/public"
  "/Users/ssahani/tt/machina/web/public"
  "/Users/ssahani/tt/hypersdk-/web/dashboard-react/public"
  "/Users/ssahani/tt/hyper2kvm-/web/dashboard/public"
  "/Users/ssahani/tt/packetwolf/web-ui/public"
  "/Users/ssahani/tt/ragnarok/frontend/public"
  "/Users/ssahani/tt/Aether/web/dashboard/public"
  "/Users/ssahani/tt/IronWolf/web/dashboard/public"
  "/Users/ssahani/tt/forge/web-ui/public"
  "/Users/ssahani/tt/vmspawn/.web/public"
  "/Users/ssahani/tt/hypersdk-web/static/img"
  "/Users/ssahani/tt/hypercluster"
  # embedded API dashboards
  "/Users/ssahani/tt/VMRogue/src/api/web"
  "/Users/ssahani/tt/nightforge/src/api/web"
  # hypersdk org (tt/tt/)
  "/Users/ssahani/tt/tt/hypersdk/web/dashboard-react/public"
  "/Users/ssahani/tt/tt/netevd/src/web"
)

for dir in "${TARGETS[@]}"; do
  mkdir -p "${dir}"
  cp -f "${LOGO}" "${dir}/zyvor-logo.png"
  echo "  ${dir}/zyvor-logo.png"
done

echo "Done. Import ZyvorFooter from components/ZyvorBrand.tsx in each app layout."
