# Optional integrations (production parity)

VMRogue API handlers work without external services using Kubernetes data and heuristics. Set the environment variables below on the **API Deployment** (`vmrogue-system`) to upgrade specific dashboard pages to production-grade backends.

**See also:** [FEATURE_MATRIX.md](FEATURE_MATRIX.md), [charts/vmrogue-monitoring/README.md](../charts/vmrogue-monitoring/README.md)

## Quick apply

**Automatic (remote deploy):** [`scripts/lib/bootstrap-integrations.sh`](../scripts/lib/bootstrap-integrations.sh) runs from [`scripts/deploy-all-remote.sh`](../scripts/deploy-all-remote.sh) when `monitoring` or `loki` namespaces exist. Set `VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP=1` to skip.

**Manual:**

1. Copy [deploy/k8s/optional-integrations.env.example.yaml](../deploy/k8s/optional-integrations.env.example.yaml) and fill in cluster-internal URLs.
2. Apply the Secret, then restart the API:

```bash
kubectl apply -f deploy/k8s/optional-integrations.env.example.yaml
kubectl rollout restart deployment/vmrogue-api -n vmrogue-system
```

With Helm, set `integrations.*` in [charts/vmrogue/values.yaml](../charts/vmrogue/values.yaml) and upgrade the release.

## Variable reference

| Variable | Affects (API / dashboard) | Example value |
|----------|---------------------------|---------------|
| `VMROGUE_PROMETHEUS_URL` | Costs (volume usage), forecasting, performance p95/p99, scheduling latency | `http://prometheus-kube-prometheus-prometheus.monitoring:9090/api/v1/query` |
| `VMROGUE_OPENCOST_URL` | Costs list/summary/forecast | `http://opencost.opencost:9003` |
| `VMROGUE_OPENCOST_TOKEN` | OpenCost auth (optional) | bearer token |
| `VMROGUE_TRIVY_URL` | Security findings (CVE merge) | `http://trivy.trivy:4954` |
| `VMROGUE_LOKI_URL` | Logs page (cluster-wide LogQL) | `http://loki.loki:3100` |
| `VMROGUE_JAEGER_QUERY_URL` | Traces | `http://jaeger-query:16686/api/traces` |
| `VMROGUE_TEMPO_QUERY_URL` | Traces (alternative) | `http://tempo:3200` |
| `VMROGUE_ALERTMANAGER_URL` | Incidents timeline | `http://alertmanager:9093` |
| `VMROGUE_ARGOCD_URL` | GitOps sync (POST) | `https://argocd-server.argocd` |
| `VMROGUE_ARGOCD_TOKEN` | Argo CD API token | (secret) |
| `VMROGUE_EMAIL_RELAY_URL` | Notification email delivery | HTTP relay |
| `VMROGUE_SLACK_WEBHOOK_URL` | Slack notifications | incoming webhook |
| `VMROGUE_PAGERDUTY_ROUTING_KEY` | PagerDuty events | routing key |

Responses include `vmrogue_context` describing the active data source when integrations are used or skipped.

**Operator UI:** `GET /api/v1/integrations/status` lists configured `VMROGUE_*` backends (redacted host, HTTP probe). Open **Integrations** in `/dashboard-next/`.

## Monitoring stack

Install the optional umbrella chart for Prometheus, Grafana, and Alertmanager:

```bash
./scripts/install-vmrogue-monitoring.sh monitoring
```

Then point `VMROGUE_PROMETHEUS_URL` at the in-cluster Prometheus query API and `VMROGUE_ALERTMANAGER_URL` at Alertmanager.

## FinOps (OpenCost)

Deploy [OpenCost](https://www.opencost.io/) in the cluster, expose the allocation API, and set `VMROGUE_OPENCOST_URL`. Without it, costs use internal reference rates (see `feature_context::costs_list`).

## Security (Trivy)

Deploy a Trivy operator or scanner HTTP API compatible with VMRogue's client (`src/security/scan.rs`). Set `VMROGUE_TRIVY_URL` to merge CVE rows with VM-spec posture checks.

## GitOps controllers

- **Argo CD:** `VMROGUE_ARGOCD_URL` + `VMROGUE_ARGOCD_TOKEN` — `POST /api/v1/gitops/sync` triggers application refresh when `application` is set in the GitOps ConfigMap.
- **Flux:** set `flux_kustomization` in the GitOps ConfigMap; sync adds the reconcile annotation (no extra env).

## OIDC / SSO

Enterprise login uses `VMROGUE_OIDC_*` — see [OIDC_SSO.md](OIDC_SSO.md). Separate from observability integrations above.
