# Veyron UI — Architecture & Vision

Veyron is a dark-theme, Kubernetes-native VM command center. The dashboard is a single-page application served inline from the Rust binary — no separate frontend build step, no CDN dependency.

---

## 1. Overview

The entire UI is one HTML file (`src/api/web/dashboard.html`, ~750 KB) with CSS and JavaScript injected at Rust compile time. `src/api/http_server.rs` reads three sibling files at startup via `include_str!` and performs a literal `replace()` substitution into the HTML template before serving `GET /`:

| Injected asset | Source file | Approx size |
|---|---|---|
| `<style>` block | `dashboard-veyron.css` | ~35 KB |
| `<script>` block | `dashboard-veyron.js` | ~134 KB |
| VMR CSS (compat) | `dashboard-vmr.css` | legacy shim |

No external network requests are required at runtime. The binary is self-contained.

---

## 2. Visual Language

All color tokens are defined as CSS custom properties on `:root`.

| Token | Value | Semantic use |
|---|---|---|
| `--blue` | `#3f7cff` | Primary actions, active nav highlight |
| `--cyan` | `#27d7ff` | Live signals, network indicators |
| `--green` | `#32e0b0` | Healthy states, success |
| `--orange` | `#ff9d2e` | Warnings, degraded states |
| `--red` | `#ff5570` | Failures, critical alerts |
| `--violet` | `#9b7cff` | AI / Veyron Copilot surfaces |

Panel background: `rgba(18, 24, 36, 0.74)` glass cards over a dark carbon body (`#0d1117`). Cards use `backdrop-filter: blur(12px)` and a `1px solid rgba(255,255,255,.08)` border.

---

## 3. 15-Page Structure

The sidebar is 240 px fixed-width. Each entry maps to a `data-page` section in the HTML.

| Page | Sidebar Icon | Key Content |
|---|---|---|
| Mission Control | ◈ | Fleet health ring, Veyron Copilot recommendations, Quick Actions, Active Alerts, Pinned VMs, Recent Activity |
| Fleet Command | ⚡ | VM cards / table / topology view, VM Inspector drawer, bulk start/stop/delete actions |
| Forge VM | ⚙ | 6-step creation wizard: Template → Profile → Hardware → Network → Security → Review |
| Template Foundry | 🗂 | 44+ OS templates, category rail, feature filters (GPU/Windows/UEFI/TPM), preview drawer |
| ConsoleHub | ▶ | noVNC, SSH-expose, RDP-expose; Link quality presets (LAN / Balanced / Low bandwidth) |
| VM Capsule | ⬡ | 8-tab single-VM deep dive: Overview / Performance / Network / Storage / Hardware / Security / Events / YAML |
| Stack Health | ◎ | KubeVirt / CDI subsystem status cards, readiness timeline, failure diagnostics |
| Event Intelligence | ⚠ | Filter bar (namespace/severity/time), incident cards with Impact / Root Cause / Suggested Fixes, timeline mode |
| Security Posture | 🛡 | Fleet security score, per-VM findings table, compliance framework status (CIS, NIST, SOC 2) |
| Network Intelligence | ⟷ | PacketWolf connection banner, flow table, traffic map |
| Snapshots & Backups | ⊞ | Per-VM protection status table, backup policy builder, restore history |
| Cost Explorer | $ | Namespace cost breakdown, 30-day forecast, optimization recommendations |
| Workloads | ◻ | 7-tab workload view: VMs / VMIs / virt-launcher pods / CDI pods / KubeVirt controllers / Veyron pods |
| Cluster Nodes | ○ | Per-node CPU/memory metrics, KubeVirt health, VM placement map, drain/cordon actions |
| Settings | ⚙ | 12 config sections: General / Cluster / Namespaces / API Keys / RBAC / Console / Templates / Backup / PacketWolf / Prometheus / Theme / Audit |

---

## 4. Architecture

### Monolithic SPA

`dashboard.html` contains all 15 page sections in a single DOM tree. Navigation is client-side only: JavaScript toggles `data-page` visibility; no HTML fragments are fetched after initial load. This keeps the binary fully self-contained and simplifies deployment behind TLS with a self-signed cert.

### Backend — SharedState

Every Axum handler shares a single `Arc<RwLock<WebState>>`:

```rust
pub type SharedState = Arc<RwLock<WebState>>;
// WebState: KubeClient, namespace, API key table, rate limiter
// All handlers: State(state): State<SharedState>
```

Each handler module exposes `pub fn router(state: SharedState) -> Router` and is merged in `src/api/handlers/mod.rs`.

### Auth

Three equivalent bearer forms are accepted on all API routes:

