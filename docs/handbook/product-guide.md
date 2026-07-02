# Veyron Product Guide

> Part of the [Veyron Handbook](README.md) · see also
> [Administration & Configuration](admin-configuration.md) ·
> [FAQ](faq.md) · [Troubleshooting](troubleshooting.md)

Veyron is a **Kubernetes-native VM command center for KubeVirt**, written in Rust.
It gives operators, GitOps engineers, and developers a single, opinionated surface
over the KubeVirt `VirtualMachine` lifecycle — replacing hand-written YAML and
`virtctl` juggling with templates, blueprints, browser consoles, migrations,
snapshots, GitOps export, and policy enforcement.

Within the Zyvor stack, Veyron is the **KubeVirt VM command center** — the control
tier for VMs running on Kubernetes, complementing `machina` (bare-metal hypervisor
OS), `zeus-os`/`v9s` (cloud/KubeVirt control plane), `guestkit` (offline VM
migration assurance, vendored as a submodule), and `packetwolf` (network
intelligence).

---

## 1. Core concepts

| Concept | What it is |
|---------|-----------|
| **VM** | A KubeVirt `VirtualMachine` (and its running `VirtualMachineInstance`). Veyron creates, mutates, and inspects these via `kube-rs`. |
| **Template** | A named OS image + defaults (44 OS templates per the README, e.g. `ubuntu-22.04`). Used with `veyron create --template`. See [OS templates](../OS_TEMPLATES.md) and [template catalog](../TEMPLATE_CATALOG.md). |
| **Resource profile** | A named CPU/memory/disk sizing preset (8 profiles). Managed via `veyron profile*`. |
| **Blueprint** | A multi-VM stack definition (LAMP, Kubernetes, 3-tier, CI/CD). Deployed with `veyron blueprint deploy` / `veyron deploy`. |
| **Snapshot** | A KubeVirt `VirtualMachineSnapshot`, optionally scheduled via a ConfigMap-driven scheduler. See [Snapshots](../SNAPSHOTS.md). |
| **Migration** | A live `VirtualMachineInstanceMigration` between nodes. |
| **CRD (`veyron.io`)** | Operator-reconciled custom resources: `VeyronVM`, `VeyronPolicy`, `VeyronBlueprint`, `VMTemplate`, `VMProfile`, `VeyronAction`, `VeyronInsight`. |
| **GuestKit** | Vendored guest-inspection / migration-assurance library used for guest-agent operations (freeze, filesystem, migrate-score). |

## 2. Surfaces

Veyron exposes four surfaces, all backed by the same Rust core.

### 2.1 CLI (`veyron`)

The `veyron` binary (`src/main.rs` → `src/cli/mod.rs`) is a `clap`-derived CLI with
200+ subcommands. Global flags include `--namespace/-n` and config resolution.
Representative command groups:

| Area | Commands (examples) |
|------|--------------------|
| Lifecycle | `create`, `list`/`ls`, `get`/`describe`, `delete`/`rm`, `start`, `stop`, `restart`, `pause`, `unpause`, `clone` |
| Guest agent | `guest-freeze`, `guest-unfreeze`, `guest-softreboot`, `ssh`, `ssh-key-inject` |
| Console | `console`, `vnc`, `logs`/`log` |
| Storage / disks | `resize`, `volume-add`, `volume-remove`, `disk-expand`, `disk-health`, `disk-usage` |
| Templates / catalog | `templates`, `template`, `catalog`, `generate`/`gen`, `validate`, `wizard`/`wiz` |
| Profiles | `profiles`, `profile`, `profile-create`, `profile-edit`, `profile-delete` |
| Blueprints | `blueprints`, `blueprint`, `blueprint-create/-edit/-delete/-validate`, `deploy` |
| Snapshots | `snapshot-create`, `snapshot-list`, `snapshot-get`, `snapshot-delete`, `snapshot-restore` |
| Monitoring | `monitor-live`, `monitor-stats`, `monitor-compare`, `monitor-top`, `resources`/`top`, `status`/`stat` |
| Network | `network-list`, `network-get`, `network-bandwidth`, `network-traffic`, `network-policies`, `network-policy` |
| Migration / HA | `migrate`, `migration-status`, `migration-list`, `ha-config`, `ha-status`, `evacuate-node`, `evacuation-status` |
| Backup / DR | `backup-create/-list/-get/-delete/-restore/-verify`, `backup-schedules`, `velero-dr` |
| GitOps | `gitops-export`, `import`, `export` |
| Ops & insight | `doctor`, `cost`, `security`, `compliance`, `observability`, `cilium`, `nodes`, `drift`, `alerts`, `slo`, `quotas`, `forecast`, `recommend` |
| API / server | `api-serve`, `api-status`, `api-routes`, `api-spec`, `api-key-list/-create/-delete`, `webhook-list/-create/-delete`, `event-list`, `event-recent` |
| Config / TUI | `config-show`, `config-init`, `tui`, `info` |

