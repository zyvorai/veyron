#!/usr/bin/env bash
# Convenience wrapper for deploy-all-remote.sh (same arguments).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${SCRIPT_DIR}/deploy-all-remote.sh" "$@"
