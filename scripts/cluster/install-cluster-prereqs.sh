#!/usr/bin/env bash
# Install common cluster components expected by v9s (idempotent where possible).
#
# Requires: kubectl, cluster admin access, outbound HTTPS to GitHub / Helm repos. Helm is required only
# when installing Cilium, MetalLB, or Prometheus (see below).
#
# Opt-in install (off unless set to 1/true/yes):
#   V9S_INSTALL_SNAPSHOT_CONTROLLER=1   CSI VolumeSnapshot CRDs + snapshot-controller
#   V9S_INSTALL_ROOK_CEPH=1
#   V9S_INSTALL_METALLB=1
#   V9S_INSTALL_PROMETHEUS=1
#   V9S_INSTALL_MULTUS=1                secondary CNI (after default CNI; see V9S_MULTUS_MANIFEST_URL)
#
# metrics-server: installed by default when absent; disable with V9S_SKIP_METRICS_SERVER=1 or
#   V9S_INSTALL_METRICS_SERVER=0 (false/no).
#
# Skip parts with env (any non-empty true-ish value), wins over INSTALL:
#   V9S_SKIP_CILIUM
#   V9S_SKIP_MULTUS
#   V9S_SKIP_METRICS_SERVER
#   V9S_SKIP_SNAPSHOT_CONTROLLER
#   V9S_SKIP_KUBEVIRT
#   V9S_SKIP_CDI
#   V9S_SKIP_ROOK_CEPH
#   V9S_SKIP_PROMETHEUS      Skip Prometheus even when V9S_INSTALL_PROMETHEUS=1
#   V9S_SKIP_METALLB
#
# MetalLB: after install, set V9S_METALLB_IP_POOL to e.g. 192.168.1.240-192.168.1.250 (or a CIDR) to
# auto-apply an IPAddressPool + L2Advertisement in V9S_METALLB_NAMESPACE (default metallb-system).
#
# Versions (override via env):
#   V9S_CILIUM_CHART_VERSION    (default: see cluster/versions.env)  Helm chart for Cilium CNI
#   V9S_KUBEVIRT_VERSION        (default: see cluster/versions.env)
#   V9S_CDI_VERSION             (default: see cluster/versions.env)
#   V9S_SNAPSHOTTER_VERSION     external-snapshotter git ref
#   V9S_METRICS_SERVER_MANIFEST components.yaml URL
#   V9S_MULTUS_MANIFEST_URL     (default: multus-cni thick DaemonSet, version from cluster/versions.env)
#   V9S_ROOK_REF                git tag for Rook manifests
#
# ALL baseline versions live in cluster/versions.env — do not hardcode them here.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

info()  { echo "  [cluster-prereq] $*"; }
step()  { echo "  [cluster-prereq] .. $*"; }
warn()  { echo "  [cluster-prereq] !! $*" >&2; }

# Usage: should_skip VAR_NAME — true if that env var requests skip (${!name} indirection).
should_skip() {
    local _n="$1"
    local _val="${!_n-}"
    case "$_val" in 1|true|TRUE|yes|Yes) return 0 ;; *) return 1 ;; esac
}

# Opt-in: run optional component only when VAR is set true-ish.
should_install() {
    local _n="$1"
    local _val="${!_n-}"
    case "$_val" in 1|true|TRUE|yes|Yes) return 0 ;; *) return 1 ;; esac
}

# Baseline versions come from the single source of truth; V9S_* still overrides.
# shellcheck source=./versions.env
source "${SCRIPT_DIR}/versions.env"

CILIUM_CHART_VERSION="${V9S_CILIUM_CHART_VERSION:-${CILIUM_CHART_VERSION}}"
CILIUM_K3S_VALUES="${V9S_CILIUM_K3S_VALUES:-${SCRIPT_DIR}/cilium-k3s-values.yaml}"
KUBEVIRT_VERSION="${V9S_KUBEVIRT_VERSION:-${KUBEVIRT_VERSION}}"
CDI_VERSION="${V9S_CDI_VERSION:-${CDI_VERSION}}"
SNAPSHOTTER_REF="${V9S_SNAPSHOTTER_VERSION:-${SNAPSHOTTER_VERSION}}"
METRICS_SERVER_URL="${V9S_METRICS_SERVER_MANIFEST:-https://github.com/kubernetes-sigs/metrics-server/releases/download/${METRICS_SERVER_VERSION}/components.yaml}"

