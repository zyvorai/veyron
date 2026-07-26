# Admin Basics (Veyron)

## Ports

| Port | Service |
|------|---------|
| **8080** | Local HTTP API + dashboard (`veyron api-serve` / `serve`) |
| **5151** | Container / Helm HTTPS API + dashboard |
| **5150** / **30150** | HTTP→HTTPS redirect (chart) |
| **30151** | Common HTTPS NodePort for dashboard / API |

## Auth

- `VEYRON_API_KEY` (admin) or `VEYRON_API_KEYS="name:key:role,..."`.
- OIDC PKCE via `VEYRON_OIDC_*`; groups `veyron-admin` / `veyron-write`.
- WebSocket consoles use one-shot tickets: `POST /api/v1/ws/ticket`.
- Shell HTML (`/`, `/dashboard`, `/assets/*`) and health/OIDC bootstrap paths are auth-exempt.

## Install sketch

```bash
# Local
veyron api-serve   # or: veyron serve

# Helm (example)
helm upgrade --install veyron charts/veyron -n veyron --create-namespace
```

OpenAPI: `/api/openapi.json`. Route dump: `veyron api-routes`.

## Related

- [Getting Started](getting-started.md)
