#!/usr/bin/env bash
# ============================================================================
# deploy-k8s-remote.sh — Deploy Veyron to remote K8s cluster
# ============================================================================
# Rsync source, build binary, build container image, deploy to K8s.
# No systemd — pure Kubernetes deployment.
# The API is served with HTTPS (self-signed cert via initContainer openssl).
# To use your own cert: create a TLS Secret in veyron-system and patch the
# Deployment to mount it at /certs (tls.crt, tls.key) instead of the generated pair.
#
# Usage:
#   ./scripts/deploy-k8s-remote.sh <host> [user]
#   ./scripts/deploy-k8s-remote.sh --help
#
# Optional environment (local):
#   VEYRON_SKIP_CDI=1       — do not install CDI when the DataVolume CRD is missing
#   VEYRON_CDI_VERSION=v1.65.0 — CDI release tag (default below); must match KubeVirt/CDI compatibility on your cluster
#   VEYRON_CONTAINER_RUNTIME_IMPORT — full shell command that reads OCI/docker tar on stdin (default: k3s ctr import, or ctr -n k8s.io for plain kubectl)
#   VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1 — do not apply deploy/k8s/bootstrap/cilium-veyron-egress.yaml when Cilium is installed
#   VEYRON_REQUIRE_KUBEVIRT=1 — fail deploy if KubeVirt VM CRD is missing (after rsync; remote kubectl check)
#   VEYRON_REMOTE_SKIP_SSH_CHECK=1 — skip SSH BatchMode preflight before rsync
#   DEPLOY_SSH_TIMEOUT=20 — SSH ConnectTimeout seconds (preflight only)
#   DEPLOY_SSH_PORT=22 — SSH port for rsync and remote commands
#   NO_COLOR=1 — disable ANSI highlights
#
# RBAC applied with the manifest matches deploy/k8s.yaml ClusterRole (KubeVirt, CDI namespaces
# via pod list, Cilium policy list, Metrics Server PodMetrics, CSIStorageCapacity, MigrationPolicy).
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=lib/deploy-ssh.sh
source "${SCRIPT_DIR}/lib/deploy-ssh.sh"

