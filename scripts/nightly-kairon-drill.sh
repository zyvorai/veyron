#!/usr/bin/env bash
# Nightly integration drill for the Kairon/FluxVM routes: runs the VM daily-ops E2E against a
# test host and exits non-zero on any failure. Schedule it from cron or CI, e.g.
#   17 2 * * *  VEYRON_API_KEY=... /path/to/scripts/nightly-kairon-drill.sh 175.110.122.71
# Set VEYRON_DRILL_WEBHOOK to a URL to receive the summary line as {"text": "..."} on failure.
set -uo pipefail
HOST="${1:?usage: $0 HOST [NODEPORT]}"
PORT="${2:-30151}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG="$(mktemp -t kairon-drill.XXXXXX)"

"${DIR}/test-vm-daily-ops-remote.sh" "${HOST}" "${PORT}" >"${LOG}" 2>&1
rc=$?
summary="$(sed 's/\x1b\[[0-9;]*m//g' "${LOG}" | grep -E 'Results:' | tail -1)"
failed="$(sed 's/\x1b\[[0-9;]*m//g' "${LOG}" | grep -E '^\s*✗' | head -10)"

echo "[drill] ${HOST}: ${summary:-no result line (exit ${rc})}"
if [[ ${rc} -ne 0 ]]; then
  [[ -n "${failed}" ]] && echo "${failed}"
  echo "[drill] full log: ${LOG}"
  if [[ -n "${VEYRON_DRILL_WEBHOOK:-}" ]]; then
    msg="Kairon drill FAILED on ${HOST}: ${summary:-exit ${rc}}"
    curl -fsS -X POST -H 'Content-Type: application/json' \
      -d "$(printf '{"text":"%s"}' "${msg//\"/\\\"}")" "${VEYRON_DRILL_WEBHOOK}" >/dev/null || true
  fi
  exit "${rc}"
fi
rm -f "${LOG}"
