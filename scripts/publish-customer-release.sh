#!/usr/bin/env bash
# Upload the customer linux/amd64 tarball from dist/ to a GitHub release.
#
# Publishes to the PUBLIC hypersdk/veyron-releases repo by default — customers
# can't download release assets from the private ssahani/Veyron source repo,
# so publishing there (the old default) produced links nobody outside the org
# could actually use. Override with --repo or PUBLISH_REPO if you really mean
# to publish somewhere else.
#
# Usage:
#   ./scripts/publish-customer-release.sh [tag] [--trial] [--repo owner/name]
#     tag           default: v$(Cargo.toml version), or v$(Cargo.toml version)-trial with --trial
#     --trial       publish the trial tarball (veyron-<ver>-trial-linux-amd64.tar.gz)
#                   instead of the regular one
#     --repo        override the target repo (default: hypersdk/veyron-releases,
#                   or $PUBLISH_REPO if set)
#
# Prerequisites:
#   - gh CLI authenticated with access to the target repo (gh auth login / gh auth switch)
#   - dist/veyron-<ver>[-trial]-linux-amd64.tar.gz (+ .sha256) from
#     package-binary-remote.sh --fetch [--trial]
#
# Examples:
#   ./scripts/package-binary-remote.sh <host> <user> --fetch
#   ./scripts/publish-customer-release.sh v0.2.0
#
#   ./scripts/package-binary-remote.sh <host> <user> --trial --fetch
#   ./scripts/publish-customer-release.sh --trial
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_DIR}/Cargo.toml" | head -1)"
DIST="${REPO_DIR}/dist"

TRIAL=false
TAG=""
TARGET_REPO="${PUBLISH_REPO:-hypersdk/veyron-releases}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --trial) TRIAL=true; shift ;;
    --repo=*) TARGET_REPO="${1#--repo=}"; shift ;;
    --repo)
      [[ $# -ge 2 ]] || { echo "--repo requires a value (owner/name)" >&2; exit 1; }
      TARGET_REPO="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      [[ -z "${TAG}" ]] || { echo "Unexpected extra argument: $1" >&2; exit 1; }
      TAG="$1"; shift
      ;;
  esac
done

if $TRIAL; then
  ARCHIVE="veyron-${VERSION}-trial-linux-amd64.tar.gz"
  TAG="${TAG:-v${VERSION}-trial}"
  TITLE="Veyron ${VERSION} — 30-Day Evaluation"
else
  ARCHIVE="veyron-${VERSION}-linux-amd64.tar.gz"
  TAG="${TAG:-v${VERSION}-veyron}"
  TITLE="Veyron ${VERSION}"
fi
CHECKSUM="${ARCHIVE}.sha256"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI required (https://cli.github.com/)" >&2
  exit 1
fi

[[ -f "${DIST}/${ARCHIVE}" ]] || {
  echo "Missing ${DIST}/${ARCHIVE}" >&2
  if $TRIAL; then
    echo "Run: ./scripts/package-binary-remote.sh <host> <user> --trial --fetch" >&2
  else
    echo "Run: ./scripts/package-binary-remote.sh <host> <user> --fetch" >&2
  fi
  exit 1
}

if [[ ! -f "${DIST}/${CHECKSUM}" ]]; then
  (cd "${DIST}" && shasum -a 256 "${ARCHIVE}" > "${CHECKSUM}")
fi

echo "==> Verifying checksum"
(cd "${DIST}" && shasum -a 256 -c "${CHECKSUM}")

echo "==> Target repo: ${TARGET_REPO}"
if ! gh release view "${TAG}" --repo "${TARGET_REPO}" >/dev/null 2>&1; then
  echo "==> Creating release ${TAG}"
  gh release create "${TAG}" \
    --repo "${TARGET_REPO}" \
    --title "${TITLE}" \
    --notes "Customer bundle and platform release ${TAG}."
fi

echo "==> Uploading customer bundle to ${TAG}"
gh release upload "${TAG}" \
  --repo "${TARGET_REPO}" \
  "${DIST}/${ARCHIVE}" \
  "${DIST}/${CHECKSUM}" \
  --clobber

URL=$(gh release view "${TAG}" --repo "${TARGET_REPO}" --json url -q .url)
ASSET_URL="${URL/tag\//download/}/${ARCHIVE}"
echo ""
echo "Release: ${URL}"
echo "Customer tarball:"
echo "  ${ASSET_URL}"
echo "Checksum:"
echo "  ${ASSET_URL}.sha256"
