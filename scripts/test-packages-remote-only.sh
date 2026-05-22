#!/usr/bin/env bash
# Run on remote host: install / test / uninstall for all *-dist tarballs.
# Usage: scp to remote && bash ~/test-packages-remote-only.sh
set -uo pipefail
TEST_ROOT="${HOME}/package-tests"
mkdir -p "${TEST_ROOT}"
RESULTS="${TEST_ROOT}/results-$(date +%Y%m%d-%H%M%S).log"
PASS=0 FAIL=0
log() { echo "$@" | tee -a "${RESULTS}"; }
pick_latest() { ls -t "$1"/*.tar.gz 2>/dev/null | head -1; }
has_uninstall() { tar tzf "$1" 2>/dev/null | awk -F/ '$NF=="uninstall.sh"{found=1} END{exit !found}'; }

test_tarball() {
  local name="$1" tarball="$2"
  log ""; log "======== ${name} ========"
  [[ -f "${tarball}" ]] || { log "FAIL: missing ${tarball}"; ((FAIL++)); return 1; }
  has_uninstall "${tarball}" || { log "FAIL: no uninstall.sh in ${tarball}"; ((FAIL++)); return 1; }
  tar tzf "${tarball}" 2>/dev/null | awk -F/ '$NF=="package-ui.sh"{u=1} END{exit !u}' || \
    { log "WARN: package-ui.sh not in bundle (old tarball?)"; }
  local work="${TEST_ROOT}/${name}-$$"
  rm -rf "${work}" && mkdir -p "${work}"
  tar xzf "${tarball}" -C "${work}" || { log "FAIL: extract"; ((FAIL++)); return 1; }
  local dir; dir=$(find "${work}" -maxdepth 1 -mindepth 1 -type d | head -1)
  cd "${dir}" || return 1
  log "  dir: ${dir}"
  if ! timeout 180 env ZYVOR_NONINTERACTIVE=1 ./install.sh </dev/null; then
    log "FAIL: install.sh (timeout 180s)"
    rm -rf "${work}"; ((FAIL++)); return 1
  fi
  log "  OK: install.sh"
  ./test-package.sh && log "  OK: test-package.sh" || log "  WARN: test-package.sh"
  [[ -x ./test-cluster.sh ]] && command -v kubectl >/dev/null && \
    [[ -n "${KUBECONFIG:-}" ]] && [[ -f "${KUBECONFIG}" ]] && \
    ./test-cluster.sh && log "  OK: test-cluster.sh" || true
  [[ -x ./test-host.sh ]] && ./test-host.sh && log "  OK: test-host.sh" || true
  ./uninstall.sh --yes --remove-dir || { log "FAIL: uninstall"; rm -rf "${work}"; ((FAIL++)); return 1; }
  sleep 3
  [[ -d "${dir}" ]] && { log "FAIL: dir still exists"; rm -rf "${work}"; ((FAIL++)); return 1; }
  log "PASS: ${name}"; ((PASS++)); return 0
}

declare -a JOBS=(
  "VMRogue|$(pick_latest "${HOME}/vmrogue-dist")"
  "machina|$(pick_latest "${HOME}/machina-dist")"
  "v9s|$(pick_latest "${HOME}/v9s-dist")"
  "guestkit|$(pick_latest "${HOME}/guestkit-dist")"
  "hypersdk|$(pick_latest "${HOME}/hypersdk-dist")"
  "hyper2kvm|$(pick_latest "${HOME}/hyper2kvm-dist")"
  "packetwolf|$(pick_latest "${HOME}/packetwolf-dist")"
  "ragnarok|$(pick_latest "${HOME}/ragnarok-dist")"
  "Aether|$(pick_latest "${HOME}/aether-dist")"
  "IronWolf|$(pick_latest "${HOME}/ironwolf-dist")"
  "forge|$(pick_latest "${HOME}/forge-dist")"
)
log "=== Remote install/uninstall tests ==="
for job in "${JOBS[@]}"; do test_tarball "${job%%|*}" "${job#*|}" || true; done
log ""; log "SUMMARY: ${PASS} passed, ${FAIL} failed"; log "Log: ${RESULTS}"
exit $((FAIL > 0))
