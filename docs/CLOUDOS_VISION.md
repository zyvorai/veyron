# CloudOS / dashboard UI — removed

Veyron no longer ships a browser dashboard or React console. Operate the cluster via the **CLI** and **HTTP API** (`veyron serve` / `api-serve`).

- Health: `GET /api/v1/health`
- Auth: `X-API-Key` or OIDC bearer (see `CLAUDE.md` / `docs/SOC.md` as applicable)

Historical design notes for the retired CloudOS SPA are gone with `src/api/web/` and `frontend/`.