PROMETHEUS_RELEASE="${V9S_PROMETHEUS_HELM_RELEASE:-v9s-prom}"
PROMETHEUS_NS="${V9S_PROMETHEUS_NAMESPACE:-monitoring}"

ROOK_REF="${V9S_ROOK_REF:-${ROOK_VERSION}}"
ROOK_NS="${V9S_ROOK_NAMESPACE:-rook-ceph}"

METALLB_RELEASE="${V9S_METALLB_HELM_RELEASE:-metallb}"
METALLB_NS="${V9S_METALLB_NAMESPACE:-metallb-system}"
METALLB_IP_POOL="${V9S_METALLB_IP_POOL:-}"
METALLB_POOL_NAME="${V9S_METALLB_POOL_NAME:-v9s-pool}"
METALLB_L2_NAME="${V9S_METALLB_L2_NAME:-v9s-l2}"

MULTUS_MANIFEST_URL="${V9S_MULTUS_MANIFEST_URL:-https://raw.githubusercontent.com/k8snetworkplumbingwg/multus-cni/${MULTUS_VERSION}/deployments/multus-daemonset-thick.yml}"

preflight() {
    command -v kubectl >/dev/null 2>&1 || { echo "kubectl not found"; exit 1; }
    kubectl cluster-info >/dev/null 2>&1 || { echo "cannot reach cluster (KUBECONFIG?)"; exit 1; }
}

require_helm() {
    command -v helm >/dev/null 2>&1 || { echo "helm not found (required for this step)"; exit 1; }
}

install_cilium() {
    should_skip V9S_SKIP_CILIUM && { info "skip Cilium (V9S_SKIP_CILIUM)"; return 0; }
    if kubectl get crd ciliumnetworkpolicies.cilium.io >/dev/null 2>&1; then
        info "Cilium CRDs already present"
        return 0
    fi
    if kubectl get ds kube-flannel-ds -n kube-system >/dev/null 2>&1 \
        || kubectl get ds -n kube-system -l app=flannel -o name >/dev/null 2>&1; then
        warn "Flannel CNI detected — skipping automatic Cilium install (use a cluster without Flannel, e.g. k3s --flannel-backend=none, then re-run). Set V9S_SKIP_CILIUM=1 to silence."
        return 0
    fi
    if kubectl get ds calico-node -n kube-system >/dev/null 2>&1; then
        warn "Calico CNI detected — skipping Cilium install"
        return 0
    fi
    require_helm
    step "Cilium CNI (Helm chart ${CILIUM_CHART_VERSION})"
    helm repo add cilium https://helm.cilium.io/ 2>/dev/null || true
    helm repo update cilium 2>/dev/null || helm repo update
    local helm_args=(
        upgrade --install cilium cilium/cilium
        --version "$CILIUM_CHART_VERSION"
        --namespace kube-system
        --wait --timeout 15m
    )
    if [ -f "$CILIUM_K3S_VALUES" ]; then
        helm_args+=(-f "$CILIUM_K3S_VALUES")
    fi
    helm "${helm_args[@]}"
    info "Cilium installed"
}

# When Cilium is already present, align routing/masquerade (pod SNAT) for KubeVirt guest internet.
ensure_cilium_pod_egress() {
    should_skip V9S_SKIP_CILIUM && return 0
    if ! kubectl get crd ciliumnetworkpolicies.cilium.io >/dev/null 2>&1; then
        return 0
    fi
    local ensure="${SCRIPT_DIR}/bootstrap/ensure-cilium-pod-egress.sh"
    if [ -x "$ensure" ]; then
        step "Cilium pod egress (masquerade / SNAT)"
        bash "$ensure" || warn "Cilium egress check failed — VMs may have no internet until fixed"
    fi
}

