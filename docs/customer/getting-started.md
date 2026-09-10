# Getting Started with Veyron

## What you need

| Requirement | Notes |
|-------------|--------|
| Kubernetes + KubeVirt | CDI recommended for disk imports |
| Veyron API + console | Local **`:8080`** or cluster HTTPS **`:5151`** (NodePort often **30151**) |
| Auth | Username/password against the console's login screen, or `VEYRON_API_KEY` / `VEYRON_API_KEYS` for API/CLI clients |

## 1. Open the console

- Local: `http://127.0.0.1:8080/console`
- Cluster: `https://<host>:30151/console` (chart defaults vary — use your deploy output)

## 2. Sign in

The console's login screen takes a **username and password**, posted to
`POST /api/v1/auth/login`, which returns a bearer token the console stores and sends back as
`Authorization: Bearer <token>` on every API call.

| Mode | What you do |
|------|-------------|
| Username/password | The only method the console's login screen exposes today. `admin` + the password from `VEYRON_API_KEY`/`auth.adminPassword` works out of the box — see [Admin basics](admin-basics.md) for the default. |
| API key (`X-API-Key`) | For CLI, scripts, and direct API calls — not used by the console's login screen. `VEYRON_API_KEY` (single admin key) or `VEYRON_API_KEYS="name:key:role,..."` (multi-key RBAC). |
| OIDC / SSO | Backend support exists (`VEYRON_OIDC_*`, `GET /api/v1/auth/oidc/config`, `POST /api/v1/auth/oidc/token`) for programmatic PKCE flows — see [Setting Up SSO](sso-setup.md). The console's login screen does not yet have an SSO button; this is API-level today. |

## 3. Orient yourself

1. **Mission Control** (Overview) — fleet summary and quick create.
2. **Left rail** — five groups: Overview, Compute, Storage & network, Security, System.
3. **Search** (top bar) — filter/jump within the current page's data.
4. See [Using the Console](using-the-console.md) for the full shell layout.

## 4. First workflows

### A. List and open a VM

**Virtual machines** (Compute) → select a row → the **Inspector** (right panel) opens with
detail, console access, and the per-VM action bar.

### B. Open a console session

**ConsoleHub** (Overview), or open a VM's Inspector and use its console action directly.

### C. Create from a template

**Template Foundry** (Compute) to browse OS templates, or the **+ New** button (top bar) to
create a VM directly.

## Next steps

- [Using the Console](using-the-console.md)
- [Admin basics](admin-basics.md)
- [Page guides](pages/README.md)
