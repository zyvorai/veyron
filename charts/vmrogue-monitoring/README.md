# vmrogue-monitoring

Optional **Prometheus + Grafana + Alertmanager** stack for VMRogue clusters, packaged as a thin Helm umbrella over [kube-prometheus-stack](https://github.com/prometheus-community/helm-charts/tree/main/charts/kube-prometheus-stack).

## What you get

- Prometheus Operator, Prometheus, Alertmanager, Grafana (upstream chart defaults tuned for broad `ServiceMonitor` / `PrometheusRule` discovery).
- **VMRogue ServiceMonitors** targeting the API (`/api/v1/health` over HTTPS) and operator (`/metrics`) in `vmrogue-system` (configurable).
- **VMRogue PrometheusRules** (KubeVirt VM and cluster hints; requires `kubevirt_*` metrics in Prometheus).
- **Grafana dashboard** ConfigMap (`VMRogue overview`) loaded by the Grafana sidecar.

## Ports

- **Grafana** ClusterIP HTTP is **3000** (not 80) so `kubectl port-forward … 3000:3000` avoids binding to remote port 80.
- **VMRogue operator** metrics Service ports are **9280** / **9281** (pod containers still use 8080 / 8081); `ServiceMonitor` continues to use port **names** `metrics` and `health`.

## Install

From the repository root:

```bash
./scripts/install-vmrogue-monitoring.sh monitoring
```

Or manually:

```bash
cd charts/vmrogue-monitoring
helm dependency build .
helm install vmrogue-monitoring . -n monitoring --create-namespace \
  --set kps.grafana.adminPassword='replace-me'
```

Override the VMRogue install namespace if your API runs elsewhere:

```bash
helm upgrade --install vmrogue-monitoring ./charts/vmrogue-monitoring -n monitoring \
  --set vmrogue.namespace=my-vmrogue-ns \
  --set kps.grafana.adminPassword='replace-me'
```

## KubeVirt metrics

Panels and alerts assume KubeVirt exposes Prometheus series (for example `kubevirt_vmi_info`). Follow KubeVirt monitoring guidance so those targets are scraped; otherwise queries stay empty while cluster-level metrics still work.

## CI / packaging

Helm dependency tarballs under `charts/` are gitignored; CI and fresh clones should run `helm dependency build charts/vmrogue-monitoring` (the lockfile `Chart.lock` pins the dependency version).

## Disable only the upstream stack

```bash
helm upgrade vmrogue-monitoring ./charts/vmrogue-monitoring -n monitoring --set kps.enabled=false
```
