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
│  Datacenter Home · VM cards · Spotlight · Ask Zeus        │
└──────────────────────────┬──────────────────────────────┘
                           │ /api/v1/experience/*
┌──────────────────────────▼──────────────────────────────┐
│  Experience API (handlers/experience.rs)                │
│  home · desktop · search · errors/translate · session   │
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
| `GET /api/v1/experience/desktop` | macOS shell: desktop tiers, dock defaults, dock catalog, keyboard shortcuts, optional fleet summary |
| `POST /experience/copilot/*` | Ask Zeus / VMRogue Copilot — see [VMROGUE_AI.md](./VMROGUE_AI.md) |

Namespace scope follows the dashboard namespace selector (`?namespace=` or `all`).

## Navigation (macOS 26 shell)

| Layer | Role | When visible |
|-------|------|--------------|
| **Menubar** | ZeusOS / Go / View menus, status island, workspace + theme | Always |
| **Dock** | Primary app launcher + Launchpad + **Ask Zeus** + Spotlight | Always |
| **Dynamic Island** | Expandable fleet health / alerts summary | Menubar center |
| **Control Center** | Theme, wallpaper, tier, workspace, refresh | Menubar ▥ button |
| **Page toolbar** | Compact window title + primary action per page | Normal / Power (replaces bulky page headers) |
| **Content window** | Single liquid-glass `.mac-page-window` pane per page | All tiers |
| **Desktop tabs** | Recent pages (Stage Manager lite) | Power (slim) / Advanced (full); hidden on Normal |
| **Mission Control** | Full-screen overlay (F3 / ⌃↑) | Overlay |
| **Launchpad** | Template grid quick launch | ZeusOS / View menu + dock |
| **Finder** | Favorites, Operations, Ask Zeus shortcuts | Collapsed by default on Normal; toolbar label **Finder** |
| **Browse** | Full classic page mega-menu | Advanced tier only |

### Desktop density tiers

| Tier | Finder | Tabs | Browse | Page chrome | Dashboard home |
|------|--------|------|--------|-------------|----------------|
| **Normal** | Collapsed default | Hidden | Hidden | Compact toolbar + glass content window | Datacenter Home + Today only (no fleet-command duplicate) |
| **Power** | Visible | Slim strip | Hidden | Toolbar + stats row | Datacenter + optional charts in `<details>` |
| **Advanced** | Visible | Full tabs | Browse button | Classic `.page-header` optional | Full command grid + charts |

Dock pins persist in `localStorage` (`vmrogue_dock_pins`); tier changes reset to API `dock_defaults` for that tier. **Customize Dock…** reorders pins from `dock_catalog`.

**Desktop tabs** (Stage Manager lite): recent pages show below the menubar with a close control (×) and middle-click to dismiss; **Home** is always pinned.

**Menubar clock** shows local date/time on the right (updates every 30s).

**Spotlight** (`⌘K`): empty state lists macOS shell shortcuts (Mission Control, Customize Dock, Launchpad, Control Center, Finder, Screen Sharing, Forge VM); API search merges the same shell actions when queries match.

**Notifications** use a top-right Notification Center–style banner (glass, icon per severity). Menubar **🔔** opens a Notification Center panel (recent items + link to full page).

**VM cards** support a macOS-style **right-click context menu** (Connect, VNC, RDP, Diagnose, Snapshot).

**Launchpad** supports arrow keys + Enter when open; **`/`** focuses the search field. Dock icons **bounce** on navigation (respects reduced motion).

**VM Inspector** (Fleet split view): glass workspace + sticky inspector header with VM name/namespace; recent VMs appear in Finder and Spotlight empty state (`vmrogue_recent_vms`).

**Mission Control** overlay: keys **1–4** jump to Infrastructure / Compute / Network / Storage zones; number keys select open desktop windows when listed.

**Desktop tabs** can be **drag-reordered** (non-Home tabs). Normal/Power tiers hide inline **nav-status** (health lives in the Dynamic Island).

**Control Center** includes a **Quick actions** tile grid (Spotlight, Mission Control, Launchpad, Finder, Alerts, Dock, Prefs).

**About VMRogue** shortcuts list is populated from `GET /experience/desktop` → `keyboard_shortcuts`.

List-view VMs support the same **right-click context menu** as grid cards.

**ZeusOS Preferences** (`⌘,` or ZeusOS menu): System Settings sheet for desktop tier, theme, wallpaper, workspace, and refresh. Normal/Power tiers hide theme/refresh from the menubar (use Control Center or Preferences).

**Launchpad** includes a search field; **double-click** a VM card/row to Connect (or Start if stopped).

**Finder** footer shows the active window title; **Mission Control** uses macOS-style header chrome.

**Ask Zeus** (fleet AI): **single** entry surface — dock (✦), **⌘J**, Control Center tile, Spotlight, Finder quick-advisor links, VM inspector chips, and context menu. Opens a right-side sheet with an **OpenRouter** badge when `VMROGUE_AI_*` is configured (`./scripts/configure-zeus-openrouter.sh HOST USER` reads `~/.zshrc`). Per-page Copilot input bars and the legacy bottom “Ask + Drawer” bar are **removed** from the DOM (not merely hidden).

Keyboard: `⌘K` Spotlight, `⌘J` Ask Zeus, `⌘\` Finder, `⌘⌥S` Finder (Machina parity).

**Remote console (VNC / Serial / RDP):** VNC and Serial use macOS Screen Sharing window chrome — traffic lights, **‹ Back**, `Esc` / `⌘W`, minimize pill above the dock. **Remote Desktop (RDP)** opens a macOS sheet with NodePort expose, guest-agent enable/disable, and copy-to-clipboard connect strings (Microsoft Remote Desktop / FreeRDP).

Cache-bust the dashboard after deploy: `?dash=20260602r` (or current `vmrogue-dashboard-rev` meta tag).

## UI surfaces

### Datacenter Home + VM cards

- **Datacenter Home** — Hero with greeting, workspace label, health ring, and action cards from `/experience/home`.
- **VM cards** — Grid view on the VMs page: app-like tiles with status, resources, and primary Open/Start action.

### Spotlight + Fix-it + Ask Zeus

- **Spotlight** — `⌘K` / `Ctrl+K` opens search; queries `/experience/search`.
- **Fix-it** — Failed VM actions call `/experience/errors/translate` and show a modal with explanation and navigation/retry actions.
- **Ask Zeus** — Natural-language VM operator via `/experience/copilot/*` (one sheet; no per-page duplicate inputs).

### Finder + Dock + Locations

- **Dock** — Glass bar at bottom; default pins from `/experience/desktop`; active app shows indicator dot; **Ask Zeus** is the AI pin (not a separate “Copilot” app).
- **Finder** — Favorites / Operations / Network / Ask Zeus sections (collapsible); toggle via menubar or `⌘\`.
- **Locations** — Multi-cluster context in the menubar.

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
