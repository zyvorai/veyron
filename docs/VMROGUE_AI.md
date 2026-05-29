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
| `GET /experience/copilot/network/:ns/:name` | Network Lens — per-VM posture |

| `GET /experience/copilot/guest/:ns/:name` | Guest Inspector |
| `GET /experience/copilot/storage` | Storage Doctor |
| `GET /experience/copilot/security/:ns/:name` | Security Sentinel (per VM) |
| `GET /experience/copilot/security` | Security Sentinel (fleet) |

### CLI

```bash
vmrogue ai guest vm-db-01
vmrogue ai storage
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

## Safety rails (product policy)

- Do not reboot production VMs without explicit approval
- Do not delete disks automatically
- Do not reduce CPU/RAM automatically
- Always recommend snapshot before risky changes
- YAML Builder **validates** against cluster before suggesting Apply

## Next phases

1. ~~**Blueprint Studio** — persist `VMRogueBlueprint` from Copilot YAML~~ **Done** — `POST /experience/copilot/blueprint/save`
2. ~~**GitOps Generator** — export kustomize trees from Copilot previews~~ **Done** — `POST /experience/copilot/gitops/export`
3. **Guest filesystem metrics** — guest-exec or Prometheus guest exporters

## Related docs

- [CLOUDOS_VISION.md](./CLOUDOS_VISION.md) — experience layer UI
- [FEATURE_MATRIX.md](./FEATURE_MATRIX.md) — parity tracking
- [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows golden images