```bash
# Create a VM from a template
veyron create --template ubuntu-22.04 --name web-01 -n dev

# Deploy a multi-VM blueprint stack
veyron blueprint deploy lamp --namespace dev

# Open a serial console / VNC
veyron console web-01 -n dev
veyron vnc web-01 -n dev

# Export live VMs as GitOps manifests
veyron gitops-export --namespace production -o manifests/
```

> Note: the README's marketing shorthand `veyron serve` corresponds to the actual
> subcommand **`veyron api-serve`** (there is no `serve` alias in the CLI).

### 2.2 TUI

`veyron tui` launches a `ratatui`/`crossterm` full-screen terminal UI (flags:
`--no-splash`, `--theme`, `--basic`). It provides an interactive, auto-refreshing
view of VMs, nodes, and status. See [Interactive TUI](../INTERACTIVE_TUI.md) and
[TUI features demo](../TUI_FEATURES_DEMO.md).

### 2.3 REST + WebSocket API

Enabled by the default `web` Cargo feature and served by `veyron api-serve`
(axum). Routes are versioned under `/api/v1` (with a `/api/v2` and product-alias
`/api/v1/veyron` surface). The OpenAPI spec is embedded and served at
`/api/openapi.json` (also `veyron api-spec`).

**Selected routes** (see `veyron api-routes` for the live list):

| Method(s) | Path | Purpose |
|-----------|------|---------|
| GET | `/api/v1/health` | Health probe |
| GET | `/api/v1/vms`, `/api/v1/vms/:ns/:name` | List / get VMs |
| POST | `/api/v1/vms/:ns/:name/start`\|`stop`\|`restart`\|`pause`\|`unpause` | Power ops |
| POST | `/api/v1/vms/:ns/:name/migrate`, GET `/migrations` | Live migration |
| POST | `/api/v1/vms/:ns/:name/clone`, `/expose`, `/rdp-expose` | Clone / expose services |
| * | `/api/v1/vms/:ns/:name/volumes/hotplug`\|`hotremove`\|`status` | Hot-plug disks |
| * | `/api/v1/vms/:ns/:name/guest/status`\|`exec`\|`freeze`\|`doctor`\|`fix-plan`\|`migrate-score` | GuestKit guest-agent ops |
| GET | `/api/v1/vms/:ns/:name/vnc`, `/console/serial`, `/serial` | WebSocket consoles |
| GET/POST | `/api/v1/snapshots`, `/api/v1/snapshots/:ns/:vm/create`, `/restore`, `/delete` | Snapshots |
| GET/POST | `/api/v1/snapshot-schedules` | Scheduled snapshots |
| GET | `/api/v1/nodes`, `/pods`, `/namespaces`, `/events`, `/activity` | Cluster inventory |
| GET/PATCH | `/api/v1/storage/pvcs`, `/storage/classes` | Storage; PVC resize via `PATCH .../pvcs/:ns/:name` |
| GET | `/api/v1/dashboard/overview`, `/templates`, `/profiles` | Dashboard data |
| GET | `/api/v1/auth/oidc/config`, POST `/auth/oidc/token` | OIDC/SSO |
| GET | `/api/v1/ws/ticket`, `/ws/metrics` | WS auth tickets + live metrics |
| GET | `/dashboard` (and `/` redirect) | Web dashboard shell |

