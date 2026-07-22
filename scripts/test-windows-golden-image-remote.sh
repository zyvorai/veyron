#!/usr/bin/env bash
# Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
# Proprietary software — see LICENSE in the repository root.
#
# End-to-end Windows golden-image workflow, driven through the Veyron API:
#   ISO import -> unattended install + sysprep seal -> publish -> clone a tenant VM.
#
# This exercises the real product paths added for golden images:
#   POST /images/import        (CDI DataVolume from a URL)
#   sysprep CD-ROM emission    (autounattend.xml as install media)
#   POST /images/publish       (Succeeded DV -> versioned golden PVC + DataSource)
#   POST /vms {image:...}       (per-VM dataVolumeTemplate clone from the DataSource)
#
# Usage:
#   VEYRON_API_KEY=... WIN_ISO_URL='https://...' \
#     ./scripts/test-windows-golden-image-remote.sh <host> <ssh-user> [https_node_port]
#
# Long-running (Windows install is 30-60+ min). Each phase polls to completion.
# The autounattend.xml lives alongside this script under examples/.
set -uo pipefail

HOST="${1:?usage: $0 <host> <ssh-user> [port]}"
SSH_USER="${2:?ssh user required}"
PORT="${3:-30151}"
KEY="${VEYRON_API_KEY:?set VEYRON_API_KEY}"
ISO_URL="${WIN_ISO_URL:?set WIN_ISO_URL to a Windows ISO download URL}"
IMG_NS="${VEYRON_IMAGE_NS:-vm-images}"
TENANT_NS="${VEYRON_TENANT_NS:-customer-a}"
SC="${VEYRON_STORAGE_CLASS:-zyvor-rbd-prod}"
BASE="https://${HOST}:${PORT}"
SSH="ssh -o StrictHostKeyChecking=no ${SSH_USER}@${HOST}"
K="sudo k3s kubectl"

G="\033[0;32m"; Y="\033[0;33m"; R="\033[0;31m"; N="\033[0m"
ok(){ printf "${G}✔${N} %s\n" "$*"; }
step(){ printf "\n${Y}▶ %s${N}\n" "$*"; }
die(){ printf "${R}✘ %s${N}\n" "$*"; exit 1; }
api(){ curl -sk -m 120 -H "X-API-Key: ${KEY}" "$@"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNATTEND="${SCRIPT_DIR}/../examples/windows-golden-image/autounattend-builder.xml"

# ── 1. Preconditions ────────────────────────────────────────────────────────
step "Preconditions"
caps="$(api "${BASE}/api/v1/platform/capabilities")"
echo "${caps}" | grep -q '"windows_golden_images":true' \
  || die "cluster reports windows_golden_images=false — run scripts/cluster/adapt-existing-cluster.sh"
ok "windows_golden_images capability present"
$SSH "$K get ns ${IMG_NS} >/dev/null 2>&1 || $K create ns ${IMG_NS}" >/dev/null 2>&1
$SSH "$K get ns ${TENANT_NS} >/dev/null 2>&1 || $K create ns ${TENANT_NS}" >/dev/null 2>&1
ok "namespaces ${IMG_NS}, ${TENANT_NS}"
# Cross-namespace clone RBAC: the TENANT's default SA (the clone's owner) needs
# datavolumes/source create in the image namespace, or the clone hangs Unauthorized.
$SSH "$K apply -f - >/dev/null 2>&1" <<EOF
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata: { name: cdi-golden-image-cloner }
rules:
  - apiGroups: ["cdi.kubevirt.io"]
    resources: ["datavolumes/source"]
    verbs: ["create"]
  - apiGroups: ["cdi.kubevirt.io"]
    resources: ["datasources"]
    verbs: ["get","list","watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata: { name: ${TENANT_NS}-golden-image-cloner, namespace: ${IMG_NS} }
subjects: [{ kind: ServiceAccount, name: default, namespace: ${TENANT_NS} }]
roleRef: { apiGroup: rbac.authorization.k8s.io, kind: ClusterRole, name: cdi-golden-image-cloner }
EOF
ok "cross-namespace clone RBAC (${TENANT_NS}:default → ${IMG_NS})"

# ── 2. Import the ISO through the API ────────────────────────────────────────
step "Import Windows ISO via POST /images/import"
import_body="$(IMG_NS="${IMG_NS}" ISO_URL="${ISO_URL}" SC="${SC}" python3 -c \
  "import json,os;print(json.dumps({'name':'win-iso','namespace':os.environ['IMG_NS'],'url':os.environ['ISO_URL'],'size':'8Gi','storage_class':os.environ['SC']}))")"
code="$(api -o /dev/null -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d "${import_body}" "${BASE}/api/v1/images/import")"
[ "${code}" = "201" ] || die "import returned HTTP ${code}"
ok "DataVolume win-iso created"

wait_dv(){ # wait_dv <ns> <name> <timeout-min>
  local ns="$1" name="$2" max="$3" i=0
  while [ $i -lt $((max*2)) ]; do
    local p; p="$($SSH "$K -n ${ns} get dv ${name} -o jsonpath={.status.phase}" 2>/dev/null)"
    local pr; pr="$($SSH "$K -n ${ns} get dv ${name} -o jsonpath={.status.progress}" 2>/dev/null)"
    printf "\r  %s: %s %s        " "${name}" "${p}" "${pr}"
    case "${p}" in Succeeded) echo; return 0;; Failed) echo; return 1;; esac
    sleep 30; i=$((i+1))
  done; echo; return 1
}
wait_dv "${IMG_NS}" win-iso 60 || die "ISO import did not succeed"
ok "ISO imported"

