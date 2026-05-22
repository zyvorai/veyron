# shellcheck shell=bash
# Source from package-binary-remote.sh REMOTE_PACK blocks.
# Usage: package_bundle_client_scripts STAGE BUILD_DIR
package_bundle_client_scripts() {
    local stage="$1" build_dir="$2"
    local lib="${build_dir}/scripts/lib"
    for src in package-install.sh package-client-install.sh package-client-test.sh; do
        if [ ! -f "${lib}/${src}" ]; then
            echo "ERROR: missing ${lib}/${src} — cannot build customer tarball" >&2
            return 1
        fi
    done
    cp "${lib}/package-install.sh" "${stage}/install.sh"
    cp "${lib}/package-client-install.sh" "${stage}/install-client-deps.sh"
    cp "${lib}/package-client-test.sh" "${stage}/test-package.sh"
    chmod +x "${stage}/install.sh" "${stage}/install-client-deps.sh" "${stage}/test-package.sh"
}