usage() {
    cat <<'USAGE_EOF'
deploy-k8s-remote.sh — Deploy Veyron API to a remote Kubernetes node (rsync → build → image → CDI → apply).

Usage:
  ./scripts/deploy-k8s-remote.sh <host> [user]
  ./scripts/deploy-k8s-remote.sh --help

Environment (local):
  VEYRON_API_KEY, VEYRON_NODE_PORT, VEYRON_SKIP_CDI, VEYRON_CDI_VERSION
  VEYRON_CONTAINER_RUNTIME_IMPORT
  VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP=1  Skip Cilium egress bootstrap when Cilium CRD exists
  VEYRON_REQUIRE_KUBEVIRT=1              Fail if KubeVirt VM CRD is missing
  VEYRON_REMOTE_SKIP_SSH_CHECK=1         Skip SSH preflight
  DEPLOY_SSH_TIMEOUT                        SSH connect timeout (default 20)
  DEPLOY_SSH_PORT                           SSH port (default 22)
  NO_COLOR=1                               Disable ANSI colors
USAGE_EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

HOST="${1:?Usage: $0 <host> [user]   (see --help)}"
USER="${2:-root}"
REMOTE_DIR="/home/${USER}/veyron"
API_KEY="${VEYRON_API_KEY:-CHANGE_ME}"
NODE_PORT="${VEYRON_NODE_PORT:-30151}"
NS="${VEYRON_NAMESPACE:-veyron-system}"
CDI_VERSION="${VEYRON_CDI_VERSION:-v1.65.0}"
SKIP_CDI="${VEYRON_SKIP_CDI:-0}"
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

if [[ "${VEYRON_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    step "Preflight: SSH (${REMOTE}, port ${DEPLOY_SSH_PORT})"
    SSH_ERR=""
    if ! SSH_ERR=$(deploy_ssh_preflight "${REMOTE}"); then
        error "SSH preflight failed on port ${DEPLOY_SSH_PORT}: ${SSH_ERR} (try: ssh -p ${DEPLOY_SSH_PORT} ${REMOTE} or VEYRON_REMOTE_SKIP_SSH_CHECK=1 $0 $*)"
    fi
    info "SSH OK (port ${DEPLOY_SSH_PORT}, timeout ${DEPLOY_SSH_TIMEOUT}s)"
fi

# Remote cluster: k3s (bundled kubectl + ctr) vs generic Kubernetes (kubectl + containerd ctr).
REMOTE_K8S_FLAVOR=$(deploy_ssh "${USER}@${HOST}" '
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
IMPORT_CMD="${VEYRON_CONTAINER_RUNTIME_IMPORT:-${IMPORT_DEFAULT}}"

echo ""
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo "  ${COLOR_BOLD}  Veyron → Kubernetes (remote API)${COLOR_RESET}"
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo ""
echo "  ${COLOR_DIM}Target:${COLOR_RESET}     ${REMOTE}"
echo "  ${COLOR_DIM}Flavor:${COLOR_RESET}     ${REMOTE_K8S_FLAVOR}  (${K})"
echo "  ${COLOR_DIM}Import:${COLOR_RESET}     ${IMPORT_CMD}"
echo "  ${COLOR_DIM}Namespace:${COLOR_RESET}  ${NS}"
echo "  ${COLOR_DIM}NodePort:${COLOR_RESET}   ${NODE_PORT} → pod :5151 (TLS)"
echo "  ${COLOR_DIM}API key:${COLOR_RESET}    ${API_KEY}"
echo ""

# ── Step 1: Rsync ──
step "Step 1/7: Syncing source to ${HOST}"
rsync -az --delete \
    --exclude target/ --exclude .git/ --exclude operator/bin/ \
    -e "$(deploy_rsync_ssh)" \
    . "${USER}@${HOST}:${REMOTE_DIR}/"
info "Source synced"

# ── Step 2: Build binary ──
step "Step 2/7: Building release binary"
deploy_ssh "${USER}@${HOST}" "
    source \$HOME/.cargo/env 2>/dev/null || true
    cd ${REMOTE_DIR}
    cargo build --release --bin veyron 2>&1 | tail -3
    strip target/release/veyron 2>/dev/null || true
    ls -lh target/release/veyron
"
info "Binary built"

# ── Step 3: Build container image ──
step "Step 3/7: Building container image"
deploy_ssh "${USER}@${HOST}" "
    cd ${REMOTE_DIR}
    cp target/release/veyron /tmp/veyron-binary
    podman build --format docker -t localhost/veyron:latest -f Dockerfile.deploy /tmp 2>&1 | tail -3
    podman save localhost/veyron:latest | ${IMPORT_CMD} 2>&1
" || error "Image import failed (non-k3s: install containerd ctr, set VEYRON_CONTAINER_RUNTIME_IMPORT, or push to a registry and adjust the Deployment image / pullPolicy)"
info "Container image built and imported"

# ── Step 4: CDI (DataVolume / import) when missing ──
# KubeVirt disk pipelines and examples often need CDI; installing it here avoids a separate manual step on fresh clusters.
step "Step 4/7: Ensuring CDI (containerized-data-importer)"
if [[ "${SKIP_CDI}" == "1" ]]; then
  info "Skipped CDI install (VEYRON_SKIP_CDI=1)"
else
  "${SCRIPT_DIR}/ensure-cdi-remote.sh" "${USER}@${HOST}" "${CDI_VERSION}" ||
    error "CDI install or wait failed (set VEYRON_SKIP_CDI=1 to skip, or fix cluster network / storage)"
  info "CDI available for DataVolume workflows"
fi

# ── Step 5: Deploy to K8s ──
# Bump pod template every run so apply triggers a rollout even when the image ref
# stays localhost/veyron:latest with imagePullPolicy: Never (otherwise old pods
# keep running the previous image layers).
DEPLOY_STAMP="$(date +%s)-${RANDOM}"

step "Step 5/7: Deploying to Kubernetes"
# shellcheck disable=SC2029
deploy_ssh "${USER}@${HOST}" "
    ${K} create namespace ${NS} 2>/dev/null || true

    if [[ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" && \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ]] && ${K} get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
      for cilium_yaml in ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-veyron-egress.yaml ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-veyron-egress.yaml; do
        if [[ -f \"\${cilium_yaml}\" ]]; then
          echo 'Applying Cilium egress bootstrap for ${NS} (veyron / nats workloads)...'
          sed -e \"s|__VEYRON_APP_NAMESPACE__|${NS}|g\" \
              -e \"s|namespace: veyron-system|namespace: ${NS}|g\" \
              -e \"s|namespace: veyron-system|namespace: ${NS}|g\" \
              \"\${cilium_yaml}\" | ${K} apply -f -
          break
        fi
      done
    fi
    if [[ \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" && \"${VEYRON_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ]] && ${K} get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null && [[ -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml ]]; then
      echo 'Applying Cilium clusterwide egress for KubeVirt virt-launcher (VM guest internet)...'
      ${K} apply -f ${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-kubevirt-virt-launcher-clusterwide-egress.yaml
    fi
    if [[ \"${VEYRON_REQUIRE_KUBEVIRT:-}\" == \"1\" || \"${VEYRON_REQUIRE_KUBEVIRT:-}\" == \"true\" ]]; then
      ${K} get crd virtualmachines.kubevirt.io &>/dev/null || { echo 'KubeVirt CRD virtualmachines.kubevirt.io not found'; exit 1; }
    fi

    for crd_yaml in ${REMOTE_DIR}/operator/config/crd/bases/veyron.io_vmtemplates.yaml ${REMOTE_DIR}/operator/config/crd/bases/veyron.io_vmprofiles.yaml; do
      if [[ -f \"\${crd_yaml}\" ]]; then
        ${K} apply -f \"\${crd_yaml}\"
      fi
    done
    if ${K} get crd vmtemplates.veyron.io &>/dev/null; then
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
  name: veyron
  namespace: ${NS}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: veyron
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
  - apiGroups: ['veyron.io']
    resources: ['veyronvms', 'veyronblueprints', 'veyronpolicies', 'veyroninsights', 'veyronactions', 'vmtemplates', 'vmprofiles']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['veyron.io']
    resources: ['veyronvms/status', 'veyronblueprints/status', 'veyronpolicies/status', 'veyroninsights/status', 'veyronactions/status', 'vmtemplates/status', 'vmprofiles/status']
    verbs: ['get', 'update', 'patch']
  - apiGroups: ['']
    resources: ['namespaces', 'nodes', 'pods', 'pods/log', 'events', 'persistentvolumeclaims', 'configmaps', 'resourcequotas']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['']
    resources: ['nodes']
    verbs: ['patch', 'update']
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
  name: veyron
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: veyron
subjects:
  - kind: ServiceAccount
    name: veyron
    namespace: ${NS}
---
apiVersion: v1
kind: Secret
metadata:
  name: veyron-api-key
  namespace: ${NS}
type: Opaque
stringData:
  api-key: "${API_KEY}"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: veyron-api
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
      app: veyron-api
  template:
    metadata:
      labels:
        app: veyron-api
      annotations:
        veyron.io/deployed-at: "${DEPLOY_STAMP}"
    spec:
      serviceAccountName: veyron
      initContainers:
        - name: gen-cert
          image: alpine/openssl:3.3.2
          command: ["/bin/sh", "-c"]
          args:
            - |
              openssl req -x509 -nodes -days 3650 \
                -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
                -keyout /certs/tls.key -out /certs/tls.crt \
                -subj "/CN=veyron/O=veyron" \
                -addext "subjectAltName=DNS:veyron-api,DNS:veyron-api.${NS}.svc,DNS:localhost,IP:127.0.0.1"
              chown 10001:10001 /certs/tls.key /certs/tls.crt
              chmod 600 /certs/tls.key && chmod 644 /certs/tls.crt
          securityContext:
            runAsUser: 0
            runAsNonRoot: false
          volumeMounts:
            - name: tls-certs
              mountPath: /certs
      containers:
        - name: veyron
          image: localhost/veyron:latest
          imagePullPolicy: Never
          args: ['api-serve', '--port', '5151', '--host', '0.0.0.0', '--tls', '--tls-cert', '/certs/tls.crt', '--tls-key', '/certs/tls.key']
          env:
            - name: VEYRON_API_KEY
              valueFrom:
                secretKeyRef:
                  name: veyron-api-key
                  key: api-key
            - name: VEYRON_API_KEY
              valueFrom:
                secretKeyRef:
                  name: veyron-api-key
                  key: api-key
            - name: RUST_LOG
              value: info
            - name: VEYRON_API_NODE_HOST
              valueFrom:
                fieldRef:
                  fieldPath: status.hostIP
            - name: VEYRON_API_NODE_HOST
              valueFrom:
                fieldRef:
                  fieldPath: status.hostIP
            - name: VEYRON_API_NODE_PORT
              value: '${NODE_PORT}'
            - name: VEYRON_API_NODE_PORT
              value: '${NODE_PORT}'
            - name: VEYRON_CLUSTER_DNS
              value: '10.43.0.10'
            - name: VEYRON_CLUSTER_DNS
              value: '10.43.0.10'
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
  name: veyron-api
  namespace: ${NS}
spec:
  type: NodePort
  selector:
    app: veyron-api
  ports:
    - name: https
      port: 443
      targetPort: https
      protocol: TCP
      nodePort: ${NODE_PORT}
YAML
" || {
  # Fallback: if kubectl apply fails (e.g. immutable selector mismatch from an old deployment),
  # patch the pod template directly so the new image + annotation bump triggers a rollout.
  deploy_ssh "${USER}@${HOST}" "
    ${K} -n ${NS} patch deployment/veyron-api --type=strategic -p \
      '{\"spec\":{\"template\":{\"metadata\":{\"annotations\":{\"veyron.io/deployed-at\":\"${DEPLOY_STAMP}\"}},\"spec\":{\"containers\":[{\"name\":\"veyron\",\"image\":\"localhost/veyron:latest\"}]}}}}' \
      2>/dev/null || true
  " || true
}
info "K8s resources applied"

# ── Step 6: Verify ──
step "Step 6/7: Verifying deployment"
# Chain with && so a failed rollout is not masked by a later kubectl (ssh exits 0 on last cmd).
deploy_ssh "${USER}@${HOST}" "
    ${K} -n ${NS} rollout status deployment/veyron-api --timeout=180s &&
    echo '' &&
    ${K} -n ${NS} get pods -l app=veyron-api -o wide &&
    echo '' &&
    ${K} -n ${NS} get svc veyron-api &&
    echo '' &&
    echo 'Cluster / integrations (read-only):' &&
    ( ${K} get nodes -o wide 2>/dev/null | head -5 ) &&
    ( ${K} get crd virtualmachines.kubevirt.io &>/dev/null && echo '  KubeVirt: CRD present' || echo '  KubeVirt: CRD not found (install KubeVirt for VM APIs)' ) &&
    ( ${K} get crd ciliumnetworkpolicies.cilium.io &>/dev/null && echo '  Cilium:   CNP CRD present' || echo '  Cilium:   not detected (egress bootstrap skipped if no CRD)' )
" || error "Deployment rollout failed (on host: ${K} -n ${NS} describe pod -l app=veyron-api)"
info "Deployment verified"

# Live Service NodePort for the https port (see port name in manifest).
DISPLAY_NODE_PORT=$(deploy_ssh "${USER}@${HOST}" "${K} -n ${NS} get svc veyron-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || true)
DISPLAY_NODE_PORT="${DISPLAY_NODE_PORT:-${NODE_PORT}}"

TOTAL_SEC=$(( $(date +%s) - RUN_STARTED_AT ))

echo ""
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo "  ${COLOR_GREEN}Done${COLOR_RESET} ${COLOR_DIM}($(format_duration "${TOTAL_SEC}"))${COLOR_RESET} — ${REMOTE}"
echo "  ${COLOR_BOLD}════════════════════════════════════════${COLOR_RESET}"
echo ""
echo "  ${COLOR_BOLD}URLs${COLOR_RESET}"
echo "    Dashboard:  https://${HOST}:${DISPLAY_NODE_PORT}/dashboard"
echo "    Health:     https://${HOST}:${DISPLAY_NODE_PORT}/api/v1/health"
echo ""
echo "  ${COLOR_DIM}TLS:${COLOR_RESET} self-signed init-container cert (browser warning) unless you mount a Secret at /certs."
echo "  ${COLOR_DIM}API key:${COLOR_RESET} ${API_KEY}"
echo ""
echo "  ${COLOR_DIM}kubectl (on remote):${COLOR_RESET}"
echo "    ${K} -n ${NS} logs deployment/veyron-api -f"
echo "    ${K} -n ${NS} get pods"
echo ""
