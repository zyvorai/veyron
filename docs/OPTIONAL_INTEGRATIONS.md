# Optional integrations (production parity)

VMRogue API handlers work without external services using Kubernetes data and heuristics. Set the environment variables below on the **API Deployment** (`vmrogue-system`) to upgrade specific dashboard pages to production-grade backends.

**See also:** [FEATURE_MATRIX.md](FEATURE_MATRIX.md), [charts/vmrogue-monitoring/README.md](../charts/vmrogue-monitoring/README.md)

## Quick apply

**Automatic (remote deploy):** [`scripts/lib/bootstrap-integrations.sh`](../scripts/lib/bootstrap-integrations.sh) runs from [`scripts/deploy-all-remote.sh`](../scripts/deploy-all-remote.sh) when observability, Argo CD, or PacketWolf services exist. It can set Prometheus/Grafana/Alertmanager URLs, `VMROGUE_ARGOCD_URL` (+ session token from `argocd-initial-admin-secret`), and **`VMROGUE_PACKETWOLF_URL`** when `packetwolf-api` is found in `cilium-system` or `packetwolf` (optional `VMROGUE_PACKETWOLF_EXTERNAL_URL` from `packetwolf-ui` NodePort). Set `VMROGUE_SKIP_INTEGRATIONS_BOOTSTRAP=1` to skip all wiring, or `VMROGUE_SKIP_ARGOCD_TOKEN_BOOTSTRAP=1` for Argo URL-only.

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
| `VMROGUE_GRAFANA_EXTERNAL_URL` | Integrations **Open console** (browser) | `http://HOST:30080` |
| `VMROGUE_PROMETHEUS_EXTERNAL_URL` | Prometheus UI (browser) | `http://HOST:30090` |
| `VMROGUE_ALERTMANAGER_EXTERNAL_URL` | Alertmanager UI (browser) | `http://HOST:30093` |
| `VMROGUE_JAEGER_EXTERNAL_URL` | Jaeger UI (browser) | `http://HOST:16686` |
| `VMROGUE_ALERTMANAGER_URL` | Incidents timeline | `http://alertmanager:9093` |
| `VMROGUE_ARGOCD_URL` | GitOps sync (POST) | `https://argocd-server.argocd` |
| `VMROGUE_ARGOCD_TOKEN` | Argo CD API token | (secret) |
| `VMROGUE_EMAIL_RELAY_URL` | Notification email delivery | HTTP relay |
| `VMROGUE_AI_URL` | Copilot LLM backend (OpenAI-compatible) | `https://api.openai.com/v1` |
| `VMROGUE_AI_API_KEY` | LLM API key | (secret) |
| `VMROGUE_AI_MODE` | `off` / `paraphrase` / `routing` / `agent` | `routing` |
| `VMROGUE_SLACK_WEBHOOK_URL` | Slack notifications | incoming webhook |
| `VMROGUE_PAGERDUTY_ROUTING_KEY` | PagerDuty events | routing key |
| `VMROGUE_ELASTIC_URL` | SOC SIEM push + Elastic hunts | `https://elastic:9200` |
| `VMROGUE_ELASTIC_API_KEY` | Elastic auth | (secret) |
| `VMROGUE_ELASTIC_INDEX` | Elastic index (optional) | `vmrogue-security` |
| `VMROGUE_ELASTIC_HUNT_ENABLED` | Dashboard/API KQL hunts | `true` |
| `VMROGUE_SPLUNK_HEC_URL` | Splunk HEC push | `https://splunk:8088/services/collector/event` |
| `VMROGUE_SPLUNK_HEC_TOKEN` | Splunk HEC token | (secret) |
| `VMROGUE_SPLUNK_HUNT_ENABLED` | Splunk SPL hunts | `true` |
| `VMROGUE_SENTINEL_DCE_URL` | Microsoft Sentinel DCE | Log Analytics DCE endpoint |
| `VMROGUE_SENTINEL_TENANT_ID` | Sentinel OAuth | tenant UUID |
| `VMROGUE_SENTINEL_CLIENT_ID` | Sentinel OAuth | app client id |
| `VMROGUE_SENTINEL_CLIENT_SECRET` | Sentinel OAuth | (secret) |
| `VMROGUE_QRADAR_HOST` | QRadar LEEF UDP | `qradar.example` |
| `VMROGUE_QRADAR_PORT` | QRadar UDP port | `514` |
| `VMROGUE_SOAR_WEBHOOK_URL` | SOAR on new SOC detections | HTTPS webhook |
| `VMROGUE_PACKETWOLF_URL` | PacketWolf Network Brain health (`GET /api/v1/packetwolf/status`), Cilium page banner | `http://packetwolf-api.cilium-system.svc:9191` |
| `VMROGUE_PACKETWOLF_EXTERNAL_URL` | Integrations **Open PacketWolf UI** (browser) | `http://HOST:30808` |
| `VMROGUE_PACKETWOLF_API_KEY` | PacketWolf API auth (optional when in-cluster trust is enabled) | (secret) |

