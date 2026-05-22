#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"
if [[ -f "${ROOT}/.package-lib/package-uninstall-lib.sh" ]]; then
  # shellcheck source=/dev/null
  source "${ROOT}/.package-lib/package-uninstall-lib.sh"
else
  # shellcheck source=package-uninstall-lib.sh
  source "$(dirname "$0")/package-uninstall-lib.sh"
fi

PRODUCT="VMRogue"
BINARIES=(vmrogue)
PORTS=(5151)
LOCAL_CONFIGS=(vmrogue.env)

package_uninstall_main "${PRODUCT}" "${ROOT}" "$@"