install_multus() {
    should_skip V9S_SKIP_MULTUS && { info "skip Multus (V9S_SKIP_MULTUS)"; return 0; }
    if ! should_install V9S_INSTALL_MULTUS; then
        info "skip Multus (set V9S_INSTALL_MULTUS=1)"
        return 0
    fi
    if kubectl get ds kube-multus-ds -n kube-system >/dev/null 2>&1; then
        info "Multus already present (kube-multus-ds)"
        return 0
    fi
    step "Multus CNI (${MULTUS_MANIFEST_URL})"
    kubectl apply -f "$MULTUS_MANIFEST_URL"
    kubectl rollout status ds/kube-multus-ds -n kube-system --timeout=300s 2>/dev/null \
        || warn "kube-multus-ds rollout not confirmed — check: kubectl get pods -n kube-system -l app=multus"
    info "Multus installed"
}

install_metrics_server() {
    should_skip V9S_SKIP_METRICS_SERVER && { info "skip metrics-server (V9S_SKIP_METRICS_SERVER)"; return 0; }
    case "${V9S_INSTALL_METRICS_SERVER:-1}" in
        0|false|FALSE|no|No)
            info "skip metrics-server (V9S_INSTALL_METRICS_SERVER=0)"
            return 0
            ;;
    esac
    if kubectl get deployment metrics-server -n kube-system >/dev/null 2>&1; then
        info "metrics-server already present"
        return 0
    fi
    step "metrics-server (${METRICS_SERVER_URL##*/})"
    kubectl apply -f "$METRICS_SERVER_URL"
    # k3s / many lab clusters need insecure kubelet TLS
    kubectl patch deployment metrics-server -n kube-system --type='json' \
        -p='[{"op": "add", "path": "/spec/template/spec/containers/0/args/-", "value": "--kubelet-insecure-tls"}]' 2>/dev/null \
        || true
    kubectl rollout status deployment/metrics-server -n kube-system --timeout=180s 2>/dev/null || warn "metrics-server rollout not confirmed"
    info "metrics-server applied"
}

install_snapshot_controller() {
    should_skip V9S_SKIP_SNAPSHOT_CONTROLLER && { info "skip snapshot controller (V9S_SKIP_SNAPSHOT_CONTROLLER)"; return 0; }
    if ! should_install V9S_INSTALL_SNAPSHOT_CONTROLLER; then
        info "skip CSI snapshot controller (set V9S_INSTALL_SNAPSHOT_CONTROLLER=1)"
        return 0
    fi
    if kubectl get crd volumesnapshotclasses.snapshot.storage.k8s.io >/dev/null 2>&1; then
        info "snapshot.storage.k8s.io CRDs already present"
    else
        step "CSI snapshot CRDs (external-snapshotter ${SNAPSHOTTER_REF})"
        kubectl apply -k "github.com/kubernetes-csi/external-snapshotter/client/config/crd?ref=${SNAPSHOTTER_REF}"
    fi
    step "snapshot-controller deployment"
    kubectl apply -k "github.com/kubernetes-csi/external-snapshotter/deploy/kubernetes/snapshot-controller?ref=${SNAPSHOTTER_REF}"
    kubectl rollout status deployment/snapshot-controller -n kube-system --timeout=180s 2>/dev/null \
        || kubectl rollout status deployment -n kube-system -l app=snapshot-controller --timeout=180s 2>/dev/null \
        || warn "snapshot-controller rollout not confirmed"
    info "VolumeSnapshot API ready"
}

install_kubevirt() {
    should_skip V9S_SKIP_KUBEVIRT && { info "skip KubeVirt (V9S_SKIP_KUBEVIRT)"; return 0; }
    if kubectl get crd kubevirts.kubevirt.io >/dev/null 2>&1; then
        info "KubeVirt CRD already present"
    else
        step "KubeVirt operator (${KUBEVIRT_VERSION})"
        kubectl apply -f "https://github.com/kubevirt/kubevirt/releases/download/${KUBEVIRT_VERSION}/kubevirt-operator.yaml"
    fi
    if ! kubectl get kubevirt kubevirt -n kubevirt >/dev/null 2>&1; then
        step "KubeVirt CR"
        kubectl apply -f "https://github.com/kubevirt/kubevirt/releases/download/${KUBEVIRT_VERSION}/kubevirt-cr.yaml"
    fi
    step "waiting for KubeVirt (this may take several minutes)"
    local i
    for i in $(seq 1 120); do
        local phase
        phase=$(kubectl get kubevirt kubevirt -n kubevirt -o jsonpath='{.status.phase}' 2>/dev/null || true)
        if [ "$phase" = "Deployed" ]; then
            info "KubeVirt Deployed"
            return 0
        fi
        sleep 5
    done
    warn "KubeVirt did not report Deployed in time — check: kubectl get kubevirt -n kubevirt"
}

