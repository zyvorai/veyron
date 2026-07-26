# Getting Started with Veyron

## What you need

| Requirement | Notes |
|-------------|--------|
| Kubernetes + KubeVirt | CDI recommended for disk imports |
| Veyron API / dashboard | Local **`:8080`** or cluster HTTPS **`:5151`** (NodePort often **30151**) |
| Auth | `VEYRON_API_KEY` / `VEYRON_API_KEYS`, or OIDC |

## 1. Open the UI

- Local: `http://127.0.0.1:8080/dashboard`
- Cluster: `https://<host>:30151/dashboard` (chart defaults vary — use your deploy output)
- Ask Zeus deep link: `https://<host>:30151/ask-zeus`

## 2. Sign in

| Mode | What you do |
|------|-------------|
| API key | Bearer / `X-API-Key` from `VEYRON_API_KEY` (or named keys with roles) |
| OIDC | PKCE browser login when `VEYRON_OIDC_*` is configured |
| Local lab | Auth may be `none` on :8080 — do not expose publicly |

## 3. Orient yourself

1. **Mission Control** (`/dashboard`) — home wall and Finder.
2. **Browse mega-menu** — Compute, Observe, Storage, Network, Security, Platform, FinOps, Ops.
3. **Spotlight (`⌘K`)** — jump to any page by label.
4. Hash routes: `/dashboard#vms`, `/dashboard#console-hub`, etc.

## 4. First workflows

### A. List and open a VM

**VMs** (`#vms`) → select a row → open **VM Capsule** or **vCentre** for detail.

### B. Open a console

**ConsoleHub** (`#console-hub`) — VNC / serial (WebSocket ticket flow).

### C. Create from a template

**Template Foundry** (`#app-store`) or **Blueprint Studio** (`#blueprint-studio`).

### D. Ask Zeus

**Ask Zeus** (`#ask-zeus`) — cluster-grounded Q&A before mutating actions.

## Next steps

- [Using the Dashboard](using-the-dashboard.md)
- [Admin basics](admin-basics.md)
- [Page guides](pages/README.md)
