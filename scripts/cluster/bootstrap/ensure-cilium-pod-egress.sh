#!/usr/bin/env bash
# Idempotent: ensure Cilium SNAT/masquerade works so pods (and KubeVirt guests) reach the internet.
# Fixes clusters where CILIUM_POST_nat is empty and packets leave with 10.0.0.x source IPs.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VALUES="${ROOT_DIR}/k8s/cilium-k3s-values.yaml"

command -v kubectl >/dev/null 2>&1 || exit 0
command -v helm >/dev/null 2>&1 || exit 0

if ! kubectl get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
  echo "ensure-cilium-pod-egress: Cilium not installed — skip"
  exit 0
fi

if ! kubectl get ds cilium -n kube-system &>/dev/null; then
  echo "ensure-cilium-pod-egress: no cilium DaemonSet — skip"
  exit 0
fi

CHART_VERSION="${V9S_CILIUM_CHART_VERSION:-1.19.4}"
helm repo add cilium https://helm.cilium.io/ 2>/dev/null || true
helm repo update cilium 2>/dev/null || helm repo update 2>/dev/null || true

echo "ensure-cilium-pod-egress: aligning Cilium routing/masquerade (chart ${CHART_VERSION})"
helm upgrade --install cilium cilium/cilium \
  --version "$CHART_VERSION" \
  --namespace kube-system \
  -f "$VALUES" \
  --reuse-values \
  --wait --timeout 10m

kubectl rollout restart daemonset/cilium -n kube-system
kubectl rollout status daemonset/cilium -n kube-system --timeout=5m

# Quick smoke: pod egress (self-signed / busy test)
if kubectl run cilium-egress-smoke --rm -i --restart=Never \
  --image=curlimages/curl --timeout=45s \
  --command -- curl -sS -m10 -o /dev/null -w '%{http_code}' https://1.1.1.1 2>/dev/null | grep -qE '301|200|302'; then
  echo "ensure-cilium-pod-egress: pod egress OK"
else
  echo "ensure-cilium-pod-egress: WARNING pod egress test failed — check routing/NAT on the node" >&2
  exit 1
fi
