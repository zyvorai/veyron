#!/usr/bin/env bash
# Upload the customer linux/amd64 tarball from dist/ to an existing GitHub release.
#
# Usage:
#   ./scripts/publish-customer-release.sh [tag]   # default: v$(Cargo.toml version)
#
# Prerequisites:
#   - gh CLI authenticated (gh auth login)
#   - dist/veyron-<ver>-linux-amd64.tar.gz (+ .sha256) from package-binary-remote.sh --fetch
#
# Example:
#   ./scripts/package-binary-remote.sh 212.8.252.194 sus --fetch
#   ./scripts/publish-customer-release.sh v0.2.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_DIR}/Cargo.toml" | head -1)"
TAG="${1:-v${VERSION}}"
ARCHIVE="vmrogue-${VERSION}-linux-amd64.tar.gz"
CHECKSUM="${ARCHIVE}.sha256"
DIST="${REPO_DIR}/dist"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI required (https://cli.github.com/)" >&2
  exit 1
fi

[[ -f "${DIST}/${ARCHIVE}" ]] || {
  echo "Missing ${DIST}/${ARCHIVE}" >&2
  echo "Run: ./scripts/package-binary-remote.sh <host> <user> --fetch" >&2
  exit 1
}

if [[ ! -f "${DIST}/${CHECKSUM}" ]]; then
  (cd "${DIST}" && shasum -a 256 "${ARCHIVE}" > "${CHECKSUM}")
fi

echo "==> Verifying checksum"
(cd "${DIST}" && shasum -a 256 -c "${CHECKSUM}")

if ! gh release view "${TAG}" >/dev/null 2>&1; then
  echo "==> Creating release ${TAG}"
  gh release create "${TAG}" \
    --title "VMRogue ${VERSION}" \
    --notes "Customer bundle and platform release ${TAG}."
fi

echo "==> Uploading customer bundle to ${TAG}"
gh release upload "${TAG}" \
  "${DIST}/${ARCHIVE}" \
  "${DIST}/${CHECKSUM}" \
  --clobber

URL=$(gh release view "${TAG}" --json url -q .url)
ASSET_URL="${URL/tag\//download/}/${ARCHIVE}"
echo ""
echo "Release: ${URL}"
echo "Customer tarball:"
echo "  ${ASSET_URL}"
echo "Checksum:"
echo "  ${ASSET_URL}.sha256"