install_cdi() {
    should_skip V9S_SKIP_CDI && { info "skip CDI (V9S_SKIP_CDI)"; return 0; }
    if kubectl get crd cdis.cdi.kubevirt.io >/dev/null 2>&1; then
        info "CDI CRD already present"
    else
        step "CDI operator (${CDI_VERSION})"
        kubectl apply -f "https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-operator.yaml"
    fi
    if ! kubectl get cdi cdi >/dev/null 2>&1; then
        kubectl apply -f "https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-cr.yaml"
    fi
    step "waiting for CDI (up to 5m)"
    local _i
    for _i in $(seq 1 60); do
        if kubectl get cdi cdi -o jsonpath='{.status.conditions[?(@.type=="Available")].status}' 2>/dev/null | grep -qE 'True|true'; then
            info "CDI Available"
            return 0
        fi
        sleep 5
    done
    warn "CDI availability not confirmed — check: kubectl get cdi, kubectl get pods -n cdi"
}

install_rook_ceph() {
    should_skip V9S_SKIP_ROOK_CEPH && { info "skip Rook Ceph (V9S_SKIP_ROOK_CEPH)"; return 0; }
    if ! should_install V9S_INSTALL_ROOK_CEPH; then
        info "skip Rook Ceph (set V9S_INSTALL_ROOK_CEPH=1)"
        return 0
    fi
    local base="https://raw.githubusercontent.com/rook/rook/${ROOK_REF}/deploy/examples"

    step "Rook Ceph operator + test cluster (${ROOK_REF})"
    kubectl apply -f "${base}/crds.yaml"
    kubectl apply -f "${base}/common.yaml"
    kubectl apply -f "${base}/operator.yaml"

    step "waiting for rook-ceph operator"
    kubectl wait --for=condition=ready pod -l app=rook-ceph-operator -n "${ROOK_NS}" --timeout=300s 2>/dev/null \
        || kubectl wait --for=condition=ready pod -n "${ROOK_NS}" --timeout=300s -l app.kubernetes.io/name=rook-ceph 2>/dev/null \
        || warn "rook operator readiness not confirmed — continuing"

    kubectl apply -f "${base}/cluster-test.yaml"

    step "waiting for Ceph cluster (cluster-test may require free disks on nodes)"
    local _n
    for _n in $(seq 1 120); do
        local h
        h=$(kubectl get cephcluster my-cluster -n "${ROOK_NS}" -o jsonpath='{.status.ceph.health}' 2>/dev/null || true)
        if [ "$h" = "HEALTH_OK" ] || [ "$h" = "HEALTH_WARN" ]; then
            info "Ceph reports ${h}"
            break
        fi
        sleep 10
    done

    if ! kubectl get cephcluster my-cluster -n "${ROOK_NS}" &>/dev/null; then
        warn "CephCluster not found — check rook-ceph operator logs and node storage"
        return 0
    fi

    local _health
    _health=$(kubectl get cephcluster my-cluster -n "${ROOK_NS}" -o jsonpath='{.status.ceph.health}' 2>/dev/null || true)
    if ! echo "$_health" | grep -qE 'HEALTH_OK|HEALTH_WARN'; then
        warn "Ceph not healthy yet (${_health:-unknown}) — skipping RGW object store. Add free raw disks or fix OSDs, then apply ${SCRIPT_DIR}/rook-ceph-object-minimal.yaml and Rook object-user.yaml."
        return 0
    fi

    step "CephObjectStore (minimal, single-replica) + S3 user for Velero"
    kubectl apply -f "${SCRIPT_DIR}/rook-ceph-object-minimal.yaml"
    kubectl apply -f "${base}/object-user.yaml"

    step "waiting for RGW / object user secret"
    local _i
    for _i in $(seq 1 90); do
        if kubectl get secret -n "${ROOK_NS}" rook-ceph-object-user-my-store-my-user &>/dev/null; then
            info "Rook object user secret ready (Velero can use RGW endpoint)"
            return 0
        fi
        sleep 10
    done
    warn "Secret rook-ceph-object-user-my-store-my-user not ready — set Velero S3 keys manually"
}