Responses include `vmrogue_context` describing the active data source when integrations are used or skipped.

**Dashboard:** `GET /api/v1/integrations/status` lists configured `VMROGUE_*` backends (redacted host, HTTP probe) and **Open console** links when `VMROGUE_*_EXTERNAL_URL` is set or a NodePort is auto-discovered (`scripts/lib/bootstrap-integrations.sh`).

## Monitoring stack

Install the optional umbrella chart for Prometheus, Grafana, and Alertmanager:

```bash
./scripts/install-vmrogue-monitoring.sh monitoring
```

Then point `VMROGUE_PROMETHEUS_URL` at the in-cluster Prometheus query API and `VMROGUE_ALERTMANAGER_URL` at Alertmanager. Optional Grafana NodePort (`charts/vmrogue-monitoring` sets port **30080** by default) is wired to `VMROGUE_GRAFANA_EXTERNAL_URL` on deploy.

## FinOps (OpenCost)

Deploy [OpenCost](https://www.opencost.io/) in the cluster, expose the allocation API, and set `VMROGUE_OPENCOST_URL`. Without it, costs use internal reference rates (see `feature_context::costs_list`).

## Security (Trivy)

Deploy a Trivy operator or scanner HTTP API compatible with VMRogue's client (`src/security/scan.rs`). Set `VMROGUE_TRIVY_URL` to merge CVE rows with VM-spec posture checks.

## SOC / SIEM

The **SOC** dashboard page (`Security → SOC`) uses `src/soc/` for normalized events, built-in detections, optional push to Elastic (ECS), Splunk (HEC), Sentinel (DCE), and QRadar (LEEF), plus read-only hunts when `VMROGUE_*_HUNT_ENABLED=true`. See [SOC.md](SOC.md).

Configure variables in [optional-integrations.env.example.yaml](../deploy/k8s/optional-integrations.env.example.yaml) and verify with:

```bash
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" https://HOST:30151/api/v1/soc/export/status
```

**OpenRouter / Ask Zeus:** `./scripts/configure-zeus-openrouter.sh HOST USER` reads `~/.zshrc` and patches `vmrogue-integrations` — see [VMROGUE_AI.md](VMROGUE_AI.md).

## GitOps controllers

- **Argo CD:** `VMROGUE_ARGOCD_URL` + `VMROGUE_ARGOCD_TOKEN` — `POST /api/v1/gitops/sync` triggers application refresh when `application` is set in the GitOps ConfigMap.
- **Flux:** set `flux_kustomization` in the GitOps ConfigMap; sync adds the reconcile annotation (no extra env).

## PacketWolf (Network Brain)

When [PacketWolf](https://zyvor.dev) runs in the same cluster (typical Helm release: `packetwolf-api` in `cilium-system`), VMRogue can probe its health and surface status on the **Integrations** and **Cilium** dashboard pages.

**Automatic wiring:** `bootstrap-integrations.sh` sets `VMROGUE_PACKETWOLF_URL` to `http://packetwolf-api.<ns>.svc:9191` when the Service exists. Restart `vmrogue-api` after the Secret is applied (deploy script does this on rollout).

**Manual verify:**

```bash
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" \
  https://HOST:30151/api/v1/packetwolf/status
curl -sk -H "X-API-Key: $VMROGUE_API_KEY" \
  https://HOST:30151/api/v1/integrations/status | grep packetwolf
```

**In-cluster trust:** when PacketWolf is deployed with `PACKETWOLF_TRUST_CLUSTER_NETWORKS=true` (default), the API pod can reach PacketWolf without `VMROGUE_PACKETWOLF_API_KEY`. Set the key only for external or authenticated endpoints.

VMRogue does not proxy the full PacketWolf UI/API (unlike v9s Zeus OS); use **Open PacketWolf** on the Cilium page or the external URL for the Network Brain console. Live Hubble flows in the Cilium page remain policy-derived; PacketWolf provides the production Network Brain layer.

## OIDC / SSO

Enterprise login uses `VMROGUE_OIDC_*` — see [OIDC_SSO.md](OIDC_SSO.md). Separate from observability integrations above.
