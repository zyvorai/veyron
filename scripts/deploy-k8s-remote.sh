#!/usr/bin/env bash
# ============================================================================
# deploy-k8s-remote.sh — Deploy VMRogue to remote K8s cluster
# ============================================================================
# Rsync source, build binary, build container image, deploy to K8s.
# No systemd — pure Kubernetes deployment.
# The API is served with HTTPS (self-signed cert via initContainer openssl).
# To use your own cert: create a TLS Secret in vmrogue-system and patch the
# Deployment to mount it at /certs (tls.crt, tls.key) instead of the generated pair.
#
# Usage:
#   ./scripts/deploy-k8s-remote.sh <host> [user]
#   ./scripts/deploy-k8s-remote.sh --help
#
# Optional environment (local):
#   VMROGUE_SKIP_CDI=1       — do not install CDI when the DataVolume CRD is missing
#   VMROGUE_CDI_VERSION=v1.65.0 — CDI release tag (default below); must match KubeVirt/CDI compatibility on your cluster
#   VMROGUE_CONTAINER_RUNTIME_IMPORT — full shell command that reads OCI/docker tar on stdin (default: k3s ctr import, or ctr -n k8s.io for plain kubectl)
#   VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1 — do not apply deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml when Cilium is installed
#   VMROGUE_REQUIRE_KUBEVIRT=1 — fail deploy if KubeVirt VM CRD is missing (after rsync; remote kubectl check)
#   VMROGUE_REMOTE_SKIP_SSH_CHECK=1 — skip SSH BatchMode preflight before rsync
#   DEPLOY_SSH_TIMEOUT=20 — SSH ConnectTimeout seconds (preflight only)
#   NO_COLOR=1 — disable ANSI highlights
#
# RBAC applied with the manifest matches deploy/k8s.yaml ClusterRole (KubeVirt, CDI namespaces
# via pod list, Cilium policy list, Metrics Server PodMetrics, CSIStorageCapacity, MigrationPolicy).
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    cat <<'USAGE_EOF'
deploy-k8s-remote.sh — Deploy VMRogue API to a remote Kubernetes node (rsync → build → image → CDI → apply).

Usage:
  ./scripts/deploy-k8s-remote.sh <host> [user]
  ./scripts/deploy-k8s-remote.sh --help

Environment (local):
  VMROGUE_API_KEY, VMROGUE_NODE_PORT, VMROGUE_SKIP_CDI, VMROGUE_CDI_VERSION
  VMROGUE_CONTAINER_RUNTIME_IMPORT
  VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP=1  Skip Cilium egress bootstrap when Cilium CRD exists
  VMROGUE_REQUIRE_KUBEVIRT=1              Fail if KubeVirt VM CRD is missing
  VMROGUE_REMOTE_SKIP_SSH_CHECK=1         Skip SSH preflight
  DEPLOY_SSH_TIMEOUT                        SSH connect timeout (default 20)
  NO_COLOR=1                               Disable ANSI colors
USAGE_EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

HOST="${1:?Usage: $0 <host> [user]   (see --help)}"
USER="${2:-root}"
REMOTE_DIR="/home/${USER}/vmrogue"
API_KEY="${VMROGUE_API_KEY:-CHANGE_ME}"
NODE_PORT="${VMROGUE_NODE_PORT:-30151}"
NS="vmrogue-system"
CDI_VERSION="${VMROGUE_CDI_VERSION:-v1.65.0}"
SKIP_CDI="${VMROGUE_SKIP_CDI:-0}"
SSH_TIMEOUT="${DEPLOY_SSH_TIMEOUT:-20}"
RUN_STARTED_AT="$(date +%s)"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    COLOR_BOLD=$'\033[1m'
    COLOR_DIM=$'\033[2m'
    COLOR_GREEN=$'\033[32m'
    COLOR_CYAN=$'\033[36m'
    COLOR_RED=$'\033[31m'
    COLOR_RESET=$'\033[0m'
else
    COLOR_BOLD=''
    COLOR_DIM=''
    COLOR_GREEN=''
    COLOR_CYAN=''
    COLOR_RED=''
    COLOR_RESET=''
fi

format_duration() {
    local total="$1"
    local mins=$((total / 60))
    local secs=$((total % 60))
    printf "%dm %02ds" "${mins}" "${secs}"
}