# ── 3. Build + seal (unattended) ─────────────────────────────────────────────
step "Unattended install + sysprep seal"
[ -f "${UNATTEND}" ] || die "answer file not found: ${UNATTEND}"
$SSH "$K -n ${IMG_NS} create secret generic win-builder-sysprep \
  --from-file=autounattend.xml=/dev/stdin --dry-run=client -o yaml | $K apply -f -" \
  < "${UNATTEND}" >/dev/null 2>&1
ok "sysprep Secret created (autounattend.xml)"
# The builder manifest is applied from the node (raw KubeVirt VM with ISO+virtio+sysprep).
# It self-seals: FirstLogonCommands installs drivers then runs sysprep /shutdown.
# runStrategy RerunOnFailure keeps KubeVirt from restarting the sealed image.
scp -q -o StrictHostKeyChecking=no "${SCRIPT_DIR}/../examples/windows-golden-image/win-builder.yaml" "${SSH_USER}@${HOST}:/tmp/win-builder.yaml"
$SSH "$K apply -f /tmp/win-builder.yaml" >/dev/null 2>&1
ok "builder VM created — installing (this is the long part)"
printf "  waiting for the builder to sysprep and power off"
i=0
while [ $i -lt 180 ]; do   # up to 90 min
  vmi="$($SSH "$K -n ${IMG_NS} get vmi win-builder -o jsonpath={.status.phase} 2>/dev/null" 2>/dev/null)"
  # Sealed = VMI gone (sysprep /shutdown) while the VM object remains.
  if [ -z "${vmi}" ] && $SSH "$K -n ${IMG_NS} get vm win-builder >/dev/null 2>&1"; then
    echo; ok "builder sealed and powered off"; break
  fi
  printf "."; sleep 30; i=$((i+1))
done
[ $i -lt 180 ] || die "builder never sealed — VNC in and check Setup"
# The VMI disappearing only means the guest shut down — CDI can still be
# finalizing the underlying win-builder-root DataVolume for a beat after
# that. Publish immediately rejects anything not yet 'Succeeded' (by
# design — see the publish handler's incomplete-image guard), so without
# this wait the very first publish attempt races it and always 400s.
wait_dv "${IMG_NS}" win-builder-root 10 || die "win-builder-root DataVolume did not reach Succeeded after builder sealed"
ok "win-builder-root DataVolume Succeeded"

# ── 4. Publish ───────────────────────────────────────────────────────────────
step "Publish golden image (Admin) via POST /images/publish"
pub="$(IMG_NS="${IMG_NS}" python3 -c \
  "import json,os;print(json.dumps({'name':'win-builder-root','namespace':os.environ['IMG_NS'],'data_source':'windows-golden','version':os.environ.get('VER','e2e')}))")"
code="$(api -o /tmp/pub.json -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d "${pub}" "${BASE}/api/v1/images/publish")"
[ "${code}" = "200" ] || die "publish returned HTTP ${code}: $(cat /tmp/pub.json)"
ok "DataSource windows-golden published"
# windows-golden lives in IMG_NS, not the API's default namespace — an
# unscoped query silently returns an empty list (namespace-scoped by
# default, not all-namespaces), which looks identical to "not published".
api "${BASE}/api/v1/images/datasources?namespace=${IMG_NS}" | grep -q '"windows-golden"' \
  && ok "DataSource visible in catalog" || die "DataSource not in catalog"

# ── 5. Clone a tenant VM from the image ─────────────────────────────────────
# disk_size must be >= win-builder.yaml's dataVolumeTemplates storage request
# (70Gi) — CDI's clone validator rejects a target smaller than its source
# ("target resources requests storage size is smaller than the source").
step "Create tenant VM from the image via POST /vms {image:...}"
create="$(TENANT_NS="${TENANT_NS}" IMG_NS="${IMG_NS}" python3 -c \
  "import json,os;print(json.dumps({'name':'win01','namespace':os.environ['TENANT_NS'],'template':'windows-2022','disk_size':'70Gi','image':{'name':'windows-golden','namespace':os.environ['IMG_NS']},'start':True}))")"
code="$(api -o /tmp/create.json -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d "${create}" "${BASE}/api/v1/vms")"
echo "${code}" | grep -qE '^20' || die "create VM returned HTTP ${code}: $(cat /tmp/create.json)"
ok "win01 created"
# It must clone its OWN disk (a dataVolumeTemplate), not reference the golden PVC.
$SSH "$K -n ${TENANT_NS} get vm win01 -o jsonpath='{.spec.dataVolumeTemplates[0].metadata.name}'" 2>/dev/null \
  | grep -q win01 && ok "win01 owns a cloned DataVolume (not sharing the golden disk)" \
  || die "win01 has no dataVolumeTemplate — it would share the golden disk"
wait_dv "${TENANT_NS}" win01-rootdisk 30 || die "clone did not bind"
ok "clone bound — win01 booting from its own copy of the golden image"

printf "\n${G}════ Windows golden-image E2E PASSED ════${N}\n"
echo "  win01 in ${TENANT_NS} is a sysprepped clone of windows-golden."
echo "  VNC:  virtctl vnc win01 -n ${TENANT_NS}   (or the dashboard console)"
