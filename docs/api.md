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
| Local account | `POST /api/v1/auth/login` with `{"username","password"}` returns a session token (a JWT, valid 24 h) |
| JWT | `Authorization: Bearer <jwt>`, HMAC-SHA256 (`VEYRON_JWT_SECRET`, `VEYRON_JWT_ISSUER`, role claim `VEYRON_JWT_ROLE_CLAIM`, default `role`; unknown or missing role means `readonly`) |
| OIDC | `Authorization: Bearer <access_token>` from your IdP, verified against JWKS ([sso.md](sso.md)) |

Notes that matter in practice:

- **`VEYRON_API_KEY` must be set.** Without it every request (except the public routes below) answers
  `503 AUTH_NOT_CONFIGURED`, even when `VEYRON_API_KEYS`, JWT or OIDC are configured.
- The only unauthenticated routes are `GET /api/v1/health`, the console shell and assets, `POST /auth/login`,
  `GET /auth/oidc/config`, `POST /auth/oidc/token`, `GET /platform/guestkit/binary` and the image-store blob download
  (the SHA-256 in its path is the credential).
- Login needs `VEYRON_JWT_SECRET` and `VEYRON_JWT_ISSUER` (otherwise `503`). Logins are rate limited per client: 30
  attempts per 5 minutes per client IP and 10 per 5 minutes per IP and username (`429 RATE_LIMITED`); one client's
  flood does not lock out others. The client IP is the TCP peer address (`X-Forwarded-For` is not trusted), so behind
  a reverse proxy all users share the proxy's address and its budget. Passwords are at least 8 characters, stored as
  Argon2 hashes in the `veyron-users` Secret. Session tokens last 24 h but are revocable: deleting a user or
  resetting their password invalidates every token issued before (checked against the user list, cached for up to
  5 s). Changing a user's role is not applied to tokens already issued (the role is in the token), and tokens issued
  before this check existed expire on their own within 24 h.
- The legacy `?token=<key>` query parameter is accepted only on WebSocket upgrade requests (`/vms/:ns/:name/{vnc,serial,tunnel}`
  and `/ws/metrics`), where browsers cannot set headers. On every other route it is ignored and the request answers
  `401`. Use headers; consoles should use a ticket.
- OIDC bearer tokens are checked for signature (RS256 via `VEYRON_OIDC_JWKS_URL`), issuer and expiry, and for
  audience when `VEYRON_OIDC_AUDIENCE` is set (unset: not checked). Without a JWKS URL, tokens are validated by calling `VEYRON_OIDC_USERINFO_URL` when set,
  and rejected otherwise.

Browser console and serial sessions use a one-time ticket from `POST /api/v1/ws/ticket` (needs the `write`
role, valid 60 s, single use), so long-lived credentials never go in a WebSocket URL.

## Roles

Three roles, each including the one below it. The rule is by route and method, from
`src/api/auth_context.rs` (`/api/v1/veyron/*` is an alias of `/api/v1/*` and gets the same rules):

| Role | Can |
|---|---|
| `readonly` | `GET`, `HEAD` and `OPTIONS` on everything except the routes listed below; chat, search, intent and draft AI routes; `POST /mcp` |
| `write` | Create, change and delete VMs, snapshots, networks, alerts and the rest; also `GET` on `/vms/:ns/:name/{vnc,serial,tunnel}` (the interactive consoles and the TCP tunnel), `/diagnostics/*` and `/network/capture/*` |
| `admin` | The routes below, and user management |

Admin-only routes:

- `/auth/users*` for every method, including `GET`
- `GET /vms/:ns/:name/{screenshot,diagnostics/.../memory-dump}`, `POST /vms/:ns/:name/input`, and
  `/migrations/.../recovery`
- any non-read request to a path containing `/restore`
- non-read methods on `/dr/*`, `/tenants*`, `/platform/*` (except the public GuestKit binary download),
  `/kairon/fleet/*`, `/machine-images*`, `/soc/playbooks/*`