info() { echo "  ${COLOR_GREEN}[✓]${COLOR_RESET} $*"; }
step() { echo ""; echo "  ${COLOR_CYAN}──${COLOR_RESET} ${COLOR_BOLD}$*${COLOR_RESET}"; }
error() { echo "  ${COLOR_RED}[✗]${COLOR_RESET} $*"; exit 1; }

REMOTE="${USER}@${HOST}"

if [[ "${VMROGUE_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    step "Preflight: SSH (${REMOTE})"
    if ! ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=accept-new \
        "${REMOTE}" "true" 2>/dev/null; then
        error "SSH preflight failed (try: ssh ${REMOTE} or VMROGUE_REMOTE_SKIP_SSH_CHECK=1 $0 $*)"
    fi
    info "SSH OK (timeout ${SSH_TIMEOUT}s)"
fi

# Remote cluster: k3s (bundled kubectl + ctr) vs generic Kubernetes (kubectl + containerd ctr).
REMOTE_K8S_FLAVOR=$(ssh "${USER}@${HOST}" '
  if [ -x /usr/local/bin/k3s ]; then echo k3s_std
  elif command -v k3s >/dev/null 2>&1; then echo k3s_path
  elif command -v kubectl >/dev/null 2>&1; then echo kubectl
  else echo none
  fi' | tr -d '\r')
case "${REMOTE_K8S_FLAVOR}" in
  k3s_std)
    K="sudo /usr/local/bin/k3s kubectl"
    IMPORT_DEFAULT="sudo /usr/local/bin/k3s ctr images import -"
    ;;
  k3s_path)
    K="sudo k3s kubectl"
    IMPORT_DEFAULT="sudo k3s ctr images import -"
    ;;
  kubectl)
    K="kubectl"
    IMPORT_DEFAULT="sudo ctr -n k8s.io images import -"
    ;;
  *)
    error "Neither k3s nor kubectl found on ${USER}@${HOST} (install a cluster CLI or use a registry-based image flow)"
    ;;
esac
IMPORT_CMD="${VMROGUE_CONTAINER_RUNTIME_IMPORT:-${IMPORT_DEFAULT}}"

echo ""
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo "  ${COLOR_BOLD}  VMRogue → Kubernetes (remote API)${COLOR_RESET}"
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo ""
echo "  ${COLOR_DIM}Target:${COLOR_RESET}     ${REMOTE}"
echo "  ${COLOR_DIM}Flavor:${COLOR_RESET}     ${REMOTE_K8S_FLAVOR}  (${K})"
echo "  ${COLOR_DIM}Import:${COLOR_RESET}     ${IMPORT_CMD}"
echo "  ${COLOR_DIM}Namespace:${COLOR_RESET}  ${NS}"
echo "  ${COLOR_DIM}NodePort:${COLOR_RESET}   ${NODE_PORT} → pod :5151 (TLS)"
echo "  ${COLOR_DIM}API key:${COLOR_RESET}    ${API_KEY}"
echo ""

# ── Step 0: Embedded React operator UI ──
step "Step 0/7: Building embedded dashboard-next"
"${SCRIPT_DIR}/build-dashboard-next.sh"
info "dashboard-next staged for rust-embed"

# ── Step 1: Rsync ──
step "Step 1/7: Syncing source to ${HOST}"
rsync -az --delete \
    --exclude target/ --exclude .git/ --exclude operator/bin/ \
    . "${USER}@${HOST}:${REMOTE_DIR}/"
info "Source synced"

# ── Step 2: Build binary ──
step "Step 2/7: Building release binary"
ssh "${USER}@${HOST}" "
    source \$HOME/.cargo/env 2>/dev/null || true
    cd ${REMOTE_DIR}
    cargo build --release 2>&1 | tail -3
    strip target/release/vmrogue 2>/dev/null || true
    ls -lh target/release/vmrogue
"
info "Binary built"

# ── Step 3: Build container image ──
step "Step 3/7: Building container image"
ssh "${USER}@${HOST}" "
    cd ${REMOTE_DIR}
    cp target/release/vmrogue /tmp/vmrogue-binary
    podman build --format docker -t localhost/vmrogue:latest -f Dockerfile.deploy /tmp 2>&1 | tail -3
    podman save localhost/vmrogue:latest | ${IMPORT_CMD} 2>&1
" || error "Image import failed (non-k3s: install containerd ctr, set VMROGUE_CONTAINER_RUNTIME_IMPORT, or push to a registry and adjust the Deployment image / pullPolicy)"
info "Container image built and imported"

