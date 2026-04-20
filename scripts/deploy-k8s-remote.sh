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
#   ./scripts/deploy-k8s-remote.sh 185.165.240.5 sus
#
# Optional environment (local):
#   VMROGUE_SKIP_CDI=1       — do not install CDI when the DataVolume CRD is missing
#   VMROGUE_CDI_VERSION=v1.65.0 — CDI release tag (default below); must match KubeVirt/CDI compatibility on your cluster
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

HOST="${1:?Usage: $0 <host> [user]}"
USER="${2:-root}"
REMOTE_DIR="/home/${USER}/vmrogue"
API_KEY="${VMROGUE_API_KEY:-Admin@321}"
NODE_PORT="${VMROGUE_NODE_PORT:-30151}"
NS="vmrogue-system"
CDI_VERSION="${VMROGUE_CDI_VERSION:-v1.65.0}"
SKIP_CDI="${VMROGUE_SKIP_CDI:-0}"

info()  { echo "  [✓] $*"; }
step()  { echo ""; echo "  --- $*"; }
error() { echo "  [✗] $*"; exit 1; }

K="sudo /usr/local/bin/k3s kubectl"

echo ""
echo "  ============================================"
echo "    VMRogue K8s Deployment"
echo "  ============================================"
echo ""
echo "  Host:      ${USER}@${HOST}"
echo "  Namespace: ${NS}"
echo "  HTTPS NodePort: ${NODE_PORT} (Service targets TLS :5151 in the pod)"
echo "  API Key:        ${API_KEY}"
echo ""

# ── Step 1: Rsync ──
step "Step 1/6: Syncing source to ${HOST}"
rsync -az --delete \
    --exclude target/ --exclude .git/ --exclude operator/bin/ \
    . "${USER}@${HOST}:${REMOTE_DIR}/"
info "Source synced"

# ── Step 2: Build binary ──
step "Step 2/6: Building release binary"
ssh "${USER}@${HOST}" "
    source \$HOME/.cargo/env 2>/dev/null || true
    cd ${REMOTE_DIR}
    cargo build --release 2>&1 | tail -3
    strip target/release/vmrogue 2>/dev/null || true
    ls -lh target/release/vmrogue
"
info "Binary built"

# ── Step 3: Build container image ──
step "Step 3/6: Building container image"
ssh "${USER}@${HOST}" "
    cd ${REMOTE_DIR}
    cp target/release/vmrogue /tmp/vmrogue-binary
    podman build --format docker -t localhost/vmrogue:latest -f Dockerfile.deploy /tmp 2>&1 | tail -3
    podman save localhost/vmrogue:latest | sudo /usr/local/bin/k3s ctr images import - 2>&1
"
info "Container image built and imported"

# ── Step 4: CDI (DataVolume / import) when missing ──
# KubeVirt disk pipelines and examples often need CDI; installing it here avoids a separate manual step on fresh k3s nodes.
step "Step 4/6: Ensuring CDI (containerized-data-importer)"
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

step "Step 5/6: Deploying to Kubernetes"
ssh "${USER}@${HOST}" "
    ${K} create namespace ${NS} 2>/dev/null || true

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
    resources: ['virtualmachines', 'virtualmachineinstances', 'virtualmachineinstancemigrations']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['subresources.kubevirt.io']
    resources: ['virtualmachineinstances/vnc', 'virtualmachineinstances/console']
    verbs: ['get']
  - apiGroups: ['subresources.kubevirt.io']
    resources: ['virtualmachineinstances/pause', 'virtualmachineinstances/unpause']
    verbs: ['update']
  - apiGroups: ['snapshot.kubevirt.io']
    resources: ['virtualmachinesnapshots', 'virtualmachinerestores', 'virtualmachinesnapshotcontents']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['vmrogue.io']
    resources: ['vmroguevms', 'vmrogueblueprints', 'vmroguepolicies', 'vmrogueinsights', 'vmrogueactions']
    verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
  - apiGroups: ['vmrogue.io']
    resources: ['vmroguevms/status', 'vmrogueblueprints/status', 'vmroguepolicies/status', 'vmrogueinsights/status', 'vmrogueactions/status']
    verbs: ['get', 'update', 'patch']
  - apiGroups: ['']
    resources: ['namespaces', 'nodes', 'pods', 'pods/log', 'events', 'services', 'persistentvolumeclaims', 'configmaps', 'secrets', 'resourcequotas']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['']
    resources: ['persistentvolumeclaims', 'configmaps']
    verbs: ['create', 'update', 'patch', 'delete']
  - apiGroups: ['storage.k8s.io']
    resources: ['storageclasses']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['networking.k8s.io']
    resources: ['networkpolicies', 'ingresses']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['rbac.authorization.k8s.io']
    resources: ['clusterroles', 'clusterrolebindings', 'roles', 'rolebindings']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['apps']
    resources: ['deployments', 'statefulsets', 'daemonsets']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['autoscaling']
    resources: ['horizontalpodautoscalers']
    verbs: ['get', 'list', 'watch']
  - apiGroups: ['apiextensions.k8s.io']
    resources: ['customresourcedefinitions']
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
  api-key: '${API_KEY}'
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
        vmrogue.io/deployed-at: '${DEPLOY_STAMP}'
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
step "Step 6/6: Verifying deployment"
# Chain with && so a failed rollout is not masked by a later kubectl (ssh exits 0 on last cmd).
ssh "${USER}@${HOST}" "
    ${K} -n ${NS} rollout status deployment/vmrogue-api --timeout=180s &&
    echo '' &&
    ${K} -n ${NS} get pods -l app=vmrogue-api -o wide &&
    echo '' &&
    ${K} -n ${NS} get svc vmrogue-api
" || error "Deployment rollout failed (on host: ${K} -n ${NS} describe pod -l app=vmrogue-api)"
info "Deployment verified"

# Live Service NodePort for the https port (see port name in manifest).
DISPLAY_NODE_PORT=$(ssh "${USER}@${HOST}" "${K} -n ${NS} get svc vmrogue-api -o jsonpath='{.spec.ports[?(@.name==\"https\")].nodePort}' 2>/dev/null" || true)
DISPLAY_NODE_PORT="${DISPLAY_NODE_PORT:-${NODE_PORT}}"

echo ""
echo "  ============================================"
echo "  Deployment complete: ${USER}@${HOST}"
echo "  ============================================"
echo ""
echo "  Dashboard:  https://${HOST}:${DISPLAY_NODE_PORT}/dashboard"
echo "  Health:     https://${HOST}:${DISPLAY_NODE_PORT}/api/v1/health"
echo "  TLS:        Self-signed in-cluster (browser warning) unless you replace /certs from a Secret"
echo "  API Key:    ${API_KEY}"
echo ""
echo "  kubectl:"
echo "    ${K} -n ${NS} logs deployment/vmrogue-api -f"
echo "    ${K} -n ${NS} get pods"
echo ""
