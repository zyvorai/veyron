#!/usr/bin/env bash
# Export VMRogue built-in templates and profiles to operator/config/catalog CRD YAML.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/operator/config/catalog}"
cd "$ROOT"
export RUST_MIN_STACK=8388608
cargo run -- catalog export --output "$OUT"
echo "Catalog manifests written to $OUT/templates and $OUT/profiles"
echo "Apply CRDs first: kubectl apply -f operator/config/crd/bases/vmrogue.io_vmtemplates.yaml"
echo "                 kubectl apply -f operator/config/crd/bases/vmrogue.io_vmprofiles.yaml"
echo "Then sync: vmrogue catalog sync"
