# CloudOS — Veyron Dashboard Architecture & Vision

CloudOS is the default shell for the Veyron dashboard: a macOS Tahoe–inspired desktop for Kubernetes-native VM operations. The UI ships as a single self-contained SPA embedded in the Rust binary — no separate frontend build, no CDN at runtime.

**Current dashboard revision:** `20260618f` (cache-bust via `?dash=<rev>` and `<meta name="veyron-dashboard-rev">`).

> **Staleness note:** this doc predates the **IronWolf** theme pivot (see `CLAUDE.md` § Design system) and the growth to 65 pages. The Themes section and page/line counts below have been corrected; other sections (tier gating, chrome-layer specifics) may still describe an earlier revision and would benefit from a fuller refresh alongside the active 65-page redesign sweep.

---

## 1. Overview

| Asset | Source | Role |
|---|---|---|
| HTML shell + inline JS | `src/api/web/dashboard.html` (~30k lines) | All 65 pages, CloudOS chrome, Ask Zeus modal |
| Layout & VMR panels | `src/api/web/dashboard-veyron.css` | Glass surfaces, dock, fleet cards |
| Theme tokens | `src/api/web/dashboard-ironwolf-themes.css` | IronWolf `tahoe`/`light` token blocks |
| Sign-in page | `src/api/web/dashboard-ironwolf-login.css` | Split-screen login hero + glass panel |
| Page renderers | `src/api/web/dashboard-veyron.js` | Mission Control, Signal Desk, Fleet, Foundry, capsule tabs |

At startup, `src/api/http_server.rs` reads the CSS/JS siblings via `include_str!` and performs literal `replace()` into the HTML template before serving `GET /`:

```text
/*__VMR_CSS__*/  →  dashboard-veyron.css + dashboard-ironwolf-themes.css + dashboard-ironwolf-login.css
/*__VMR_JS__*/   →  dashboard-veyron.js
```

The binary is fully self-contained. Browsers cache aggressively; bump `DASH_REV` / `__VEYRON_DASH_REV` when shipping UI changes.

---

## 2. CloudOS Shell

The active layout uses `body.mac-desktop-root`. Legacy `veyron-topbar` / `veyron-sidebar` chrome was removed; navigation lives in the desktop shell.

### Chrome layers

| Layer | Purpose | Key shortcuts |
|---|---|---|
| **Menubar** (40px) | App menu, View, tier badge, clock, Control Center | ⌘J Ask Zeus, ⌘K Spotlight, ⌘⇧B Browse |
| ~~Finder sidebar~~ | Legacy collapsible sidebar (`#cloudos-finder`) — force-hidden at boot (`initFinderCollapsed()`), its reopen control permanently disabled. Dead code pending removal; **not** live navigation. | — |
| **Dock** | Pinned pages + Ask Zeus (✦) + trash metaphor | Click or Launchpad |
| **mac-page-window** | Content pane for the active `data-page` section | `.mac-page-toolbar` replaces legacy `.page-header` on Normal/Power |
| **Mission Control overlay** | Cluster overview grid (F3) | Fleet health at a glance |
| **Launchpad** | Full-page app grid | All tier-allowed pages |
| **Spotlight** | ⌘K fuzzy search | Pages, VMs, actions including Ask Zeus |
| **Control Center** | Theme, wallpaper, tier, quick tiles | Ask Zeus tile, desktop tier switcher |
| **Browse sheet** | Mega-menu of all pages (⌘⇧B) | Mobile: bottom sheet |

### Desktop tiers

Stored in `localStorage` as `veyron_desktop_tier` + `veyron_desktop_tier_auto`. Default: **Automatic** (all pages reachable; UI expands to Advanced chrome when you open platform pages).

| Tier | Finder | Dock pages | Gated pages |
|---|---|---|---|
| **Automatic** | Visible | Power dock; expands to Advanced UI on platform pages | None — Spotlight/Finder/Browse always navigate |
| **Normal** | Hidden | Core VM ops | Manual tier shows upgrade toast |
| **Power** | Visible | Normal + Stack Health, Events, Snapshots | Advanced-only pages toast or auto-elevate |
| **Advanced** | Visible | Full cluster surface | None |

`autoElevateTierForPage()` bumps manual tiers; Automatic mode skips gating entirely.

### Mock-aligned chrome (Power / Automatic home)

The default Mission Control home matches the `veyron.png` reference layout:

| Surface | Behavior |
|---|---|
| **Menubar center** | Search, alert pill, **Home / Monitor / Ask Zeus** quick pills |
| **Finder Platform rail** | Stack Health → Settings (nine links); Favorites collapsed on Power |
| **Finder Ask Zeus** | Four shortcuts: Ask Zeus, Unhealthy VMs, Backup Advisor, Storage Doctor |
| **Mission Control** | Compact title + Live badge; 4-card grid (Fleet Health, Ask Zeus, Quick Actions, Alerts) |
| **Pinned VMs** | Always visible with dashed **Forge New VM** placeholder |
| **Status bar** | **Zen mode on** (Automatic tier or View → Zen mode) + large clock above dock |
| **Advanced home** | Restores hero, metrics strip, and Recent Activity on Mission Control |

