<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Integrations

Veyron works on Kubernetes data alone. Each integration below upgrades part of the console to a
production backend. All of them are optional and configured through environment variables on the
API.

## How to configure

- **Automatic:** during `deploy-remote.sh`, `scripts/lib/bootstrap-integrations.sh` finds
  Prometheus, Grafana, Alertmanager, Argo CD, PacketWolf and Atlas Services in the cluster and
  writes their URLs into the `veyron-integrations` Secret. Skip it with
  `VEYRON_SKIP_INTEGRATIONS_BOOTSTRAP=1`.
- **Manual:** fill in
  [`deploy/k8s/optional-integrations.env.example.yaml`](../deploy/k8s/optional-integrations.env.example.yaml),
  apply it, then run `kubectl rollout restart deployment/veyron-api -n veyron-system`.
- **Helm:** set `integrations.*` in `charts/veyron/values.yaml`.

`GET /api/v1/integrations/status` reports what is connected.

## Reference

| Area | Variables | Unlocks |
|---|---|---|
| Metrics | `VEYRON_PROMETHEUS_URL` | Volume usage, forecasts, p95/p99 latency |
| Alerts | `VEYRON_ALERTMANAGER_URL` | Incident timeline |
| Logs | `VEYRON_LOKI_URL` | Cluster-wide log search |
| Traces | `VEYRON_JAEGER_QUERY_URL` or `VEYRON_TEMPO_QUERY_URL` | Trace views |
| Costs | `VEYRON_OPENCOST_URL`, `VEYRON_OPENCOST_TOKEN` | Real allocation costs (built-in reference rates otherwise) |
| Vulnerabilities | `VEYRON_TRIVY_URL` | CVE findings merged with VM posture checks |
| GitOps | `VEYRON_ARGOCD_URL`, `VEYRON_ARGOCD_TOKEN` | `POST /api/v1/gitops/sync`; Flux works through an annotation |
| Copilot | `VEYRON_AI_URL`, `VEYRON_AI_API_KEY`, `VEYRON_AI_MODE` | Natural-language fleet assistant on any OpenAI-compatible model |
| Notifications | `VEYRON_SLACK_WEBHOOK_URL`, `VEYRON_PAGERDUTY_ROUTING_KEY`, `VEYRON_EMAIL_RELAY_URL` | Alert delivery |
| SOC | `VEYRON_ELASTIC_*`, `VEYRON_SPLUNK_*`, `VEYRON_SENTINEL_*`, `VEYRON_QRADAR_*`, `VEYRON_SOAR_WEBHOOK_URL` | [soc.md](soc.md) |
| SSO | `VEYRON_OIDC_*` | [sso.md](sso.md) |
| Lab machines | `VEYRON_KRYTON_URL`, `VEYRON_KRYTON_TOKEN`, `VEYRON_KRYTON_PROJECT` | [Kryton](#kryton-lab-and-edge-machines-golden-images) machines and golden images |
| Browser links | `VEYRON_GRAFANA_EXTERNAL_URL`, `VEYRON_PROMETHEUS_EXTERNAL_URL`, `VEYRON_ALERTMANAGER_EXTERNAL_URL` | "Open console" buttons |

Monitoring stack in one command:

```bash
./scripts/install-veyron-monitoring.sh monitoring   # Prometheus, Grafana, Alertmanager
```

## Atlas: Ceph-backed disk protection

[Atlas](https://zyvor.dev) is the Zyvor storage control plane in front of Ceph (RBD, CephFS, RGW),
NFS and ZFS. With `VEYRON_ATLAS_URL` set, Veyron runs Ceph snapshots, clones, restores and
off-cluster backups (RBD `export-diff` to RGW/S3) for VM disks as Atlas jobs. This adds to the
native snapshot flow; it doesn't replace it.

| Variable | Notes |
|---|---|
| `VEYRON_ATLAS_URL` | Gateway, for example `http://atlas-gateway.zyvor:5110` |
| `VEYRON_ATLAS_TOKEN` | HS256 bearer, only when Atlas requires auth |
| `VEYRON_ATLAS_TENANT` | Tenant recorded on volumes and backups (default `global`) |

Routes live under `/api/v1/atlas/*`: status, volumes, the volume map for each VM, and
`ceph-snapshot`, `ceph-backup`, restore (admin), clone and jobs. Write routes return `202` with a
`job_id`. A disk maps to an Atlas volume by namespace and PVC name, so only disks on Ceph RBD are
actionable.

## Kryton: lab and edge machines, golden images

[Kryton](https://github.com/zyvorai/zyvor-kryton) is the Zyvor machine API: one REST contract for
Windows and Linux machines on `dockur` (Windows in containers on a lab host), `libvirt` (Linux
cloud images on a KVM host), `kubevirt` or the in-memory `demo` provider. With `VEYRON_KRYTON_URL`
set, Veyron proxies it under `/api/v1/kryton/*`, so one Veyron API key drives lab machines next to
cluster VMs.

| Variable | Notes |
|---|---|
| `VEYRON_KRYTON_URL` | Kryton base URL, for example `http://kryton.kryton-system:8080` |
| `VEYRON_KRYTON_TOKEN` | Kryton bearer API key (Kryton stores only its SHA-256 digest) |
| `VEYRON_KRYTON_PROJECT` | Optional project; Kryton's default project otherwise |

| Route | Does |
|---|---|
| `GET /kryton/status` | Configured, reachable, summary and provider capabilities |
| `GET /kryton/images` | Kryton catalog (Windows and Linux images, readiness) |
| `GET/POST /kryton/machines`, `GET/DELETE /kryton/machines/:id` | List, create, inspect, delete machines |
| `POST /kryton/machines/:id/{start,stop}` | Power actions |
| `GET/POST /kryton/machines/:id/snapshots` | Snapshots |
| `GET /kryton/golden`, `POST /kryton/golden/:id/bootstrap` | Golden builds; bootstrap publishes the CDI DataSource `kryton-images/<image-id>` |
| `GET /kryton/jobs/:id` | Poll async jobs |

Veyron's built-in templates follow the Kryton catalog. `rocky-10` and `rocky-9` clone the Kryton
golden DataSource `kryton-images/rocky-*`, because the public Rocky containerdisk needs
registry credentials. Bootstrap the image in Kryton before creating Rocky VMs.

## PacketWolf: network intelligence

With `VEYRON_PACKETWOLF_URL` set, Veyron shows PacketWolf health and proxies its network overview
and flows. PacketWolf uses JWT login: set `VEYRON_PACKETWOLF_USERNAME` and
`VEYRON_PACKETWOLF_PASSWORD`. Veyron caches the token and refreshes it before it expires.
`VEYRON_PACKETWOLF_API_KEY` exists only for older deployments that accept static keys. The
`/status` health probe is unauthenticated, so "reachable" alone doesn't prove the login works.
