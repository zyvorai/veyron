#!/usr/bin/env bash
# ============================================================================
# deploy-k8s-remote.sh — Deploy VMRogue to remote K8s cluster
# ============================================================================
# Rsync source, build binary, build container image, deploy to K8s.
# No systemd — pure Kubernetes deployment.
#
# Usage:
#   ./scripts/deploy-k8s-remote.sh <host> [user]
#   ./scripts/deploy-k8s-remote.sh 185.165.240.5 sus
# ============================================================================

set -euo pipefail

HOST="${1:?Usage: $0 <host> [user]}"
USER="${2:-root}"
REMOTE_DIR="/home/${USER}/vmrogue"
API_KEY="${VMROGUE_API_KEY:-Admin@321}"
NODE_PORT="${VMROGUE_NODE_PORT:-30151}"
NS="vmrogue-system"

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
echo "  NodePort:  ${NODE_PORT}"
echo "  API Key:   ${API_KEY}"
echo ""

# ── Step 1: Rsync ──
step "Step 1/5: Syncing source to ${HOST}"
rsync -az --delete \
    --exclude target/ --exclude .git/ --exclude operator/bin/ \
    . "${USER}@${HOST}:${REMOTE_DIR}/"
info "Source synced"

# ── Step 2: Build binary ──
step "Step 2/5: Building release binary"
ssh "${USER}@${HOST}" "
    source \$HOME/.cargo/env 2>/dev/null || true
    cd ${REMOTE_DIR}
    cargo build --release 2>&1 | tail -3
    strip target/release/vmrogue 2>/dev/null || true
    ls -lh target/release/vmrogue
"
info "Binary built"

# ── Step 3: Build container image ──
step "Step 3/5: Building container image"
ssh "${USER}@${HOST}" "
    cd ${REMOTE_DIR}
    cp target/release/vmrogue /tmp/vmrogue-binary
    podman build --format docker -t localhost/vmrogue:latest -f Dockerfile.deploy /tmp 2>&1 | tail -3
    podman save localhost/vmrogue:latest | sudo /usr/local/bin/k3s ctr images import - 2>&1
"
info "Container image built and imported"

# ── Step 4: Deploy to K8s ──
step "Step 4/5: Deploying to Kubernetes"
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
  selector:
    matchLabels:
      app: vmrogue-api
  template:
    metadata:
      labels:
        app: vmrogue-api
    spec:
      serviceAccountName: vmrogue
      containers:
        - name: vmrogue
          image: localhost/vmrogue:latest
          imagePullPolicy: Never
          args: ['api-serve', '--port', '5151', '--host', '0.0.0.0']
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
              name: http
          resources:
            requests:
              cpu: 50m
              memory: 64Mi
            limits:
              cpu: 500m
              memory: 256Mi
          livenessProbe:
            httpGet:
              path: /api/v1/health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/v1/health
              port: http
            initialDelaySeconds: 3
            periodSeconds: 10
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
    - name: http
      port: 5151
      targetPort: 5151
      nodePort: ${NODE_PORT}
YAML
"
info "K8s resources applied"

# ── Step 5: Verify ──
step "Step 5/5: Verifying deployment"
ssh "${USER}@${HOST}" "
    ${K} -n ${NS} rollout status deployment/vmrogue-api --timeout=60s
    echo ''
    ${K} -n ${NS} get pods -l app=vmrogue-api -o wide
    echo ''
    ${K} -n ${NS} get svc vmrogue-api
"
info "Deployment verified"

echo ""
echo "  ============================================"
echo "  Deployment complete: ${USER}@${HOST}"
echo "  ============================================"
echo ""
echo "  Dashboard:  http://${HOST}:${NODE_PORT}/dashboard"
echo "  API Key:    ${API_KEY}"
echo "  Health:     http://${HOST}:${NODE_PORT}/api/v1/health"
echo ""
echo "  kubectl:"
echo "    ${K} -n ${NS} logs deployment/vmrogue-api -f"
echo "    ${K} -n ${NS} get pods"
echo ""