Zen mode: `veyron_zen_mode` in `localStorage`, or on by default when desktop tier is Automatic. Hides desktop tabs and shows the status-bar label.

### Themes & wallpapers — IronWolf

Two themes ship today (Settings → Theme, Control Center, menubar), branded **IronWolf**:

| Theme key | Mood |
|---|---|
| `tahoe` | Default dark — calm steel-blue metal-sheen surfaces (`#64a0dc` accent) |
| `light` | Daylight counterpart |

Every previous theme id (`holo`, `nebula`, `solaris`, `biolume`, `sakura`, `mono`, `inferno`, `voltage`, `daylight`, `prism`, `wolf`, `forge`, `ember`, `sonoma`, `graphite`, `slate`, `frost`, etc.) is remapped onto `tahoe` or `light` via `LEGACY_THEME_ALIASES` in `dashboard.html` — existing `localStorage` values migrate silently on load, no user-visible break.

Keys: `veyron_theme`, `veyron_wallpaper`.

**Surface tokens:** `dashboard-ironwolf-themes.css` defines the token blocks under `html[data-theme='tahoe']`/`html.theme-tahoe` and a `light` counterpart — `--void`, `--hull`, `--panel*`, `--plasma`, `--ink*`, `--metal-sheen`/`--metal-highlight`/`--accent-steel*`. Status tokens (`--nominal/--caution/--critical/--inert`) are theme-independent, defined once in `dashboard.html`'s base `:root`. Glass cards, forms, tables, hero panels, and charts read these vars so both themes tint correctly with no per-page work.

**Sign-in theme picker:** the API key modal (`dashboard-ironwolf-login.css` split-screen hero + glass panel) includes a tahoe/light appearance grid. Theme choice persists via `veyron_theme` before connect.

**Retired:** the prior 8/10-theme "Holographic Ops" `glass-deck` family and its PacketWolf-mirrored `dashboard-packetwolf-themes.css` surface tokens have been removed. See `CLAUDE.md` § "The retired `glass-deck` family" for what's left to prune.

### Typography & density

- System stack: `-apple-system`, SF Pro, Segoe UI fallbacks
- Menubar 40px; glass cards with `backdrop-filter: blur(12–20px)`
- `--violet` (`#9b7cff`) for Ask Zeus / AI surfaces

---

## 3. Fifteen-Page Map

Navigation is client-side: JavaScript toggles `[data-page]` visibility. Labels are centralized in `NAV_PAGE_LABELS` / `navPageLabel()` (HTML + `dashboard-veyron.js`).

| Page ID | Label | Key content |
|---|---|---|
| `home` | Mission Control | Fleet health ring, Ask Zeus panel, quick actions, alerts, pinned VMs (+ Forge placeholder) |
| `vms` | Fleet Command | VM cards/table/topology, inspector drawer, bulk lifecycle |
| `create` | Forge VM | 6-step wizard: Template → Profile → Hardware → Network → Security → Review |
| `app-store` | Template Foundry | 44+ OS templates, category rail, GPU/Windows/UEFI filters |
| `console` | ConsoleHub | noVNC, SSH expose, RDP expose; link quality presets (`veyron_vnc_preset`) |
| `vm-detail` | VM Capsule | 8-tab deep dive: Overview, Performance, Network, Storage, Hardware, Security, Events, YAML |
| `stack-health` | Stack Health | KubeVirt/CDI readiness, subsystem cards |
| `events` | Event Intelligence | Filtered incidents, timeline, suggested fixes |
| `security` | Security Posture | Fleet score, findings, CIS/NIST/SOC2 frameworks |
| `network` | Network Intelligence | PacketWolf banner, flows, Cilium context |
| `snapshots` | Snapshots & Backups | Protection status, policies, restore history |
| `costs` | Cost Explorer | Namespace breakdown, forecast, optimization |
| `workloads` | Workloads | VMs, VMIs, virt-launcher, CDI, controllers, Veyron pods |
| `nodes` | Cluster Nodes | Per-node metrics, placement, cordon/drain |
| `settings` | Settings | General, cluster, namespaces, API keys, RBAC, console, templates, backup, PacketWolf, Prometheus, theme, audit |
| `blueprint-studio` | Blueprint Studio | Edit `VeyronBlueprint` CRDs (Advanced tier) |

---

## 4. Ask Zeus (AI Assistant)

**Ask Zeus** is the user-facing name for the experience-layer assistant. Internal JS still uses `openCopilot()` / `/api/v1/experience/copilot/*` for API compatibility.

### Entry points

