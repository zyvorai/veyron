#!/usr/bin/env bash
# From your laptop: sync test script, optionally rebuild one product, run remote E2E.
#
# Usage:
#   ./scripts/test-customer-e2e-remote.sh HOST USER
#   ./scripts/test-customer-e2e-remote.sh HOST USER --quick          # skip rebuild
#   ./scripts/test-customer-e2e-remote.sh HOST USER --product machina # rebuild one tarball
#
# Examples:
#   ./scripts/test-customer-e2e-remote.sh HOST sus
#   ZYVOR_E2E_SKIP=VMRogue,v9s ./scripts/test-customer-e2e-remote.sh HOST sus --quick
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi

HOST="${1:?usage: $0 HOST USER [--quick] [--product NAME]}"
USER="${2:?usage: $0 HOST USER [--quick] [--product NAME]}"
shift 2 || true

QUICK=0
PRODUCT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick) QUICK=1 ;;
    --product) PRODUCT="${2:?}"; shift ;;
    -h|--help)
      sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
  shift
done

REMOTE="${USER}@${HOST}"
SSH_OPTS=(-o ConnectTimeout=15 -o BatchMode=yes -o StrictHostKeyChecking=accept-new)

echo "==> Remote customer E2E → ${REMOTE}"
echo ""

scp "${SSH_OPTS[@]}" "${ROOT}/scripts/test-packages-remote-only.sh" "${REMOTE}:~/test-packages-remote-only.sh"

if [[ "${QUICK}" -eq 0 ]]; then
  if [[ -n "${PRODUCT}" ]]; then
    case "${PRODUCT,,}" in
      vmrogue) "${ROOT}/scripts/package-binary-remote.sh" "${HOST}" "${USER}" --fetch ;;
      machina) "${ROOT}/../machina/scripts/package-binary-remote.sh" "${HOST}" "${USER}" --fetch ;;
      v9s) "${ROOT}/../v9s/scripts/package-binary-remote.sh" "${HOST}" "${USER}" --fetch ;;
      *)
        echo "Rebuild for --product ${PRODUCT}: run that repo's scripts/package-binary-remote.sh ${HOST} ${USER}"
        exit 1
        ;;
    esac
  else
    echo "==> Quick path: using existing tarballs on remote (pass --product machina to rebuild one)"
  fi
fi

echo ""
echo "==> Running remote install/uninstall tests (install-everything when bundled)..."
ssh "${SSH_OPTS[@]}" "${REMOTE}" 'chmod +x ~/test-packages-remote-only.sh && bash ~/test-packages-remote-only.sh'
echo ""
echo "Done. Fetch log: scp ${REMOTE}:~/package-tests/results-*.log ."
