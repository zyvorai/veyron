# Veyron AI — Ask Zyra & Lifecycle Intelligence

> **Veyron should be the AI-native VM lifecycle brain for KubeVirt** — deploy, inspect, heal, migrate, secure, resize, snapshot, backup, and explain VMs from one simple interface.

This document captures the long-term AI vision and what ships in **v1** today (rule-based assistant that composes real cluster data).

**UI branding:** the dashboard calls the assistant **Ask Zyra** (`openAskZyra()`, ⌘J). API routes and internal code still use the `/experience/copilot/*` path prefix.

## North star

```text
Veyron should feel like a smart VM operator for Kubernetes/KubeVirt,
not just a CLI or dashboard.
```

Users ask in plain language; Veyron responds with **root cause**, **evidence from the cluster**, **recommended fixes**, and **one-click actions** — not raw stack traces.

## Product modules (roadmap names)

| Module | Purpose |
|--------|---------|
| **Ask Zyra** | Natural-language entry point (UI); API: `POST /experience/copilot/ask` |
| **Veyron Doctor** | Per-VM health score + issues |
| **Veyron Error Explainer** | Translate K8s/KubeVirt errors |
| **Veyron YAML Builder** | Generate + validate VirtualMachine YAML |
| **Veyron Scheduling Explainer** | Human-readable Pending / Unschedulable |
| **Veyron Blueprint Studio** | Reusable AI-generated profiles |
| **Veyron Network Lens** | VM-to-VM connectivity debugging |
| **Veyron Storage Doctor** | PVC / snapshot bloat |
| **Veyron Backup Advisor** | Velero + policy gaps |
| **Veyron Security Sentinel** | Exposed RDP, policies, drift |
| **Veyron GitOps Generator** | Repo folders + runbooks |

## v1 shipped (deterministic Ask Zyra)

v1 does **not** require an external LLM. It routes intents to handlers that query KubeVirt/Kubernetes and reuse Veyron templates.

### API (`/api/v1/experience/copilot/*`)

