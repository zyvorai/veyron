#!/usr/bin/env bash
# Generate a KubeVirt VM from a Windows template and add cloudInitConfigDrive.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-${ROOT}/generated/vm.yaml}"
VM_NAME="${2:-win11-vm}"
GOLDEN_DV="${3:-win11-golden-dv}"
TEMPLATE="${4:-windows-11}"

mkdir -p "$(dirname "$OUT")"
vmrogue generate "${VM_NAME}" --template "${TEMPLATE}" --kubevirt --memory 8Gi -o "${OUT}"
python3 "$(dirname "$0")/patch_kubevirt_configdrive.py" "${OUT}" "${VM_NAME}" "${GOLDEN_DV}"
echo "Wrote ${OUT}"
