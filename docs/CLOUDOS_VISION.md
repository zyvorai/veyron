# Zyvor CloudOS — Product Vision

> **KubeVirt is the virtualization engine. Kubernetes is the orchestration fabric. Zyvor CloudOS is the operating system above both.**

CloudOS is the experience layer for VMRogue: a macOS/iOS-inspired control plane that hides Kubernetes vocabulary while preserving full platform power behind the scenes.

## Principles

1. **Product vocabulary first** — Users see *Workspaces*, *Virtual Machines*, *Disks*, *Networks*, and *Health* — not CRDs, pods, or virt-launcher jargon in default flows.
2. **One decisive home** — Datacenter Home aggregates fleet health, quick actions, and operator signals in a single surface.
3. **Progressive disclosure** — Advanced K8s/KubeVirt tools remain available in classic pages; CloudOS routes power users there when needed.
4. **Fix-it, not stack traces** — API and operator errors are translated into plain language with suggested next steps.

## Architecture

```text
┌─────────────────────────────────────────────────────────┐
│  CloudOS UI (dashboard.html)                            │
│  Datacenter Home · VM cards · Spotlight · Copilot       │
└──────────────────────────┬──────────────────────────────┘
                           │ /api/v1/experience/*
┌──────────────────────────▼──────────────────────────────┐
│  Experience API (handlers/experience.rs)                │
│  home · search · errors/translate · session · health    │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│  VMRogue API + operator + KubeVirt                      │
└─────────────────────────────────────────────────────────┘
```

The experience API is an **aggregation and vocabulary layer**. It does not replace KubeVirt reconciliation; it composes existing VMRogue handlers and cluster queries into product-shaped responses.

## Experience API (v1)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/experience/home` | Datacenter Home: greeting, health score, stats, action cards |
| `GET /api/v1/experience/search?q=` | Spotlight backend: VMs, pages, quick actions |
| `POST /api/v1/experience/errors/translate` | Fix-it error translator (`message`, optional `context`) |
| `GET /api/v1/experience/session` | Role + write/admin flags for UI |
| `GET /api/v1/experience/fleet/health` | Per-VM health scores |
| `GET /api/v1/experience/templates` | Template Store catalog |
| `GET /api/v1/experience/locations` | Multi-cluster as CloudOS locations |
| `POST /experience/copilot/*` | VMRogue Copilot — see [VMROGUE_AI.md](./VMROGUE_AI.md) |

Namespace scope follows the dashboard namespace selector (`?namespace=` or `all`).

## UI surfaces

### Datacenter Home + VM cards

- **Datacenter Home** — Hero with greeting, workspace label, health ring, and action cards from `/experience/home`.
- **VM cards** — Grid view on the VMs page: app-like tiles with status, resources, and primary Open/Start action.

### Spotlight + Fix-it + Copilot

- **Spotlight** — `⌘K` / `Ctrl+K` opens search; queries `/experience/search`.
- **Fix-it** — Failed VM actions call `/experience/errors/translate` and show a modal with explanation and navigation/retry actions.
- **Copilot** — Natural-language VM operator via `/experience/copilot/*`.

### Finder + Locations

- **Finder** — Favorites / Operations / Network / AI sections (collapsible).
- **Locations** — Multi-cluster context in the top bar.

## Mapping: CloudOS ↔ Kubernetes

| CloudOS term | VMRogue / K8s backing |
|--------------|----------------------|
| Workspace | Kubernetes namespace |
| Virtual Machine | KubeVirt `VirtualMachine` / VMRogueVM |
| Template Store | VMTemplate / VMProfile CRDs + embedded catalog |
| Disk | PVC / DataVolume |
| Network | NAD, masquerade/slirp, Cilium policies |
| Health | VM status + node Ready + warning events |

## Related docs

- [VMROGUE_AI.md](./VMROGUE_AI.md) — AI copilot vision + v1 API/CLI
- [FEATURE_MATRIX.md](./FEATURE_MATRIX.md) — API and UI parity tracking
- [TEMPLATE_CATALOG.md](./TEMPLATE_CATALOG.md) — VMTemplate / VMProfile
- [OPTIONAL_INTEGRATIONS.md](./OPTIONAL_INTEGRATIONS.md) — Prometheus, Grafana, Velero

## Internal codenames

- **CloudOS** — customer-facing experience name
- **HyperOS** — optional short internal codename
- **experience_compose** — `data_source` tag on experience API responses