| Route | Module |
|-------|--------|
| `POST /experience/copilot/ask` | Ask Zyra — natural language router |
| `GET /experience/copilot/doctor/:ns/:name` | Veyron Doctor |
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
veyron ai guest vm-db-01
veyron ai filesystem vm-db-01
veyron ai storage
veyron ai cost
veyron ai cost --name vm-app-01
veyron ai security --name vm-app-01
veyron ai "Why is my VM not starting?"
veyron ai doctor vm-db-01
veyron ai scheduling vm-app-01
veyron ai explain "0/5 nodes are available: 2 Insufficient memory"
veyron ai yaml --template windows-2022 --cpus 8 --memory 32Gi --disk 500Gi
veyron ai recommend "I need a VM for PostgreSQL"
```

### CloudOS UI

- **Ask Zyra** — dock ✦ button, ⌘J, Finder section, Control Center tile → modal → `POST /experience/copilot/ask`
- VM context menu **Ask Zyra** scopes queries to the selected VM
- Mission Control **Zyra briefing** chips from `GET /experience/home` → `copilot_briefing`
- Existing **Fix-it** modal still uses `/experience/errors/translate` (same rules as Error Explainer)

See [CLOUDOS_VISION.md](./CLOUDOS_VISION.md) for shell layout, tiers, and themes.

### Example Ask Zyra response shape

```json
{
  "module": "Veyron Doctor",
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
| 10 | Optional LLM backend | **Done** — `VEYRON_AI_URL` paraphrase layer |
| 11 | AI Cost Advisor | **Done** — OpenCost or reference rates, fleet + per-VM |

## Safety rails (product policy)

- Do not reboot production VMs without explicit approval
- Do not delete disks automatically
- Do not reduce CPU/RAM automatically
- Always recommend snapshot before risky changes
- YAML Builder **validates** against cluster before suggesting Apply

## Next phases

1. ~~**Blueprint Studio** — persist `VeyronBlueprint` from Copilot YAML~~ **Done** — `POST /experience/copilot/blueprint/save`
2. ~~**GitOps Generator** — export kustomize trees from Copilot previews~~ **Done** — `POST /experience/copilot/gitops/export`
3. ~~**Guest filesystem metrics** — guest-exec or Prometheus guest exporters~~ **Done** — `GET /api/v1/vms/:ns/:name/guest-filesystem`, Copilot `GET /experience/copilot/guest/:ns/:name/filesystem`, VM detail panel

## Copilot Expansion Pack (fleet advisors)

| Module | API | CLI |
|--------|-----|-----|
| Performance Advisor | `GET /experience/copilot/performance` | `veyron ai performance` |
| GitOps Advisor | `GET /experience/copilot/gitops` | `veyron ai gitops` |
| Forecast Advisor | `GET /experience/copilot/forecast` | `veyron ai forecast` |
| Integrations Advisor | `GET /experience/copilot/integrations` | `veyron ai integrations` |
| Scheduling Fleet | `GET /experience/copilot/scheduling` | `veyron ai pending` |

Natural-language routing via `POST /experience/copilot/ask` recognizes the same intents. Dashboard chips live on Monitoring, GitOps, Forecasting, Integrations, and Scheduling pages.

## Copilot Expansion Pack 2 (platform advisors)

| Module | API | CLI |
|--------|-----|-----|
| Compliance Advisor | `GET /experience/copilot/compliance` | `veyron ai compliance` |
| Observability Advisor | `GET /experience/copilot/observability` | `veyron ai observability` |
| Cilium Advisor | `GET /experience/copilot/cilium` | `veyron ai cilium` |
| Node Advisor | `GET /experience/copilot/nodes` | `veyron ai nodes` |
| Drift Advisor | `GET /experience/copilot/drift` | `veyron ai drift` |
| Alert Advisor | `GET /experience/copilot/alerts` | `veyron ai alerts` |
| SLO Advisor | `GET /experience/copilot/slo` | `veyron ai slo` |
| Migration Advisor | `GET /experience/copilot/migrations` | `veyron ai migrations` |

## LLM upgrade (agent layer)

When an API key is set (`VEYRON_AI_API_KEY`, `OPENROUTER_API_KEY`, or `ANTHROPIC_AUTH_TOKEN` with `sk-or-v1-…`):

| Env | Purpose |
|-----|---------|
| `VEYRON_AI_URL` | OpenAI-compatible base (OpenRouter default `https://openrouter.ai/api/v1`) |
| `VEYRON_AI_API_KEY` | Bearer token |
| `VEYRON_AI_MODE` | `off` / `paraphrase` (default) / `routing` / `agent` |
| `VEYRON_AI_MODEL` | Model id (`openrouter/free` default for OpenRouter keys) |
| `VEYRON_AI_APP_TITLE` | Optional OpenRouter `X-Title` (e.g. **Veyron**) |

**OpenRouter from `~/.zshrc`:** `./scripts/configure-zyra-openrouter.sh --print-env` or `./scripts/configure-zyra-openrouter.sh HOST USER` to patch `veyron-integrations` and restart the API.

| Env | Purpose |
|-----|---------|
| `VEYRON_AI_MAX_TOOL_ROUNDS` | Agent tool invocations per message (default `2`) |
| `VEYRON_AI_TIMEOUT_SECS` | LLM HTTP timeout (default `30`) |
| `VEYRON_AI_RATE_LIMIT_PER_MIN` | Separate rate bucket for `POST /copilot/chat` and `/copilot/ask` when LLM active (default `20`) |

| Route | Purpose |
|-------|---------|
| `POST /experience/copilot/chat` | Multi-turn chat with optional tool trace |
| `GET /experience/copilot/status` | LLM configured + active mode |

Evidence, validation, and YAML remain deterministic — LLM only routes or paraphrases.

The agent tool registry exposes **27 tools** (all fleet/per-VM advisors plus `yaml_preview` and `explain_error`).

When `VEYRON_AI_MODE=agent`, `POST /experience/copilot/chat` returns a `tool_trace` array showing which deterministic tools ran (displayed as chips in the Ask Zyra modal).

**Proactive briefing:** `GET /experience/home` includes `copilot_briefing` — up to 3 fleet insights (failed/pending/degraded VMs, node pressure) with one-click Ask Zyra actions on the Mission Control hero.

## Copilot Expansion Pack 3

| Module | API | CLI |
|--------|-----|-----|
| Quota Advisor | `GET /experience/copilot/quotas` | `veyron ai quotas` |
| Catalog Advisor | `GET /experience/copilot/catalog` | `veyron ai catalog` |
| Velero DR Advisor | `GET /experience/copilot/velero-dr` | `veyron ai velero-dr` |
| Trivy/CVE (Security Sentinel) | merged into `/experience/copilot/security` | `veyron ai security` |
| Prometheus p95 (Performance) | merged into `/experience/copilot/performance` | `veyron ai performance` |

## Future (not scheduled)

- ~~**Prometheus guest exporters** — optional join with `node_exporter` / Windows perf counters when `VEYRON_PROMETHEUS_URL` is set~~ **Done** — PVC kubelet stats + guest exporter queries on `GET /api/v1/vms/:ns/:name/guest-filesystem`
- ~~**Cost Copilot** — OpenCost integration for per-VM spend narratives~~ **Done** — `GET /experience/copilot/cost`, `veyron ai cost`
- ~~**Blueprint Studio UI** — edit saved `VeyronBlueprint` CRDs from the dashboard~~ **Done** — Platform → Blueprint Studio, `PUT /api/v1/crds/blueprints/:ns/:name`

## Related docs

- [CLOUDOS_VISION.md](./CLOUDOS_VISION.md) — experience layer UI
- [FEATURE_MATRIX.md](./FEATURE_MATRIX.md) — parity tracking
- [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows golden images