- `POST /gitops/sync`, `POST /catalog/sync`, `POST /clusters/:id/activate`, `POST /images/publish`
- `POST /nodes/:name/{cordon,uncordon,reboot}`, `DELETE /storage/orphans`
- `DELETE /image-store/:name`, `POST /vms/:ns/:name/capture`
- changes to a VM's network access (`/vms/:ns/:name/egress*`, `/egress/apply`, `/nodes/:name/egress-defaults`)
- node branches (`/nodes/:name/branches/...`) and agent-session approvals, grants and promotion
- `/ai/settings/llm` and `/ai/mcp-servers*` for non-read methods (a `GET` of either is `readonly`; neither returns secrets)

AI routes that only read or draft (`/ai/chat`, `/ai/chat/stream`, `/ai/search`, `/ai/intent/vm`,
`/ai/policies/draft`, `/mcp`) need only `readonly`; anything they want to change becomes a
proposal that needs the step's role to approve. See [ai.md](ai.md).

### Keys

- `VEYRON_API_KEY` is a single key with the `admin` role.
- `VEYRON_API_KEYS` holds several keys in the form `name:key:role`, comma-separated, for example
  `ops:k1:admin,ci:k2:write,viewer:k3:readonly`. A missing role means `readonly`.
- There is no key store behind the `veyron api-key-*` CLI commands: `api-key-create`, `api-key-delete` and
  `api-key-list` fail with an explanation instead of pretending. Add or revoke keys by editing `VEYRON_API_KEY` /
  `VEYRON_API_KEYS` (Helm: `auth.apiKey`, the `veyron-api-key` Secret) and restarting the API.
- Local console accounts are managed under **Security → Users & roles** (admin only), or through
  `/api/v1/auth/users`.

## Route families

| Area | Examples |
|---|---|
| VMs | `GET/POST /vms`, `POST /vms/:ns/:name/{start,stop,restart,pause,unpause,migrate}`, `DELETE /vms/:ns/:name` |
| Compute | `POST /vms/:ns/:name/hotplug`, `PUT /vms/:ns/:name/run-strategy`, `POST /vms/bulk` |
| Access | `GET/PUT/DELETE /vms/:ns/:name/expose` (SSH), `/rdp-expose` (RDP), `PUT /vms/:ns/:name/network/internet` |
| Console | `GET /vms/:ns/:name/vnc`, `/serial` (WebSocket, `write` role, ticket required) |
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
fails rather than truncating it. VMs with install media attached can't live-migrate; eject the
media first.

#### Eject install media

`POST /api/v1/vms/:ns/:name/cdroms/:cdrom/eject` (Write) removes that drive from the Machine's
`spec.cdroms`. The node agent then ejects the disc from the running guest; a stopped VM just comes
back without it. The drive is not re-inserted on the next start. The patch is pinned to the
Machine's `resourceVersion`, so a concurrent edit answers `409 CONFLICT`; retry. A name that isn't
in the spec answers `404 CDROM_NOT_FOUND`. When Kairon's Machine admission webhook is enabled, a
Kairon release that predates eject rejects the change with `422 EJECT_REJECTED`; without the
webhook an older kairon-node accepts the edit but leaves the disc in until the VM is recreated.
The answer is `202` with the drives still attached:

```bash
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" \
  "https://$HOST:30151/api/v1/vms/default/win01/cdroms/install/eject"
# {"namespace":"default","vm":"win01","ejected":"install","cdroms":["drivers"],…}
```

`GET /api/v1/vms` lists the remaining drive names as `cdroms` for Kairon VMs, and the console shows
an **Eject media** action on VMs that still have one. Removal only: attaching new media to an
existing VM is not supported. On KubeVirt the route answers `501 KAIRON_REQUIRED`.

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

Creating a snapshot (`POST /snapshots`, or `POST /snapshots/:ns/:vm/create`) takes the VM as
`vm_name` and an optional `name` (`snapshot_name` also works); a VM that doesn't exist returns
`404` and creates nothing. A VM that is meant to run reports `status: "Starting"` until the engine
confirms it, so a just-created VM never reads `Stopped`.

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
