<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# API

Everything the console does goes through the HTTPS API under `/api/v1`. The OpenAPI document is
served at `GET /api/openapi.json`.

## Authentication

Send one of:

| Method | How |
|---|---|
| API key | `X-API-Key: <key>` (or `Authorization: Bearer <key>`) |
| Local account | `POST /api/v1/auth/login` with `{"username","password"}` returns a session token |
| JWT | `Authorization: Bearer <jwt>`, HMAC-SHA256 (`VEYRON_JWT_SECRET`, `VEYRON_JWT_ISSUER`) |
| OIDC | `Authorization: Bearer <access_token>` from your IdP, verified against JWKS ([sso.md](sso.md)) |

Browser console and serial sessions use a one-time ticket from `POST /api/v1/ws/ticket`, so
long-lived credentials never go in a WebSocket URL.

## Roles

Three roles, each including the one below it:

| Role | Can |
|---|---|
| `readonly` | Every `GET` |
| `write` | Create, change and delete VMs, snapshots, networks, alerts and the rest |
| `admin` | Cluster-wide or destructive actions (below) and user management |

Admin-only routes, from `src/api/auth_context.rs`:

- `/auth/users*` (even `GET`)
- any path containing `/restore`
- `/dr/*`, `/tenants*`
- `/gitops/sync`, `/catalog/sync`
- `/clusters/:id/activate`
- `/nodes/:name/{cordon,uncordon,reboot}`
- `DELETE /storage/orphans`
- `/soc/playbooks/*`
- `/images/publish`
- `/platform/*`, except the public GuestKit binary download
- `/ai/settings/llm` and `/ai/mcp-servers*` (even `GET`)

AI routes that only read or draft (`/ai/chat`, `/ai/chat/stream`, `/ai/search`, `/ai/intent/vm`,
`/ai/policies/draft`, `/mcp`) need only `readonly`; anything they want to change becomes a
proposal that needs the step's role to approve. See [ai.md](ai.md).

### Keys

- `VEYRON_API_KEY` is a single key with the `admin` role.
- `VEYRON_API_KEYS` holds several keys in the form `name:key:role`, comma-separated, for example
  `ops:k1:admin,ci:k2:write,viewer:k3:readonly`. A missing role means `readonly`.
- Local console accounts are managed under **Security → Users & roles** (admin only), or through
  `/api/v1/auth/users`.

## Route families

| Area | Examples |
|---|---|
| VMs | `GET/POST /vms`, `POST /vms/:ns/:name/{start,stop,restart,pause,unpause,migrate}`, `DELETE /vms/:ns/:name` |
| Compute | `POST /vms/:ns/:name/hotplug`, `PUT /vms/:ns/:name/run-strategy`, `POST /vms/bulk` |
| Access | `GET/PUT/DELETE /vms/:ns/:name/expose` (SSH), `/rdp-expose` (RDP), `PUT /vms/:ns/:name/network/internet` |
| Console | `GET /vms/:ns/:name/vnc`, `/serial` (WebSocket, ticket required) |
| Guests | `GET /vms/:ns/:name/guest/{status,evidence,doctor,fix-plan}`, `/guest-filesystem`, `POST /vms/:ns/:name/guest/patch` |
| VM disks | `GET /vms/:ns/:name/volumes/status`, `POST …/volumes/{hotplug,hotremove}`, `POST …/storage/data-disk` (+ `/defaults`), `GET …/migrations` |
| Data | `/snapshots`, `/snapshot-schedules`, `/backups`, `/clones`, `/storage/{usage,pools,orphans}`, `/velero/*`, `/atlas/*` |
| Images | `/images`, `/images/upload`, `/images/publish` |
| Fleet | `/nodes`, `/gpus`, `/pods`, `/workloads`, `/capacity/headroom`, `/platform/capabilities` |
| Operations | `/alerts`, `/events`, `/incidents/timeline`, `/logs`, `/metrics`, `/costs`, `/costs/budgets`, `/recommendations`, `/slo/*`, `/self-healing/*` |
| Security | `/soc/*`, `/security/{findings,posture}`, `/compliance/*`, `/audit/{trail,stats}`, `/rbac/*`, `/vms/:ns/:name/security` |
| Platform | `/operators`, `/helm/releases`, `/namespaces`, `/quotas`, `/gitops/*`, `/catalog/*`, `/custom-resources`, `/clusters` |
| Network | `/networks`, `/cilium/*`, `/network-policies`, `/ingress` |
| AI | `GET /ai/{status,tools,forecast,investigations}`, `POST /ai/{chat,chat/stream,search,intent/vm}`, `POST /ai/policies/{draft,preview}`, `POST /ai/investigations/run` |
| AI proposals | `GET /ai/proposals`, `GET /ai/proposals/:id`, `POST /ai/proposals/:id/{approve,reject}` |
| AI models | `GET/POST /ai/models`, `DELETE /ai/models/:ns/:name`, `GET/PUT /ai/settings/llm` |
| MCP | `POST /mcp` (MCP server, Streamable HTTP), `GET/PUT /ai/mcp-servers`, `POST /ai/mcp-servers/:server/call` |
| Sandboxes | `GET/POST /sandboxes`, `GET/DELETE /sandboxes/:id`, `POST /sandboxes/:id/exec`, `GET/PUT /sandboxes/:id/files` |

Query `?namespace=all` for cluster-wide lists.

Snapshot schedules take standard five-field cron (`minute hour day month weekday`, UTC), for
example `0 2 * * *`; Veyron stores it with a leading seconds field (`0 0 2 * * *`). Six-field
expressions are accepted unchanged.

## Errors

Errors come back as JSON with an HTTP status that means something: `400` bad input, `401` no or bad
credentials, `403` role too low, `404` missing, `409` conflict (for example migrating a GPU
passthrough VM), `422` failed preflight. Features the current VM engine can't provide return
`501 Not Implemented` with a reason, never a fake success.

## Example

```bash
BASE=https://<node-ip>:30151
H="X-API-Key: $VEYRON_API_KEY"

curl -sk -H "$H" "$BASE/api/v1/vms?namespace=all"
curl -sk -H "$H" -X POST "$BASE/api/v1/vms/default/demo/stop"
curl -sk -H "$H" -H 'Content-Type: application/json' -X POST "$BASE/api/v1/snapshots" \
  -d '{"vm_name":"demo","namespace":"default","name":"before-upgrade"}'
```
