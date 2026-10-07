<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Integrations

Veyron works on Kubernetes data alone. Each integration below upgrades part of the console to a
production backend. All of them are optional and configured through environment variables on the
API.

## How to configure

- **Automatic:** during `deploy-remote.sh`, `scripts/lib/bootstrap-integrations.sh` finds
  Prometheus, Grafana, Alertmanager, Argo CD, Netra, Paqtra and Atlas Services in the cluster and
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

## Netra: network observability

[Netra](https://github.com/zyvorai/netra) is Zyvor's eBPF network observability product. Veyron
shows its health, a flow summary (verdicts, protocols, drop reasons, top destinations) and Netra's
per-VM network view, including which VMs are locked down, on the console's **Netra** page.

There is nothing to configure when Netra runs in the same cluster: Veyron finds the `netra` Service
in `netra-system` and talks to `https://netra.netra-system.svc:30870`.

| Variable | Default | Purpose |
|---|---|---|
| `VEYRON_NETRA_URL` | auto-detected | Netra controller URL when it runs outside `netra-system` |
| `VEYRON_NETRA_API_KEY` | `Admin@321` | Netra's `NETRA_API_KEY`, sent as `Authorization: Bearer` |
| `VEYRON_NETRA_EXTERNAL_URL` | NodePort, set by bootstrap | Browser link for **Open Netra** |
| `VEYRON_NETRA_TLS_VERIFY` | off | `1` verifies Netra's certificate (it is self-signed by default) |

The key default matches Netra's own lab default, so a fresh Netra install works without setup. If
you rotate `NETRA_API_KEY`, set `VEYRON_NETRA_API_KEY` to the same value; otherwise the status shows
"Unauthorized".

| Route | Netra endpoint |
|---|---|
| `GET /api/v1/netra/status` | `/healthz` and `/api/v1/status` (version, agents, mode, flow rate) |
| `GET /api/v1/netra/flows/summary?namespace=&number=` | `/api/v1/flows/summary` |
| `GET /api/v1/netra/vms?namespace=` | `/api/v1/vms` |

## Paqtra: Cilium flow history

[Paqtra](https://github.com/zyvorai/zyvor-paqtra) is Zyvor's Cilium-native observability product.
It stores every Hubble flow and explains Cilium drops. Veyron's **Paqtra** console page shows
Paqtra's health and Hubble ingest, the Cilium agents, the network policy posture, explained drops,
and recent flows (optionally only dropped ones).

When Paqtra runs in the same cluster with its default release name, there is nothing to configure.
Veyron finds the `paqtra-api` Service in the `paqtra` namespace and talks to
`http://paqtra-api.paqtra.svc:9191`.

Paqtra signs callers in with a 24-hour JWT from `POST /api/v1/auth/login`. Its admin credentials are
generated randomly on install, so Veyron reads them from Paqtra's own `paqtra/paqtra-secret`. It
uses `ADMIN_USERNAME` and `API_KEY`, which Paqtra accepts as the admin password, and falls back to
`ADMIN_PASSWORD`. The token is cached and refreshed before it expires. Veyron's `ClusterRole`
already grants `get` on Secrets. If you trim that, set the credentials explicitly.

| Variable | Default | Purpose |
|---|---|---|
| `VEYRON_PAQTRA_URL` | auto-detected | Paqtra API URL when it runs elsewhere |
| `VEYRON_PAQTRA_USERNAME` | `admin` | Paqtra account to sign in with |
| `VEYRON_PAQTRA_PASSWORD` | read from `paqtra-secret` | That account's password or Paqtra's `API_KEY` |
| `VEYRON_PAQTRA_EXTERNAL_URL` | `paqtra-ui` NodePort, set by bootstrap | Browser link for **Open Paqtra** |
| `VEYRON_PAQTRA_TLS_VERIFY` | off | `1` verifies the certificate when `VEYRON_PAQTRA_URL` is `https` |

For least privilege, create a read-only Paqtra user and set `VEYRON_PAQTRA_USERNAME` and
`VEYRON_PAQTRA_PASSWORD` instead of relying on the admin key. Veyron only calls read endpoints.

| Route | Paqtra endpoint |
|---|---|
| `GET /api/v1/paqtra/status` | `/health`, `/api/v1/cilium/status`, `/api/v1/cluster/health` |
| `GET /api/v1/paqtra/flows?namespace=&verdict=&limit=` | `/api/v1/flows` (`limit` 1–500, default 100) |
| `GET /api/v1/paqtra/drops` | `/api/v1/modules/rootcause/drops` |

## Gryvia: GPU platform

[Gryvia](https://github.com/zyvorai/gryvia) is Zyvor's Kubernetes GPU platform for containerized
training and inference: GPU-aware job admission, tenants, quotas, budgets and GPU-hour metering.
Veyron runs GPU VMs; Gryvia runs GPU jobs. The console's **Gryvia** page (under Compute) shows
Gryvia's health, container GPU capacity and job counts, its registered GPU nodes, tenants, and
GPU-hour usage by tenant.

When Gryvia runs in the same cluster there is nothing to configure. Veyron finds the
`gryvia-api-gateway` Service in `gryvia-system`, talks to
`https://gryvia-api-gateway.gryvia-system.svc:8080`, and reads the bearer key from Gryvia's own
`gryvia-system/gryvia-api-key` Secret (key `GRYVIA_API_KEY`).

| Variable | Default | Purpose |
|---|---|---|
| `VEYRON_GRYVIA_URL` | auto-detected | Gryvia api-gateway URL when it runs elsewhere |
| `VEYRON_GRYVIA_API_KEY` | read from `gryvia-api-key` | Gryvia's `GRYVIA_API_KEY`, sent as `Authorization: Bearer` |
| `VEYRON_GRYVIA_EXTERNAL_URL` | `gryvia-ui` NodePort | Browser link for **Open Gryvia** |
| `VEYRON_GRYVIA_TLS_VERIFY` | off | `1` verifies Gryvia's certificate (it is self-signed by default) |

Gryvia's key is an admin key, so every Veyron user with read access sees Gryvia's cluster-wide
GPU nodes, tenants and usage through these routes. Veyron only calls read endpoints.

| Route | Gryvia endpoint |
|---|---|
| `GET /api/v1/gryvia/status` | `/health` and `/api/cluster/stats` (GPUs, utilization, jobs) |
| `GET /api/v1/gryvia/nodes` | `/api/nodes` |
| `GET /api/v1/gryvia/tenants` | `/api/tenants` |
| `GET /api/v1/gryvia/quotas` | `/api/quotas` |
| `GET /api/v1/gryvia/usage?tenant=&from=&to=&group_by=` | `/api/usage` (`group_by` tenant, sku or day) |

**Sharing GPUs between VMs and jobs.** Passthrough VMs need GPUs bound to `vfio-pci`; Gryvia's jobs
need them on the NVIDIA driver. With the NVIDIA GPU Operator's sandbox workloads enabled, each node's
`nvidia.com/gpu.workload.config` label says which side owns its GPUs. Veyron honors it: GPUs on a
`container` node are reported with `vm_usable: false` in `GET /api/v1/gpus`, and `POST /vms` with a
GPU returns `422 GPU_RESERVED_FOR_CONTAINERS` when the requested resource only exists on such nodes.
Label a node `vm-passthrough` (or `vm-vgpu`) to give its GPUs to VMs. See [gpu.md](gpu.md).
| `GET /api/v1/paqtra/posture` | `/api/v1/security/posture` |
