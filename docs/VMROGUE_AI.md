# VMRogue AI — Product Direction

> **VMRogue should be the AI-native VM lifecycle brain for KubeVirt** — deploy, inspect, heal, migrate, secure, resize, snapshot, backup, and explain VMs from one simple interface.

This document captures the long-term AI vision and what ships in **v1** today (rule-based copilot that composes real cluster data).

## North star

```text
VMrougue should feel like a smart VM operator for Kubernetes/KubeVirt,
not just a CLI or dashboard.
```

Users ask in plain language; VMRogue responds with **root cause**, **evidence from the cluster**, **recommended fixes**, and **one-click actions** — not raw stack traces.

## Product modules (roadmap names)

| Module | Purpose |
|--------|---------|
| **VMRogue Copilot** | Natural-language entry point |
| **VMRogue Doctor** | Per-VM health score + issues |
| **VMRogue Error Explainer** | Translate K8s/KubeVirt errors |
| **VMRogue YAML Builder** | Generate + validate VirtualMachine YAML |
| **VMRogue Scheduling Explainer** | Human-readable Pending / Unschedulable |
| **VMRogue Blueprint Studio** | Reusable AI-generated profiles |
| **VMRogue Network Lens** | VM-to-VM connectivity debugging |
| **VMRogue Storage Doctor** | PVC / snapshot bloat |
| **VMRogue Backup Advisor** | Velero + policy gaps |
| **VMRogue Security Sentinel** | Exposed RDP, policies, drift |
| **VMRogue GitOps Generator** | Repo folders + runbooks |

## v1 shipped (deterministic copilot)

v1 does **not** require an external LLM. It routes intents to handlers that query KubeVirt/Kubernetes and reuse VMRogue templates.

### API (`/api/v1/experience/copilot/*`)

| Route | Module |
|-------|--------|
| `POST /experience/copilot/ask` | Copilot — natural language router |
| `GET /experience/copilot/doctor/:ns/:name` | VMRogue Doctor |
| `GET /experience/copilot/scheduling/:ns/:name` | Scheduling Explainer |
| `POST /experience/copilot/yaml/preview` | YAML Builder + validation |
| `POST /experience/copilot/recommend` | Blueprint-style recommendation |
| `GET /experience/copilot/backup` | Backup Advisor — fleet coverage |
| `GET /experience/copilot/cost` | Cost Advisor — fleet spend |
| `GET /experience/copilot/cost/:ns/:name` | Cost Advisor — per VM |
| `GET /experience/copilot/network/:ns/:name` | Network Lens — per-VM posture |
| `GET /experience/copilot/guest/:ns/:name` | Guest Inspector |
| `GET /experience/copilot/guest/:ns/:name/filesystem` | Guest filesystem (guest-exec) |
| `GET /experience/copilot/storage` | Storage Doctor |
| `GET /experience/copilot/security/:ns/:name` | Security Sentinel (per VM) |
| `GET /experience/copilot/security` | Security Sentinel (fleet) |

### CLI

```bash
vmrogue ai guest vm-db-01
vmrogue ai filesystem vm-db-01
vmrogue ai storage
vmrogue ai cost
vmrogue ai cost --name vm-app-01
vmrogue ai security --name vm-app-01
vmrogue ai "Why is my VM not starting?"
vmrogue ai doctor vm-db-01
vmrogue ai scheduling vm-app-01
vmrogue ai explain "0/5 nodes are available: 2 Insufficient memory"
vmrogue ai yaml --template windows-2022 --cpus 8 --memory 32Gi --disk 500Gi
vmrogue ai recommend "I need a VM for PostgreSQL"
```

### CloudOS UI

- Top bar **Copilot** button → modal → `POST /experience/copilot/ask`
- Existing **Fix-it** modal still uses `/experience/errors/translate` (same rules as Error Explainer)

### Example Copilot response shape

```json
{
  "module": "VMRogue Doctor",
  "title": "Health report: default/vm-db-01",
  "summary": "vm-db-01 is Running. Fleet health: Healthy (92/100).",
  "evidence": ["Scheduled node: worker-2", "Event FailedScheduling: ..."],
  "recommendations": ["Reduce CPU/memory or add capacity."],
  "actions": [{ "label": "View events", "action": "navigate", "page": "events" }],
  "health_score": 92,
  "validation": null,
  "yaml_preview": null
}
```

## Best v1 feature set (implemented / partial)