# ── Step 4: CDI (DataVolume / import) when missing ──
# KubeVirt disk pipelines and examples often need CDI; installing it here avoids a separate manual step on fresh clusters.
step "Step 4/7: Ensuring CDI (containerized-data-importer)"
if [[ "${SKIP_CDI}" == "1" ]]; then
  info "Skipped CDI install (VMROGUE_SKIP_CDI=1)"
else
  "${SCRIPT_DIR}/ensure-cdi-remote.sh" "${USER}@${HOST}" "${CDI_VERSION}" ||
    error "CDI install or wait failed (set VMROGUE_SKIP_CDI=1 to skip, or fix cluster network / storage)"
  info "CDI available for DataVolume workflows"
fi

# ── Step 5: Deploy to K8s ──
# Bump pod template every run so apply triggers a rollout even when the image ref
# stays localhost/vmrogue:latest with imagePullPolicy: Never (otherwise old pods
# keep running the previous image layers).
DEPLOY_STAMP="$(date +%s)-${RANDOM}"

step "Step 5/7: Deploying to Kubernetes"
# shellcheck disable=SC2029
ssh "${USER}@${HOST}" "
    ${K} create namespace ${NS} 2>/dev/null || true

    if [[ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" && \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ]] && ${K} get crd ciliumnetworkpolicies.cilium.io &>/dev/null && [[ -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml ]]; then
      echo 'Applying Cilium egress bootstrap for ${NS} (API → apiserver / in-cluster)...'
      ${K} apply -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-vmrogue-egress.yaml
    fi
    if [[ \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" && \"${VMROGUE_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ]] && ${K} get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null && [[ -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml ]]; then
      echo 'Applying Cilium clusterwide egress for KubeVirt virt-launcher (VM guest internet)...'
      ${K} apply -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml
    fi
    if [[ \"${VMROGUE_REQUIRE_KUBEVIRT:-}\" == \"1\" || \"${VMROGUE_REQUIRE_KUBEVIRT:-}\" == \"true\" ]]; then
      ${K} get crd virtualmachines.kubevirt.io &>/dev/null || { echo 'KubeVirt CRD virtualmachines.kubevirt.io not found'; exit 1; }
    fi

    for crd_yaml in ${REMOTE_DIR}/operator/config/crd/bases/vmrogue.io_vmtemplates.yaml ${REMOTE_DIR}/operator/config/crd/bases/vmrogue.io_vmprofiles.yaml; do
      if [[ -f \"\${crd_yaml}\" ]]; then
        ${K} apply -f \"\${crd_yaml}\"
      fi
    done
    if ${K} get crd vmtemplates.vmrogue.io &>/dev/null; then
      if [[ -d ${REMOTE_DIR}/operator/config/catalog/templates ]]; then
        echo 'Applying VMTemplate catalog manifests...'
        for f in ${REMOTE_DIR}/operator/config/catalog/templates/*.yaml; do
          [[ -f \"\$f\" ]] && ${K} apply -f \"\$f\" || true
        done
      fi
      if [[ -d ${REMOTE_DIR}/operator/config/catalog/profiles ]]; then
        echo 'Applying VMProfile catalog manifests...'
        for f in ${REMOTE_DIR}/operator/config/catalog/profiles/*.yaml; do
          [[ -f \"\$f\" ]] && ${K} apply -f \"\$f\" || true
        done
      fi
    fi

    cat << 'YAML' | ${K} apply -f -
apiVersion: v1
kind: ServiceAccount
metadata:
  name: vmrogue
  namespace: ${NS}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: vmrogue
rules:
  - apiGroups: ['kubevirt.io']
    resources: ['virtualmachines', 'virtualmachineinstances']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['kubevirt.io']
    resources: ['virtualmachineinstancemigrations']
    verbs: ['get', 'list', 'watch', 'create', 'delete']
  - apiGroups: ['subresources.kubevirt.io']
    resources:
      - 'virtualmachineinstances/vnc'
      - 'virtualmachineinstances/console'
      - 'virtualmachines/start'
      - 'virtualmachines/stop'
      - 'virtualmachines/restart'
      - 'virtualmachines/migrate'
    verbs: ['get', 'update', 'patch']
  - apiGroups: ['subresources.kubevirt.io']
    resources: ['virtualmachineinstances/pause', 'virtualmachineinstances/unpause']
    verbs: ['update', 'patch']
  - apiGroups: ['subresources.kubevirt.io']
    resources: ['virtualmachineinstances/guest-exec']
    verbs: ['get', 'create', 'update']
  - apiGroups: ['subresources.kubevirt.io']
    resources: ['virtualmachineinstances/guest-exec-status']
    verbs: ['get']
  - apiGroups: ['snapshot.kubevirt.io']
    resources: ['virtualmachinesnapshots', 'virtualmachinesnapshotcontents', 'virtualmachinerestores']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['vmrogue.io']
    resources: ['vmroguevms', 'vmrogueblueprints', 'vmroguepolicies', 'vmrogueinsights', 'vmrogueactions', 'vmtemplates', 'vmprofiles']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['vmrogue.io']
    resources: ['vmroguevms/status', 'vmrogueblueprints/status', 'vmroguepolicies/status', 'vmrogueinsights/status', 'vmrogueactions/status', 'vmtemplates/status', 'vmprofiles/status']
    verbs: ['get', 'update', 'patch']
  - apiGroups: ['']
    resources: ['namespaces', 'nodes', 'pods', 'pods/log', 'events', 'persistentvolumeclaims', 'configmaps', 'resourcequotas']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['']
    resources: ['pods/exec', 'pods/attach', 'pods/portforward']
    verbs: ['create', 'get']
  - apiGroups: ['']
    resources: ['namespaces']
    verbs: ['create']
  - apiGroups: ['']
    resources: ['secrets', 'services']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['']
    resources: ['persistentvolumeclaims', 'configmaps']
    verbs: ['create', 'update', 'patch', 'delete']
  - apiGroups: ['coordination.k8s.io']
    resources: ['leases']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch']
  - apiGroups: ['storage.k8s.io']
    resources: ['storageclasses', 'csistoragecapacities', 'volumeattachments']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['snapshot.storage.k8s.io']
    resources: ['volumesnapshots', 'volumesnapshotcontents', 'volumesnapshotclasses']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['cdi.kubevirt.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['upload.cdi.kubevirt.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['instancetype.kubevirt.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['k8s.cni.cncf.io']
    resources: ['network-attachment-definitions']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['gateway.networking.k8s.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['chaos-mesh.org', 'litmuschaos.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['networking.k8s.io']
    resources: ['networkpolicies', 'ingresses']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['cilium.io']
    resources: ['*']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['rbac.authorization.k8s.io']
    resources: ['clusterroles', 'clusterrolebindings', 'roles', 'rolebindings']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['apps']
    resources: ['deployments', 'statefulsets', 'daemonsets']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['autoscaling']
    resources: ['horizontalpodautoscalers']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['apiextensions.k8s.io']
    resources: ['customresourcedefinitions']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['migrations.kubevirt.io']
    resources: ['migrationpolicies']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['metrics.k8s.io']
    resources: ['pods', 'nodes']
    verbs: ['get', 'list', 'watch']
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: vmrogue
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: vmrogue
subjects:
  - kind: ServiceAccount
    name: vmrogue
    namespace: ${NS}
---
apiVersion: v1
kind: Secret
metadata:
  name: vmrogue-api-key
  namespace: ${NS}
type: Opaque
stringData:
  api-key: "${API_KEY}"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: vmrogue-api
  namespace: ${NS}
spec:
  replicas: 1
  # One replica + default rolling params can schedule a second pod during rollout; on a
  # tight node the new pod stays Pending and rollout times out. Never surge: replace in place.
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 0
      maxUnavailable: 1
  selector:
    matchLabels:
      app: vmrogue-api
  template:
    metadata:
      labels:
        app: vmrogue-api
      annotations:
        vmrogue.io/deployed-at: "${DEPLOY_STAMP}"
    spec:
      serviceAccountName: vmrogue
      initContainers:
        - name: gen-cert
          image: alpine/openssl:3.3.2
          command: ["/bin/sh", "-c"]
          args:
            - |
              openssl req -x509 -nodes -days 3650 \
                -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
                -keyout /certs/tls.key -out /certs/tls.crt \
                -subj "/CN=vmrogue/O=vmrogue" \
                -addext "subjectAltName=DNS:vmrogue-api,DNS:vmrogue-api.${NS}.svc,DNS:localhost,IP:127.0.0.1"
              chown 10001:10001 /certs/tls.key /certs/tls.crt
              chmod 600 /certs/tls.key && chmod 644 /certs/tls.crt
          securityContext:
            runAsUser: 0
            runAsNonRoot: false
          volumeMounts:
            - name: tls-certs
              mountPath: /certs
      containers:
        - name: vmrogue
          image: localhost/vmrogue:latest
          imagePullPolicy: Never
          args: ['api-serve', '--port', '5151', '--host', '0.0.0.0', '--tls', '--tls-cert', '/certs/tls.crt', '--tls-key', '/certs/tls.key']
          env:
            - name: VMROGUE_API_KEY
              valueFrom:
                secretKeyRef:
                  name: vmrogue-api-key
                  key: api-key
            - name: RUST_LOG
              value: info
          ports:
            - containerPort: 5151
              name: https
              protocol: TCP
          livenessProbe:
            httpGet:
              path: /api/v1/health
              port: https
              scheme: HTTPS
            initialDelaySeconds: 5
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/v1/health
              port: https
              scheme: HTTPS
            initialDelaySeconds: 5
            periodSeconds: 10
          securityContext:
            runAsNonRoot: true
            runAsUser: 10001
            allowPrivilegeEscalation: false
            seccompProfile:
              type: RuntimeDefault
            capabilities:
              drop:
                - ALL
          resources:
            requests:
              cpu: 50m
              memory: 64Mi
            limits:
              cpu: 500m
              memory: 256Mi
          volumeMounts:
            - name: tls-certs
              mountPath: /certs
              readOnly: true
      volumes:
        - name: tls-certs
          emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: vmrogue-api
  namespace: ${NS}
spec:
  type: NodePort
  selector:
    app: vmrogue-api
  ports:
    - name: https
      port: 443
      targetPort: https
      protocol: TCP
      nodePort: ${NODE_PORT}
YAML
"
info "K8s resources applied"

# ── Step 6: Verify ──
step "Step 6/7: Verifying deployment"
# Chain with && so a failed rollout is not masked by a later kubectl (ssh exits 0 on last cmd).
ssh "${USER}@${HOST}" "
    ${K} -n ${NS} rollout status deployment/vmrogue-api --timeout=180s &&
    echo '' &&
    ${K} -n ${NS} get pods -l app=vmrogue-api -o wide &&
    echo '' &&
    ${K} -n ${NS} get svc vmrogue-api &&
    echo '' &&
    echo 'Cluster / integrations (read-only):' &&
    ( ${K} get nodes -o wide 2>/dev/null | head -5 ) &&
    ( ${K} get crd virtualmachines.kubevirt.io &>/dev/null && echo '  KubeVirt: CRD present' || echo '  KubeVirt: CRD not found (install KubeVirt for VM APIs)' ) &&
    ( ${K} get crd ciliumnetworkpolicies.cilium.io &>/dev/null && echo '  Cilium:   CNP CRD present' || echo '  Cilium:   not detected (egress bootstrap skipped if no CRD)' )
" || error "Deployment rollout failed (on host: ${K} -n ${NS} describe pod -l app=vmrogue-api)"
info "Deployment verified"

# Live Service NodePort for the https port (see port name in manifest).
DISPLAY_NODE_PORT=$(ssh "${USER}@${HOST}" "${K} -n ${NS} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || true)
DISPLAY_NODE_PORT="${DISPLAY_NODE_PORT:-${NODE_PORT}}"

TOTAL_SEC=$(( $(date +%s) - RUN_STARTED_AT ))

echo ""
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo "  ${COLOR_GREEN}Done${COLOR_RESET} ${COLOR_DIM}($(format_duration "${TOTAL_SEC}"))${COLOR_RESET} — ${REMOTE}"
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo ""
echo "  ${COLOR_BOLD}URLs${COLOR_RESET}"
echo "    Dashboard:  https://${HOST}:${DISPLAY_NODE_PORT}/dashboard"
echo "    Operator UI: https://${HOST}:${DISPLAY_NODE_PORT}/dashboard-next/"
echo "    Health:     https://${HOST}:${DISPLAY_NODE_PORT}/api/v1/health"
echo ""
echo "  ${COLOR_DIM}TLS:${COLOR_RESET} self-signed init-container cert (browser warning) unless you mount a Secret at /certs."
echo "  ${COLOR_DIM}API key:${COLOR_RESET} ${API_KEY}"
echo ""
echo "  ${COLOR_DIM}kubectl (on remote):${COLOR_RESET}"
echo "    ${K} -n ${NS} logs deployment/vmrogue-api -f"
echo "    ${K} -n ${NS} get pods"
echo ""