install_prometheus() {
    case "${V9S_INSTALL_PROMETHEUS:-}" in 1|true|TRUE|yes|Yes) ;; *)
        info "skip Prometheus (not installed by default; set V9S_INSTALL_PROMETHEUS=1)"
        return 0
        ;;
    esac
    should_skip V9S_SKIP_PROMETHEUS && { info "skip Prometheus (V9S_SKIP_PROMETHEUS)"; return 0; }
    require_helm
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts 2>/dev/null || true
    helm repo update prometheus-community 2>/dev/null || helm repo update
    step "Prometheus server (Helm, namespace ${PROMETHEUS_NS})"
    helm upgrade --install "$PROMETHEUS_RELEASE" prometheus-community/prometheus \
        --namespace "$PROMETHEUS_NS" \
        --create-namespace \
        --set alertmanager.enabled=false \
        --set pushgateway.enabled=false \
        --set kubeStateMetrics.enabled=true \
        --set nodeExporter.enabled=true \
        --set server.retention=15d \
        --set server.persistentVolume.enabled=false \
        --set server.service.servicePort=9090 \
        --wait --timeout 10m
    info "Prometheus UI/API: http://${PROMETHEUS_RELEASE}-server.${PROMETHEUS_NS}.svc:9090"
}

install_metallb() {
    should_skip V9S_SKIP_METALLB && { info "skip MetalLB (V9S_SKIP_METALLB)"; return 0; }
    if ! should_install V9S_INSTALL_METALLB; then
        info "skip MetalLB (set V9S_INSTALL_METALLB=1)"
        return 0
    fi
    require_helm
    helm repo add metallb https://metallb.github.io/metallb 2>/dev/null || true
    helm repo update metallb 2>/dev/null || helm repo update
    step "MetalLB (Helm, namespace ${METALLB_NS})"
    helm upgrade --install "$METALLB_RELEASE" metallb/metallb \
        --namespace "$METALLB_NS" \
        --create-namespace \
        --wait --timeout 10m

    if [ -n "$METALLB_IP_POOL" ]; then
        step "MetalLB L2 IPAddressPool + L2Advertisement (pool: ${METALLB_POOL_NAME})"
        kubectl apply -f - <<YAML
apiVersion: metallb.io/v1beta1
kind: IPAddressPool
metadata:
  name: ${METALLB_POOL_NAME}
  namespace: ${METALLB_NS}
spec:
  addresses:
  - ${METALLB_IP_POOL}
---
apiVersion: metallb.io/v1beta1
kind: L2Advertisement
metadata:
  name: ${METALLB_L2_NAME}
  namespace: ${METALLB_NS}
spec:
  ipAddressPools:
  - ${METALLB_POOL_NAME}
YAML
        info "MetalLB pool configured (L2). IPs must be routable on your L2 segment."
    else
        warn "MetalLB controller installed but no pool configured. Set V9S_METALLB_IP_POOL (e.g. 192.168.1.240-192.168.1.250) and re-run this script, or apply IPAddressPool + L2Advertisement manually."
    fi
}

main() {
    preflight
    install_cilium
    ensure_cilium_pod_egress
    install_multus
    install_metrics_server
    install_snapshot_controller
    install_kubevirt
    install_cdi
    install_rook_ceph
    install_prometheus
    install_metallb
    echo ""
    info "Done. metrics-server: on by default (V9S_INSTALL_METRICS_SERVER=0 or V9S_SKIP_METRICS_SERVER=1 to skip). Optional: V9S_INSTALL_MULTUS, V9S_INSTALL_SNAPSHOT_CONTROLLER, V9S_INSTALL_ROOK_CEPH, V9S_INSTALL_METALLB, V9S_INSTALL_PROMETHEUS."
}

main "$@"
