#!/usr/bin/env bash
# Install optional Veyron monitoring bundle (Prometheus Operator + Prometheus +
# Grafana + Alertmanager + node-exporter + kube-state-metrics) via Helm.
#
# Usage:
#   ./scripts/install-veyron-monitoring.sh [namespace] [release_name]
#
# Examples:
#   ./scripts/install-veyron-monitoring.sh monitoring
#   GRAFANA_ADMIN_PASSWORD='your-secret' ./scripts/install-veyron-monitoring.sh monitoring veyron-mon
#
# Requires: helm 3.9+, kubectl, cluster with default StorageClass (for Prometheus PVCs).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CHART="${ROOT}/charts/veyron-monitoring"

NS="${1:-monitoring}"
REL="${2:-veyron-monitoring}"
# Never ship a guessable default. Use the provided password, or generate a random
# one and print it once (so a forgotten override can't silently install "changeme").
if [ -n "${GRAFANA_ADMIN_PASSWORD:-}" ]; then
  GRAFANA_PASSWORD="${GRAFANA_ADMIN_PASSWORD}"
else
  GRAFANA_PASSWORD="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 24)"
  echo "⚠️  GRAFANA_ADMIN_PASSWORD not set — generated a random Grafana admin password:"
  echo "    ${GRAFANA_PASSWORD}"
  echo "    (set GRAFANA_ADMIN_PASSWORD to choose your own; store this somewhere safe)"
fi

if ! command -v helm >/dev/null 2>&1; then
  echo "helm is required (https://helm.sh)" >&2
  exit 1
fi

echo "==> helm dependency build (${CHART})"
helm dependency build "${CHART}"

echo "==> Installing ${REL} into namespace ${NS}"
kubectl get ns "${NS}" >/dev/null 2>&1 || kubectl create ns "${NS}"

helm upgrade --install "${REL}" "${CHART}" \
  --namespace "${NS}" \
  --set-string kps.grafana.adminPassword="${GRAFANA_PASSWORD}" \
  --wait --timeout 20m

echo ""
echo "Done. Grafana admin user: admin"
echo "Port-forward: kubectl -n ${NS} port-forward svc/${REL}-grafana 3000:3000"
