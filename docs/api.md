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
| Images | `/images`, `/images/catalog` (CDI `/images/upload`, `/images/publish` are KubeVirt-only) |
| Image catalog (Kairon) | `GET/POST /machine-images`, `DELETE /machine-images/:name`, `PUT /image-store/:name` (raw body upload), `GET /image-store`, `DELETE /image-store/:name` |
| Imports | `GET/POST /imports`, `GET /imports/:ns/:name` — boot a migrated disk (h2kvm) on Kairon from a URL + SHA-256 |
| Guest logs | `GET /vms/:ns/:name/logs?tail=` |
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

### Images on Kairon (no CDI)

Kairon replaces CDI with its own catalog. Upload a file into Veyron's image store, publish it as a
cluster-scoped `MachineImage`, then create VMs from it by name:

```bash
# 1. Upload (Write role). The body is the raw file; Veyron hashes it while it streams.
curl -sk -X PUT -H "X-API-Key: $VEYRON_API_KEY" --data-binary @ws2022.iso \
  "https://$HOST:30151/api/v1/image-store/ws2022.iso"

# 2. Publish (Admin). kind defaults to iso for .iso uploads, disk otherwise.
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/machine-images" \
  -d '{"name":"ws2022-iso","from_store":"ws2022.iso","os":"windows"}'

# Or publish a disk that already lives on an HTTP(S) server
#   {"name":"ubuntu-24.04","url":"https://…/noble.qcow2","sha256":"…","format":"qcow2",
#    "kind":"disk","disk_size":"20Gi","cpu":"2","memory":"4Gi"}

# 3a. Boot from a catalog disk image
#   POST /vms {"name":"web1","template":"ubuntu-24.04","image":{"name":"ubuntu-24.04"}}
# 3b. Install from an ISO onto a blank disk (Windows templates add the virtio-win CD when that
#     MachineImage exists; "driver_iso" picks another, "" attaches none)
#   POST /vms {"name":"win01","template":"windows-2022","iso":"ws2022-iso","disk_size":"80Gi"}
# 3c. Put the root disk on a PVC (empty, Filesystem mode); Kairon seeds it on first start
#   POST /vms {…, "image":{"name":"ubuntu-24.04"}, "root_volume":"web1-root"}
```

Kairon nodes download uploads from `GET /api/v1/image-store/blobs/<sha256>`. That path needs no
credentials (kairon-node sends none): the digest works as the capability and every node checks the
bytes against it. Publishing an upload writes the blob URL into the `MachineImage`; set
`VEYRON_IMAGE_STORE_PUBLIC_URL` when nodes should use another address than the API's NodePort.
Deleting an upload that a `MachineImage` still uses answers `409 IN_USE`. On KubeVirt these routes
answer `501 KAIRON_REQUIRED`; `iso`, `driver_iso` and `root_volume` on `POST /vms` answer `400`.

`image` on Kairon resolves a `MachineImage`, not a CDI DataSource. Without `disk_size` the root disk
takes the image's size (or the `MachineImage`'s `defaults.diskSize`); a size smaller than the image
fails rather than truncating it. VMs with install media attached can't live-migrate.

#### Capture a VM as a golden image

`POST /api/v1/vms/:ns/:name/capture` (Admin) turns a running or stopped Kairon VM's root disk into a
new catalog image. It answers `202` and runs in the background: Veyron takes a `MachineBackup`
(guest filesystems are frozen while the guest agent answers; `"quiesce":"required"` fails without
it, `"never"` skips it), streams the backup's root qcow2 from the node's kairon-node relay into the
image store, publishes a `MachineImage` of kind `disk`, then deletes the backup unless
`"keep_backup":true`.

```bash
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/vms/default/web1/capture" \
  -d '{"name":"web-gold-v1","os":"linux","disk_size":"20Gi","cpu":"2","memory":"4Gi"}'
# → {"phase":"BackingUp","backup":"web1-capture-20261008…",…}

curl -sk -H "X-API-Key: $VEYRON_API_KEY" "https://$HOST:30151/api/v1/image-captures"
# phases: BackingUp → Downloading → Publishing → Succeeded | Failed (with message)
```

The name must be new in both the catalog and the store (`409` otherwise). Generalize the guest
first if clones need fresh identities (`cloud-init clean`, Sysprep `/generalize` on Windows). Only
image-backed root disks can be captured; a root disk on a PVC belongs to `MachineSnapshot`. Job
records are ConfigMaps labeled `veyron.io/type=image-capture` in the API namespace;
`DELETE /api/v1/image-captures/:image` removes a finished one. Needs Kairon with the
`/backup-root` relay route and FluxVM with `GET /v1/backups/{name}/root`.

Query `?namespace=all` for cluster-wide lists. `GET /vms` leaves agent sandbox VMs out unless
you pass `include_sandboxes=true` or ask for their namespace.

Snapshot schedules take standard five-field cron (`minute hour day month weekday`, UTC), for
example `0 2 * * *`; Veyron stores it with a leading seconds field (`0 0 2 * * *`). Six-field
expressions are accepted unchanged.

## Errors

Errors come back as JSON with an HTTP status that means something: `400` bad input, `401` no or bad
credentials, `403` role too low, `404` missing, `409` conflict (for example migrating a GPU
passthrough VM), `422` failed preflight. Features the current VM engine can't provide return
`501 Not Implemented` with a reason, never a fake success; KubeVirt/CDI-only routes use the
code `KUBEVIRT_ONLY`.

## Example

```bash
BASE=https://<node-ip>:30151
H="X-API-Key: $VEYRON_API_KEY"

curl -sk -H "$H" "$BASE/api/v1/vms?namespace=all"
curl -sk -H "$H" -X POST "$BASE/api/v1/vms/default/demo/stop"
curl -sk -H "$H" -H 'Content-Type: application/json' -X POST "$BASE/api/v1/snapshots" \
  -d '{"vm_name":"demo","namespace":"default","name":"before-upgrade"}'
```
