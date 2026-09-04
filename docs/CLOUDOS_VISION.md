# CloudOS — Veyron Dashboard Architecture & Vision

CloudOS is the default shell for the Veyron dashboard: a macOS-inspired desktop for Kubernetes-native VM operations, presented with an **Apple Store–style** product UI (chapter heroes, paper/true-black themes, Action Blue). The UI ships as a single self-contained SPA embedded in the Rust binary — no separate frontend build, no CDN at runtime.

**Current dashboard revision:** `20260902b` (cache-bust via `?dash=<rev>` and `<meta name="veyron-dashboard-rev">`).

> Design SoT: `CLAUDE.md` § Design system — Apple Store / Zeus product look. IronWolf and `holo` CSS have been removed.

---

## 1. Overview

| Asset | Source | Role |
|---|---|---|
| HTML shell + inline JS | `src/api/web/dashboard.html` (~30k lines) | All ~65 pages, CloudOS chrome, Ask Zyra modal |
| Layout & VMR panels | `src/api/web/dashboard-veyron.css` | Panels, dock hooks, fleet cards |
| Theme tokens (SoT) | `src/api/web/dashboard-apple-themes.css` | Light paper + Dark iPad Pro tokens; chapter/band/shelf primitives |
| Sign-in page | `src/api/web/dashboard-apple-login.css` | Apple chapter login + Veyron / Zyvor identity |
| Page renderers | `src/api/web/dashboard-veyron.js` | Mission Control, Signal Desk, Fleet, Foundry, capsule tabs |
| Zyvor Z mark | `src/api/web/zyvor-z-mark.png` | `/assets/zyvor-z-mark.png` on login |

At startup, `src/api/http_server.rs` reads the CSS/JS siblings via `include_str!` and performs literal `replace()` into the HTML template before serving `GET /dashboard`:

```text
/*__VMR_CSS__*/     → carbon + carbon-deck + shell-layout + sidebar + veyron
                      + apple-login + apple-themes
/*__SHELL_LAYOUT__*/ → shell + sidebar + carbon (+ deck) + apple-login + apple-themes  (apple last = wins)
/*__VMR_JS__*/       → dashboard-veyron.js
```

The binary is fully self-contained. Browsers cache aggressively; bump `DASH_REV` / `__VEYRON_DASH_REV` when shipping UI changes.

---

## 2. CloudOS Shell

The active layout uses `body.mac-desktop-root`. Legacy `veyron-topbar` / `veyron-sidebar` chrome was removed; navigation lives in the desktop shell.

### Chrome layers

| Layer | Purpose | Key shortcuts |
|---|---|---|
| **Menubar** (40px) | App menu, Go, View, Fleet/Ops/Observe/Secure/AI, Control Center | ⌘J Ask Zyra, ⌘K Spotlight, ⌘⇧B Browse |
| **Finder sidebar** | Live primary nav (`#cloudos-finder`, `dashboard-sidebar.css`); collapsible sections, filter, tier locks | ⌘\\ cycle width |
| **Dock** | Visually suppressed; Launchpad / Browse / Finder cover the same jobs | — |
| **mac-page-window** | Content pane for the active `data-page` section | `.mac-page-toolbar` / chapter heroes |
| **Mission Control overlay** | Cluster overview grid (F3) | Fleet health at a glance |
| **Launchpad** | Full-page app grid | All tier-allowed pages |
| **Spotlight** | ⌘K fuzzy search | Pages, VMs, actions including Ask Zyra |
| **Control Center** | Theme, wallpaper, tier, quick tiles | Light/Dark, desktop tier |
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

The default Mission Control home matches the product reference layout:

| Surface | Behavior |
|---|---|
| **Menubar center** | Search, alert pill, **Home / Monitor / Ask Zyra** quick pills |
| **Finder Platform rail** | Stack Health → Settings; Favorites on Power+ |
| **Finder Ask Zyra** | Ask Zyra, Unhealthy VMs, Backup Advisor, Storage Doctor |
| **Mission Control** | Apple chapter hero + fleet briefing / quick actions / alerts |
| **Create VM** | Menubar Fleet → Create VM opens **forge wizard** (`openCreateModal` → `openForgeWizard`) |
| **Status bar** | Zen mode + clock when enabled |
| **Advanced home** | Restores denser metrics / activity where still present |

Zen mode: `veyron_zen_mode` in `localStorage`, or on by default when desktop tier is Automatic.

### Themes & wallpapers — Apple Light / Dark

Two themes ship (Settings → Theme, Control Center, menubar):

| Theme key | Mood |
|---|---|
| `light` | Default — paper `#f5f5f7`, white panels, Action Blue `#0071e3` |
| `dark` | iPad Pro true black `#000`, silver type, sky `#2997ff` |

Legacy ids (`tahoe`, `holo`, `nebula`, `carbon`, `daylight`, …) remap via `LEGACY_THEME_ALIASES` onto `light` or `dark`. Keys: `veyron_theme`, `veyron_wallpaper` (pairing: light→`tahoe`, dark→`midnight`).

**Surface tokens:** `dashboard-apple-themes.css` under `html[data-theme='light'|'dark']` — maps `--void`, `--hull`, `--panel*`, `--plasma`, `--ink*`, plus chapter/band/shelf primitives (`.apple-chapter`, `.apple-band`, `.apple-shelf`, `.apple-tile`). Status tokens (`--nominal/--caution/--critical/--inert`) stay theme-independent in `dashboard.html` `:root`.

**Sign-in:** `dashboard-apple-login.css` chapter login with Zyvor Z mark and Light/Dark appearance control before connect.

