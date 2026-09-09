# CloudOS / console UI

Veyron’s browser UI is the **React console** at **`/console`** (`frontend/`), embedded into the API binary via `include_dir!`.

Operate the cluster via:

- **Console**: `https://<host>:30151/console` (API key login)
- **HTTP API**: `GET /api/v1/health`, VM/lifecycle routes under `/api/v1/…`
- **CLI**: `veyron` commands

Auth: `X-API-Key` (stored by the console as `localStorage['veyron_api_key']`) or OIDC bearer — see `CLAUDE.md`.

The old single-file CloudOS `dashboard.html` SPA was removed; do not restore it.