| Form | Example |
|---|---|
| Header | `X-API-Key: <key>` |
| Bearer JWT | `Authorization: Bearer <token>` (HMAC-SHA256 or OIDC) |
| Query param | `?token=<key>` |

Multi-key RBAC: `VEYRON_API_KEYS="admin:key1,write:key2,readonly:key3"`. Route-level minimum roles enforced in `src/api/auth_context.rs`.

### WebSocket endpoints

| Path | Protocol | Purpose |
|---|---|---|
| `/api/v1/ws/vnc/:ns/:name` | noVNC binary | VM VNC console proxy |
| `/api/v1/ws/serial/:ns/:name` | text | VM serial console |
| `/api/v1/ws/metrics` | JSON frames | Live fleet metrics push |
| `POST /api/v1/ws/ticket` | — | One-time ticket for VNC/serial auth |

VNC and serial WebSocket upgrades are exempt from the `VEYRON_HTTP_REQUEST_TIMEOUT_SECS` layer.

---

## 5. VM Inspector Drawer

Selecting any VM row or card in Fleet Command opens a 480 px right-side panel without leaving the page. The drawer shows:

- VM name, running state badge, namespace, IP address, host node
- Resource summary (vCPU, memory, disk)
- Action buttons: **Console** (opens ConsoleHub), **Capsule** (opens VM Capsule tab), **YAML** (raw manifest viewer), **Veyron Copilot** (opens AI chat scoped to this VM)

Recent VMs are tracked in `localStorage` under `veyron_recent_vms` and surfaced in Spotlight empty state.

---

## 6. Forge VM Wizard

Six sequential steps; progress is tracked in a step rail at the top of the panel.

| Step | Content |
|---|---|
| 1 — Template | Pick from 44+ templates grouped by OS family; preview shows default CPU/RAM/firmware |
| 2 — Profile | Optional VMProfile CRD from the cluster catalog; override fields shown inline |
| 3 — Hardware | vCPU count, memory, root disk size, GPU passthrough toggle |
| 4 — Network | Interface type (masquerade / bridge / SRIOV), NAD selection, extra NICs |
| 5 — Security | TPM enable/disable, Secure Boot toggle, backup policy binding, network policy binding, resource limits (requests/limits for CPU and memory) |
| 6 — Review | 3-column summary grid (Identity / Hardware / Security) + collapsible YAML preview of the generated KubeVirt `VirtualMachine` manifest; **Create VM** submits `POST /api/v1/vms` |

---

## 7. Responsive Layout

| Breakpoint | Sidebar | Mission grid | Notes |
|---|---|---|---|
| 1280 × 720 | 240 px fixed | 2-column | Minimum supported resolution |
| 1440 × 900 | 240 px fixed | 3-column | Default design target |
| ≥ 1920 | 240 px fixed | 4-column | Wide fleet cards |

Topbar: 56 px fixed. Sidebar: 240 px fixed. Content area: `calc(100vw - 240px)` with `16px` padding. All glass cards use CSS Grid with `auto-fit, minmax(320px, 1fr)` so columns reflow without JavaScript.

---

## 8. Veyron Copilot Integration

Copilot surfaces use the `--violet` (`#9b7cff`) color token throughout. Each module name maps to a scoped AI chat panel:

| Module | Scope |
|---|---|
| Veyron Copilot | General fleet assistant |
| Veyron Doctor | Diagnose a failing VM |
| Veyron Error Explainer | Translate raw K8s/KubeVirt error messages |
| Veyron YAML Builder | Generate VMRogue/KubeVirt manifests from natural language |
| Veyron Scheduling Explainer | Explain node placement decisions |
| Veyron Blueprint Studio | Design multi-VM VMRogueBlueprint CRs |
| Veyron Network Lens | Analyze Cilium policy and flow data |
| Veyron Storage Doctor | Diagnose PVC / DataVolume issues |
| Veyron Backup Advisor | Review snapshot and retention policies |
| Veyron Security Sentinel | Surface CVEs and policy gaps |
| Veyron GitOps Generator | Produce Kustomize / GitOps repo structure |

Copilot chat is routed through `/api/v1/experience/copilot/*`. The backend wires to an external LLM when `VEYRON_AI_*` environment variables are configured; without them the Copilot panels render but return a "not configured" response.

---

## 9. Related Docs

- `docs/OPTIONAL_INTEGRATIONS.md` — Prometheus, Grafana, PacketWolf
- `docs/TEMPLATE_CATALOG.md` — VMTemplate / VMProfile CRDs
- `docs/SOC.md` — Security Posture / SOC handler details
- `docs/WINDOWS_KUBEVIRT_PRODUCTION.md` — Windows guest setup (RDP, Sysprep)
- `src/api/handlers/experience.rs` — Experience API handler source