**Retired (do not restore):** IronWolf CSS, `dashboard-holo.css`, PacketWolf mirrored theme packs, multi-deck `glass-deck` aurora/spotlight/starfield.

### Typography & density

- System stack: `-apple-system`, SF Pro Display / Text, Helvetica Neue fallbacks (`--font-hero` / `--font-display`)
- Menubar 40px; prefer flat product sections over heavy glass cards
- Ask Zyra / AI surfaces use the Action Blue / sky accent family (not a separate violet mandate)

---

## 3. Fifteen-Page Map

Navigation is client-side: JavaScript toggles `[data-page]` visibility. Labels are centralized in `NAV_PAGE_LABELS` / `navPageLabel()` (HTML + `dashboard-veyron.js`).

| Page ID | Label | Key content |
|---|---|---|
| `home` | Mission Control | Fleet health ring, Ask Zyra panel, quick actions, alerts, pinned VMs (+ Forge placeholder) |
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

## 4. Ask Zyra (AI Assistant)

**Ask Zyra** is the user-facing name for the experience-layer assistant. Internal JS still uses `openCopilot()` / `/api/v1/experience/copilot/*` for API compatibility.

### Entry points

| Surface | Action |
|---|---|
| Dock ✦ button | `openAskZyra()` |
| ⌘J | Global shortcut |
| Menubar → Ask Zyra… | Opens modal |
| Finder → Ask Zyra section | Quick fleet prompts (Doctor, Storage, Cost, …) |
| Control Center tile | Ask Zyra |
| Spotlight / Launchpad | Search "zeus" or "copilot" (alias) |
| VM context menu | Ask Zyra (scoped to VM) |
| Mission Control briefing chips | Zyra briefing from `GET /experience/home` → `copilot_briefing` |
| Page stat pills | "Ask Zyra" on clickable fleet metrics |

The modal is a macOS-style sheet (`#copilot-modal`). Module badges show backend names (e.g. **Veyron Doctor**, **Veyron YAML Builder**); the default badge is **Ask Zyra**.

### Backend modules

See [VEYRON_AI.md](./VEYRON_AI.md) for the full API table. Deterministic advisors query KubeVirt/Kubernetes; optional LLM paraphrase/routing via `VEYRON_AI_*`.

Configure OpenRouter from shell keys:

```bash
./scripts/configure-zyra-openrouter.sh HOST USER
```

---

## 5. VM Inspector & Capsule

Selecting a VM or node in the roster no longer opens a persistent right-side rail. As of the 2026-08-04 "unbox everything" pass, `positionVmFocusPaneInline()` / `positionNodeFocusPaneInline()` (`dashboard.html`) move the Inspector `<aside>` via JS to sit directly after the clicked roster row — an accordion-style inline expand, full width, no side column. The split grid (`[data-shell-region="split"]`) collapses to a single column on desktop; the roster's own scroll box is un-bounded so expanded content flows with the page instead of clipping to a fixed-height panel. This only runs above the 1024px breakpoint — below it the pane is moved back to its original rail slot so the pre-existing mobile slide-in drawer (`shell-layout.css`) is unaffected.

Inspector content, unchanged:
- Identity, state, namespace, IP, node
- Resource summary (vCPU, memory, disk)
- **Console**, **Capsule**, **YAML**, **Ask Zyra**, **Diagnose**

Recent VMs: `localStorage` key `veyron_recent_vms` (Spotlight empty state).

---

## 6. Create VM Wizard

User-facing label is **Create VM**. Entry: menubar Fleet → Create VM, or `openCreateModal()` (routes to `openForgeWizard()` when `#forge-wizard-modal` exists). Six steps with a top progress rail:

1. **Template** — OS family from catalog
2. **Profile** — optional `VMProfile` CRD
3. **Hardware** — CPU, RAM, disk, GPU (when advertised)
4. **Network** — expose / internet egress
5. **Security** — TPM, Secure Boot, backup/policy bindings, limits
6. **Review** — summary → `POST /api/v1/vms` (Create / Create + Start)

Legacy single-sheet `#create-modal` remains in the DOM as fallback when the wizard is absent.

---

## 7. Responsive & Mobile

| Breakpoint | Behavior |
|---|---|
| ≥ 1280px | Full CloudOS: Finder + dock + page window |
| 768–1279px | Finder overlay with backdrop; quick nav bar |
| &lt; 768px | Bottom mobile nav (Mission Control, Fleet, Forge, Console, Ask Zyra); Browse as bottom sheet |

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

Dashboard tier checks in `test-remote.sh` validate CloudOS shell markers (`mac-desktop-root`, dock, Ask Zyra entry).

---

## 10. Related Docs

| Doc | Topic |
|---|---|
| [VEYRON_AI.md](./VEYRON_AI.md) | Ask Zyra API, modules, LLM agent layer |
| [OPTIONAL_INTEGRATIONS.md](./OPTIONAL_INTEGRATIONS.md) | Prometheus, Grafana, PacketWolf |
| [TEMPLATE_CATALOG.md](./TEMPLATE_CATALOG.md) | VMTemplate / VMProfile CRDs |
| [SOC.md](./SOC.md) | Security Posture / SOC handlers |
| [WINDOWS_KUBEVIRT_PRODUCTION.md](./WINDOWS_KUBEVIRT_PRODUCTION.md) | Windows RDP, Sysprep, golden images |
| [USER_STORIES.md](./USER_STORIES.md) | Persona journeys & acceptance criteria |

Source: `src/api/handlers/experience.rs`, `src/api/web/dashboard*.html|css|js`.