WebSocket consoles (`vnc`, `serial`, `rdp`) and the metrics stream authenticate via
a short-lived **ticket** obtained from `/api/v1/ws/ticket` and passed as
`?ticket=` (see [auth](admin-configuration.md#authentication)).

### 2.4 Web dashboard ("Mission Control")

The API serves a single-page dashboard at `/dashboard` (root `/` redirects there).
It ships **40+ pages**, including: Dashboard, VMs (with card / table / topology /
map / pulse views), Snapshots, Nodes, Pods, Storage, Events, CRDs, Blueprint
Studio, Policies, Insights, Actions, Costs, Security, Monitoring, Workloads,
Alerts, Audit, SOC, Notifications, Helm, Operators, Custom Resources, SLO, Chaos,
RBAC, Quotas, Ingress, HPA, Backups, Catalog, App Store, Console Hub, Stack Health,
VM Capsule, Network Intel, Mission Control, Topology, Dependencies, Autoscaler,
Forecasting, GitOps, **Ask Zeus** (AI), Scheduling, Cilium, Observability,
Performance, Webhooks, Compliance, DR, Heatmap, Custom Dashboards, Integrations,
Metrics, and Settings.

Cache-bust after a UI redeploy with `https://HOST:30151/dashboard?dash=<rev>` (see
the `veyron-dashboard-rev` meta tag). Design system details: [CloudOS vision](../CLOUDOS_VISION.md),
[theme](../THEME.md).

## 3. Feature deep-dive

### VM lifecycle
Declarative `VMConfig` (`src/config/types.rs`) covers CPU/memory, disks,
interfaces, cloud-init, firmware/clock/features, scheduling (node selector,
affinity, tolerations, topology spread, priority class), KubeVirt `instancetype`/
`preference` matchers, TPM, RNG, USB tablet, watchdog, host devices, and virtiofs.
Create from templates or blueprints; validate before apply (`veyron validate`);
generate raw YAML (`veyron generate`).

### Consoles & developer access
Browser VNC and serial consoles run over a direct Kubernetes WebSocket to
`/apis/subresources.kubevirt.io/v1/.../virtualmachineinstances/.../vnc` — avoiding
`virtctl` timeouts. RDP exposure (`/rdp-expose`, guest-agent enable/disable RDP) is
available for Windows guests. See [Developer VM access](../DEVELOPER_VM_ACCESS.md)
and [Windows on KubeVirt](../WINDOWS_KUBEVIRT_PRODUCTION.md).

### Live migration & HA
`veyron migrate` / the `/migrate` route drive `VirtualMachineInstanceMigration`;
`ha-config`, `evacuate-node`, and `evacuation-status` support node drains. GuestKit
provides a **migrate-score** to gauge migration readiness from inside the guest.

### Snapshots & backup/DR
KubeVirt snapshots with restore, plus a ConfigMap-driven snapshot **scheduler**
(with an optional lease for HA — `VEYRON_SCHEDULER_LEASE_*`). Backups and
Velero-based DR round out data protection. See [Snapshots](../SNAPSHOTS.md).

### Storage & disks
Hot-plug/hot-remove volumes, online disk expansion, PVC resize
(`PATCH /api/v1/storage/pvcs/:ns/:name`), storage-class inventory, and disk-health
reporting. See [Disk management](../DISK_MANAGEMENT.md).

### Networking
Per-VM/-workload ingress/egress, bandwidth and traffic views, network policies,
Cilium integration, and optional `packetwolf` network-intelligence wiring
(`VEYRON_PACKETWOLF_*`). See [Network management](../NETWORK_MANAGEMENT.md).

### GitOps & operator
`veyron gitops-export` renders live VMs to CR manifests; the Go operator
(`operator/`) reconciles `veyron.io` CRDs into KubeVirt objects. `VeyronPolicy`
enforces guardrails (CEL-style deny rules). A Terraform provider
(`terraform-provider-veyron/`) is also included.

### AI — Ask Zeus & copilots
An LLM assistant ("Ask Zeus") plus per-domain copilots (`src/copilot/`) for
forecasting, GitOps, security sentinel, performance, observability, alerts, and
storage doctor advice. Fully configurable and optional via `VEYRON_AI_*` and the
observability integration URLs. See [Veyron AI](../VEYRON_AI.md).

### Security, compliance & SOC
Trivy-backed VM image scanning, compliance reports, and a SOC module that can
export security events to Splunk, QRadar, Microsoft Sentinel, and Elastic, or fan
out via a SOAR webhook. See [SOC / SIEM](../SOC.md) and
[Optional integrations](../OPTIONAL_INTEGRATIONS.md).

### Observability & cost
First-class hooks for Prometheus, Loki, Jaeger/Tempo, Alertmanager, and Grafana,
plus cost reporting via OpenCost. A companion `charts/veyron-monitoring` chart ships
ServiceMonitors, PrometheusRules, and Grafana dashboards.

For a full capability list, see the [feature matrix](../FEATURE_MATRIX.md) and
[advanced features](../ADVANCED_FEATURES.md).
