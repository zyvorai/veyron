# Zyvor CloudOS — Product Vision

> **KubeVirt is the virtualization engine. Kubernetes is the orchestration fabric. Zyvor CloudOS is the operating system above both.**

CloudOS is the experience layer for VMRogue: a macOS/iOS-inspired control plane that hides Kubernetes vocabulary while preserving full platform power behind the scenes.

## Principles

1. **Product vocabulary first** — Users see *Workspaces*, *Virtual Machines*, *Disks*, *Networks*, and *Health* — not CRDs, pods, or virt-launcher jargon in default flows.
2. **One decisive home** — Datacenter Home aggregates fleet health, quick actions, and operator signals in a single surface.
3. **Progressive disclosure** — Advanced K8s/KubeVirt tools remain available in classic pages; CloudOS routes power users there when needed.
4. **Fix-it, not stack traces** — API and operator errors are translated into plain language with suggested next steps.
5. **Migration as a first-class app** — VMware/OVA/VMDK paths are guided workflows, not a loose collection of CDI YAML.

## Architecture

```text
┌─────────────────────────────────────────────────────────┐
│  CloudOS UI (dashboard.html)                            │
│  Datacenter Home · VM cards · Spotlight · Migration     │
└──────────────────────────┬──────────────────────────────┘
                           │ /api/v1/experience/*
┌──────────────────────────▼──────────────────────────────┐
│  Experience API (handlers/experience.rs)                │
│  home · search · errors/translate · migration/*         │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│  VMRogue API + operator + KubeVirt + CDI                │
└─────────────────────────────────────────────────────────┘
```

The experience API is an **aggregation and vocabulary layer**. It does not replace KubeVirt reconciliation; it composes existing VMRogue handlers and cluster queries into product-shaped responses.

## Experience API (v1 skeleton)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/experience/home` | Datacenter Home: greeting, health score, stats, control center tiles, action cards |
| `GET /api/v1/experience/search?q=` | Spotlight backend: VMs, pages, quick actions |
| `POST /api/v1/experience/errors/translate` | Fix-it error translator (`message`, optional `context`) |
| `GET /api/v1/experience/migration/sources` | Supported migration source types |
| `POST /api/v1/experience/migration/scan` | Readiness scan over inventory items |
| `POST /api/v1/experience/migration/plan` | Per-VM migration plan steps |
| `POST /api/v1/experience/migration/execute` | CDI DataVolume import + optional VMRogueVM create |

Namespace scope follows the dashboard namespace selector (`?namespace=` or `all`).

### Health score (heuristic)

The home health score blends running VM ratio, node readiness, and warning penalties. It is intentionally simple until guest metrics and SLO burn rates are wired. Treat scores below 70 as “needs attention,” not as a formal SLA.

### Migration pipeline

```text
Source (VMware / OVA / VMDK)
  → readiness checks (firmware, disk format, VirtIO, network)
  → CDI DataVolume import (HTTP URL or upload path)
  → PVC bound
  → VMRogueVM / KubeVirt VirtualMachine from template + profile
  → cutover & decommission source
```

Full vCenter inventory sync and live vMotion-style migration are **future** work; the current assistant focuses on disk import and guided planning.

## UI surfaces (classic dashboard)

### A) Datacenter Home + VM cards + Control Center

- **Datacenter Home** — Hero with greeting, workspace label, health ring, and action cards from `/experience/home`.
- **Control Center bar** — Persistent strip under the page context bar with cluster health, running VMs, nodes, warnings; toggles from the same API.
- **VM cards** — Grid view on the VMs page (list/card toggle): app-like tiles with status, resources, and primary Open/Start action.

### B) Spotlight + Fix-it

- **Spotlight** — `⌘K` / `Ctrl+K` opens search; queries `/experience/search`.
- **Fix-it** — Failed VM actions call `/experience/errors/translate` and show a modal with explanation and navigation/retry actions.

### C) Migration Assistant

- **Platform → Migration** — Wizard: pick source, paste or edit inventory JSON, scan readiness, review plan, execute CDI import.

## Mapping: CloudOS ↔ Kubernetes

| CloudOS term | VMRogue / K8s backing |
|--------------|----------------------|
| Workspace | Kubernetes namespace |
| Virtual Machine | KubeVirt `VirtualMachine` / VMRogueVM |
| Template Store | VMTemplate / VMProfile CRDs + embedded catalog |
| Disk | PVC / DataVolume |
| Network | NAD, masquerade/slirp, Cilium policies |
| Health | VM status + node Ready + warning events |
| Migration task | CDI import + operator reconcile |

## Roadmap status

| Phase | Capability | Status |
|-------|------------|--------|
| **Now** | Experience API, Datacenter Home, Spotlight, Fix-it, migration scan/plan/execute | **Done** |
| **Next** | Guest health scores, Template App Store, Finder sidebar, Locations bar, RBAC session | **Done** (classic dashboard) |
| **Awesome** | Calmer chrome, Today panel, VM cards-first, Finder sections, Spotlight recents, migration wizard | **Done** — merged location bar, stepper UI, `GET /experience/migration/tasks` |
| **Later** | vCenter API live inventory, live migration polish, mobile shell | Partial — discover stub + migrate fix-it; full vCenter API pending |
| **Enterprise** | RBAC-filtered action cards, multi-location switcher | Partial — session role + location bar; full tenant RBAC pending |

### Experience API (full surface)

| Route | Purpose |
|-------|---------|
| `GET /experience/session` | Role + write/admin flags for UI |
| `GET /experience/fleet/health` | Per-VM health scores |
| `GET /experience/templates` | Template Store catalog |
| `GET /experience/locations` | Multi-cluster as CloudOS locations |
| `POST /experience/migration/discover` | vCenter inventory stub → JSON template |
| `GET /experience/migration/tasks` | In-progress CDI imports (`vmrogue.io/managed-by=vmrogue-migration`) |
| `POST /experience/copilot/ask` | VMRogue Copilot — NL router to Doctor, YAML, scheduling, explain |
| `GET /experience/copilot/doctor/:ns/:name` | Per-VM health report |
| `GET /experience/copilot/scheduling/:ns/:name` | Scheduling failure explainer |

### Awesome sprint UI (dashboard)

- **Top bar** — page title, compact location + workspace selector, Browse filter mega-menu
- **Datacenter Home** — single hero with health chips; **Today** panel (stats + priority queue); pinned running VMs; operator details collapsed
- **VMs** — cards-first grid with OS icons, health pills, Open/RDP hints
- **Finder** — Favorites / Operations / Network sections (collapsible); pin pages from Browse
- **Spotlight** — recents in `localStorage`, grouped results, keyboard hints
- **Migration** — 5-step wizard (Source → Workloads table → Readiness → Import → Tasks)

## Related docs

- [VMROGUE_AI.md](./VMROGUE_AI.md) — AI copilot vision + v1 API/CLI
- [FEATURE_MATRIX.md](./FEATURE_MATRIX.md) — API and UI parity tracking
- [TEMPLATE_CATALOG.md](./TEMPLATE_CATALOG.md) — VMTemplate / VMProfile
- [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) — Windows golden images
- [OPTIONAL_INTEGRATIONS.md](./OPTIONAL_INTEGRATIONS.md) — Prometheus, Grafana, Velero

## Internal codenames

- **CloudOS** — customer-facing experience name
- **HyperOS** — optional short internal codename
- **experience_compose** — `data_source` tag on experience API responses