| Priority | Feature | Status |
|----------|---------|--------|
| 1 | AI Error Explainer | **Done** — `copilot/fixit.rs`, Fix-it modal |
| 2 | AI YAML Builder | **Done** — preview + StorageClass/memory/template checks |
| 3 | AI VM Doctor | **Done** — events, VMI, health score |
| 4 | AI Scheduling Explainer | **Done** — event-driven reasons + fixes |
| 5 | AI Backup Advisor | **Done** — snapshot/schedule/Velero coverage |
| 6 | AI Network Lens | **Done** — interfaces, expose, policies per VM |
| 7 | AI Guest Inspector | **Done** — guest agent, OS info, Doctor merge |
| 8 | AI Storage Doctor | **Done** — PVC pressure, snapshot sprawl |
| 9 | AI Security Sentinel | **Done** — RDP expose, drift, policies |
| 10 | Optional LLM backend | **Done** — `VMROGUE_AI_URL` paraphrase layer |
| 11 | AI Cost Advisor | **Done** — OpenCost or reference rates, fleet + per-VM |

## Safety rails (product policy)

- Do not reboot production VMs without explicit approval
- Do not delete disks automatically
- Do not reduce CPU/RAM automatically
- Always recommend snapshot before risky changes
- YAML Builder **validates** against cluster before suggesting Apply

## Next phases

1. ~~**Blueprint Studio** — persist `VMRogueBlueprint` from Copilot YAML~~ **Done** — `POST /experience/copilot/blueprint/save`
2. ~~**GitOps Generator** — export kustomize trees from Copilot previews~~ **Done** — `POST /experience/copilot/gitops/export`
3. ~~**Guest filesystem metrics** — guest-exec or Prometheus guest exporters~~ **Done** — `GET /api/v1/vms/:ns/:name/guest-filesystem`, Copilot `GET /experience/copilot/guest/:ns/:name/filesystem`, VM detail panel

## Copilot Expansion Pack (fleet advisors)

| Module | API | CLI |
|--------|-----|-----|
| Performance Advisor | `GET /experience/copilot/performance` | `vmrogue ai performance` |
| GitOps Advisor | `GET /experience/copilot/gitops` | `vmrogue ai gitops` |
| Forecast Advisor | `GET /experience/copilot/forecast` | `vmrogue ai forecast` |
| Integrations Advisor | `GET /experience/copilot/integrations` | `vmrogue ai integrations` |
| Scheduling Fleet | `GET /experience/copilot/scheduling` | `vmrogue ai pending` |

Natural-language routing via `POST /experience/copilot/ask` recognizes the same intents. Dashboard chips live on Monitoring, GitOps, Forecasting, Integrations, and Scheduling pages.

## Copilot Expansion Pack 2 (platform advisors)

| Module | API | CLI |
|--------|-----|-----|
| Compliance Advisor | `GET /experience/copilot/compliance` | `vmrogue ai compliance` |
| Observability Advisor | `GET /experience/copilot/observability` | `vmrogue ai observability` |
| Cilium Advisor | `GET /experience/copilot/cilium` | `vmrogue ai cilium` |
| Node Advisor | `GET /experience/copilot/nodes` | `vmrogue ai nodes` |
| Drift Advisor | `GET /experience/copilot/drift` | `vmrogue ai drift` |
| Alert Advisor | `GET /experience/copilot/alerts` | `vmrogue ai alerts` |
| SLO Advisor | `GET /experience/copilot/slo` | `vmrogue ai slo` |
| Migration Advisor | `GET /experience/copilot/migrations` | `vmrogue ai migrations` |

## Future (not scheduled)

- ~~**Prometheus guest exporters** — optional join with `node_exporter` / Windows perf counters when `VMROGUE_PROMETHEUS_URL` is set~~ **Done** — PVC kubelet stats + guest exporter queries on `GET /api/v1/vms/:ns/:name/guest-filesystem`
- ~~**Cost Copilot** — OpenCost integration for per-VM spend narratives~~ **Done** — `GET /experience/copilot/cost`, `vmrogue ai cost`
- ~~**Blueprint Studio UI** — edit saved `VMRogueBlueprint` CRDs from the dashboard~~ **Done** — Platform → Blueprint Studio, `PUT /api/v1/crds/blueprints/:ns/:name`

## Related docs

- [CLOUDOS_VISION.md](./CLOUDOS_VISION.md) — experience layer UI
- [FEATURE_MATRIX.md](./FEATURE_MATRIX.md) — parity tracking
- [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows golden images
