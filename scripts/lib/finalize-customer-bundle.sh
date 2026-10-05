#!/usr/bin/env bash
# Finalize customer tarball: license files, markdown docs, path verification.
# Usage: finalize-customer-bundle.sh <stage> <build-dir> <product> [version]
set -euo pipefail

STAGE="${1:?stage directory}"
BUILD_DIR="${2:?build directory}"
PRODUCT="${3:?product name}"
LIB="${BUILD_DIR}/scripts/lib"

for f in LICENSE NOTICE README.md; do
  [[ -f "${BUILD_DIR}/${f}" ]] || { echo "ERROR: missing ${BUILD_DIR}/${f}" >&2; exit 1; }
  cp "${BUILD_DIR}/${f}" "${STAGE}/${f}"
done

mkdir -p "${STAGE}/docs"
cp "${BUILD_DIR}"/docs/*.md "${STAGE}/docs/"

[[ -x "${LIB}/verify-bundle-script-paths.sh" ]] || { echo "ERROR: missing ${LIB}/verify-bundle-script-paths.sh" >&2; exit 1; }
"${LIB}/verify-bundle-script-paths.sh" "${STAGE}"

test -f "${STAGE}/docs/getting-started.md" || { echo "ERROR: ${PRODUCT} bundle missing docs/getting-started.md" >&2; exit 1; }