| Surface | Action |
|---|---|
| Dock ✦ button | `openAskZeus()` |
| ⌘J | Global shortcut |
| Menubar → Ask Zeus… | Opens modal |
| Finder → Ask Zeus section | Quick fleet prompts (Doctor, Storage, Cost, …) |
| Control Center tile | Ask Zeus |
| Spotlight / Launchpad | Search "zeus" or "copilot" (alias) |
| VM context menu | Ask Zeus (scoped to VM) |
| Mission Control briefing chips | Zeus briefing from `GET /experience/home` → `copilot_briefing` |
| Page stat pills | "Ask Zeus" on clickable fleet metrics |

The modal is a macOS-style sheet (`#copilot-modal`). Module badges show backend names (e.g. **Veyron Doctor**, **Veyron YAML Builder**); the default badge is **Ask Zeus**.

### Backend modules

See [VEYRON_AI.md](./VEYRON_AI.md) for the full API table. Deterministic advisors query KubeVirt/Kubernetes; optional LLM paraphrase/routing via `VEYRON_AI_*`.

Configure OpenRouter from shell keys:

```bash
./scripts/configure-zeus-openrouter.sh HOST USER
```

---

## 5. VM Inspector & Capsule

Selecting a VM in Fleet opens a right-side drawer (480px) without leaving the page:

- Identity, state, namespace, IP, node
- Resource summary (vCPU, memory, disk)
- **Console**, **Capsule**, **YAML**, **Ask Zeus**, **Diagnose**

Recent VMs: `localStorage` key `veyron_recent_vms` (Spotlight empty state).

---

## 6. Forge VM Wizard

Six steps with a top progress rail:

1. **Template** — OS family from catalog
2. **Profile** — optional `VMProfile` CRD
3. **Hardware** — CPU, RAM, disk, GPU
4. **Network** — masquerade/bridge/SRIOV, NADs
5. **Security** — TPM, Secure Boot, backup/policy bindings, limits
6. **Review** — summary grid + collapsible YAML → `POST /api/v1/vms`

---

## 7. Responsive & Mobile

| Breakpoint | Behavior |
|---|---|
| ≥ 1280px | Full CloudOS: Finder + dock + page window |
| 768–1279px | Finder overlay with backdrop; quick nav bar |
| &lt; 768px | Bottom mobile nav (Mission Control, Fleet, Forge, Console, Ask Zeus); Browse as bottom sheet |

Mission Control and fleet grids use CSS Grid `auto-fit, minmax(320px, 1fr)`.

---

## 8. Backend Architecture

### Monolithic SPA

All pages live in one DOM tree. No fragment fetches after initial load — ideal for TLS with self-signed certs and air-gapped clusters.

### SharedState

```rust
pub type SharedState = Arc<RwLock<WebState>>;
// WebState: KubeClient, namespace, API key table, rate limiter
```

Handlers: `pub fn router(state: SharedState) -> Router` merged in `src/api/handlers/mod.rs`.

### Auth

| Form | Example |
|---|---|
| Header | `X-API-Key: <key>` |
| Bearer | `Authorization: Bearer <jwt>` |
| Query | `?token=<key>` |

Multi-key RBAC: `VEYRON_API_KEYS="admin:key1,write:key2,readonly:key3"`.

### WebSockets

| Path | Purpose |
|---|---|
| `/api/v1/ws/vnc/:ns/:name` | noVNC proxy |
| `/api/v1/ws/serial/:ns/:name` | Serial console |
| `/api/v1/ws/metrics` | Live fleet metrics |
| `POST /api/v1/ws/ticket` | One-time VNC/serial ticket |

VNC, serial, and metrics streams bypass `VEYRON_HTTP_REQUEST_TIMEOUT_SECS`.

---

## 9. Testing the Dashboard

After deploy to a remote cluster:

```bash
# HTTPS smoke (health, templates, VM list)
./scripts/verify-veyron-remote.sh HOST [30151]

# Full cluster E2E via SSH
./scripts/test-remote.sh HOST USER

# Daily VM lifecycle (~10–15 min)
VEYRON_API_KEY='...' ./scripts/test-vm-daily-ops-remote.sh HOST [30151]
```

Force cache refresh in browser: `https://HOST:30151/dashboard?dash=20260618b`

Dashboard tier checks in `test-remote.sh` validate CloudOS shell markers (`mac-desktop-root`, dock, Ask Zeus entry).

---

## 10. Related Docs

| Doc | Topic |
|---|---|
| [VEYRON_AI.md](./VEYRON_AI.md) | Ask Zeus API, modules, LLM agent layer |
| [OPTIONAL_INTEGRATIONS.md](./OPTIONAL_INTEGRATIONS.md) | Prometheus, Grafana, PacketWolf |
| [TEMPLATE_CATALOG.md](./TEMPLATE_CATALOG.md) | VMTemplate / VMProfile CRDs |
| [SOC.md](./SOC.md) | Security Posture / SOC handlers |
| [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) | Windows RDP, Sysprep, golden images |
| [USER_STORIES.md](./USER_STORIES.md) | Persona journeys & acceptance criteria |

Source: `src/api/handlers/experience.rs`, `src/api/web/dashboard*.html|css|js`.
