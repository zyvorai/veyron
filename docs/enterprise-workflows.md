# Enterprise workflow center

This PR rebuilds enterprise workflow foundations on Veyron main
`a24f004112f6b64e7cf917a4c883884f217a1e04`. It is a new implementation,
not a byte-for-byte recovery of the lost draft.

## Delivered behavior

| Area | Behavior | Execution limits |
|---|---|---|
| Power operations | Durable start/stop requests, owner-scoped history, idempotency, CAS claims, per-VM locks, heartbeat, deadlines | Success means observed controller power state; no application health claim |
| Tenant admission | VM count, CPU, memory, GPU reservations across API replicas | Veyron `VmBackend::create` only; namespace quotas, not tenant-wide aggregates |
| Tenant authorization | Opt-in subject-to-namespace policy, fail closed, route allowlist | Scoped users cannot use broad cluster, AI/MCP, console, guest-exec, hotplug, raw create routes |
| Blueprint | Dependency graph validation, deterministic topological order, aggregate sizing | No VM creation, health-gated deployment, rollback or TTL cleanup |
| Placement | Live KubeVirt Node/Pod request accounting, headroom, labels, exclusion, taints | Kairon rejected until reservation accounting exists; no scheduler reservation or CPU/NUMA/CSI proof |
| Maintenance | Capacity-aware evacuation plan with sequential destination reservations | Caller-supplied evidence; no cordon, drain or migration execution |
| VMware migration | Conversion, VirtIO, boot, network/storage, fencing and application evidence checks | No h2kvm conversion or cutover execution |
| GPU | Device, IOMMU, driver, license and isolation evidence; reject whole-GPU live migration | No hardware probes, device assignment or driver installation |
| Recovery | Ordered timestamps, RPO/RTO, snapshot, fencing, isolation, boot/application checks | Caller-supplied evidence; no snapshot, restore or recovery drill execution |
| Fleet | Distinct cluster/context identities, readiness, replication and recovery evidence | No cross-cluster connection, replication or failover execution |
| Console ticket | Single use and original issuing caller/role | Tickets are in-process; clients need session affinity across replicas |

## API

All endpoints require existing Veyron authentication. Both `/api/v1` and
`/api/v1/veyron` handler prefixes expose enterprise endpoints.

| Method | Path after prefix | Role |
|---|---|---|
| GET | `/enterprise/capabilities` | ReadOnly |
| POST | `/enterprise/blueprints/validate` | ReadOnly |
| POST | `/enterprise/placement` | ReadOnly, unscoped |
| POST | `/enterprise/assess/{maintenance,migration,gpu,recovery,fleet}` | ReadOnly, unscoped |
| GET | `/enterprise/operations?namespace=apps` | ReadOnly; owner history or unscoped Admin |
| POST | `/enterprise/operations?namespace=apps` | Write |
| GET | `/enterprise/operations/:id?namespace=apps` | ReadOnly; owner or unscoped Admin |
| POST | `/enterprise/operations/:id/cancel?namespace=apps` | Write; owner or unscoped Admin; queued only |

POST power example:

```json
{"namespace":"apps","vm":"web","action":"start","idempotency_key":"change-2026-001"}
```

The query and body namespace must match. Reusing the same owner/namespace/key
returns the existing operation. A changed payload, Kubernetes context or VM
backend returns 409. Keep the key when retrying a network error; use a new key
for an intentionally new operation. The ConfigMap name hashes owner, namespace
and key, rather than exposing the key. All cluster errors are surfaced.

A successful POST means **accepted**, not completed. Refresh history until a
terminal state is reported. No automatic retry of a claimed mutation occurs.
Operations use a 15-minute deadline, a 15-second heartbeat, and a 90-second stale
threshold. Stale detection occurs on reads/retries. Queued work after restart is
resumed only when the owner retries the identical submission. ConfigMaps persist
in the workload namespace; there is no automatic record cleanup.

A separate per-VM ConfigMap lock prevents overlapping power workflows in this
API. Different keys submitted concurrently can yield a failed lock acquisition;
no power action is sent by that losing workflow. If the VM can't be read before the
power request is sent (for example it doesn't exist), the operation is `failed` and
the lock is released. Locks for ambiguous outcomes (`needs_review`, after the request
was sent) are retained for administrator review. Existing direct start/stop endpoints,
CLI, operators and other clients do not participate in this lock.

## Namespace authorization

Configure before startup:

```sh
export VEYRON_TENANT_SCOPES='{"team-a":["tenant-a"],"team-b":["tenant-b","staging-b"]}'
```

Subjects are existing authenticated API key names, local usernames or OIDC/JWT
subjects. The policy is cached on first use; restart to change it. When absent,
existing global authorization is retained. Invalid JSON or invalid namespace
assignments deny authenticated API calls. Under an enabled policy, unmapped
non-admins are denied. An unmapped Admin remains global; a mapped Admin is scoped.

Scoped routes deliberately use a small allowlist: explicit namespace VM list,
get, start/stop; enterprise capabilities; blueprint validation; owner operation
history/submission/cancellation. Blueprint namespace is checked by the handler.
Assessment endpoints expose cluster/supplied fleet evidence and are denied to
scoped callers. All other routes are denied, including aliases. UI-wide resource
polling may report forbidden requests for scoped users; use the allowed API
routes directly until a tenant-specific shell is available.

WebSocket tickets retain issuing subject and role. Scoped users cannot access
console routes. Never assign a scoped tenant an unlisted Admin subject.
Namespace labels alone do not create authorization; configure the policy.

## VM quota reservations

Managed namespaces also carry `veyron.io/vm-backend=kubevirt` or `kairon`.
Admission and bootstrap reject missing/mismatched backend labels; inspect existing
VM inventory before backfilling labels on older namespaces. Do not mix backend
objects inside a managed namespace. Namespace bootstrap checks ownership before
adopting an existing namespace and
uses the selected backend's object quota key. It reconciles the `tenant-quota`
ResourceQuota with CPU, memory, VM count and GPU requests. Deployment/Helm/remote manifests include ResourceQuota write
permissions for reconciliation. `gpu_quota` defaults to
zero. Existing tenant quotas must include `requests.nvidia.com/gpu` and the selected
backend's count key before creates proceed.

Admission reads the namespace label `veyron.io/tenant`, quota, VM inventory and
pending ledger. It reserves capacity with ConfigMap resourceVersion CAS, so API
replicas cannot simultaneously reserve the same remaining budget. Reservations
are retained after a failed/ambiguous create; they are removed only when a later
admission observes the VM in inventory, at which point actual sizing is counted.
Stopped VMs count toward quotas. Hotplug maximum memory/CPU in existing objects
is conservatively accounted. GPU claims in Kairon are conservatively counted as
devices; non-GPU DRA claims can over-reserve GPU budget.

Integer Kubernetes quantities only are accepted (CPU cores or `m`; memory integer
binary/decimal units). Fractions and scientific notation fail closed. Unresolved
instancetype sizing and unclassified host devices are rejected in managed
namespaces. Pod resource overhead and storage size are not included in this
VM admission budget. Kubernetes ResourceQuota still governs Pods and VM counts.
No cluster admission webhook is installed; direct Kubernetes/CLI/operator creates
and later hotplug/patches can bypass this Veyron budget. Use namespace RBAC and
external admission policies when quota enforcement across all writers is needed.

## Manual review

For `needs_review`, inspect the operation ConfigMap, recorded context/backend,
VM object and controller status. Confirm no worker is still running before
removing its matching `veyron-power-*` lock. Do not replay a workflow solely
because a heartbeat expired. After confirming the outcome, issue a new key only
if another action is required. Keep the operation record for audit.

For orphan admission reservations, inspect `veyron-vm-admission` in the namespace.
Confirm the corresponding VM was never created, the API request finished, and no
creator is still in flight before removing that pending entry using a
resourceVersion-guarded edit. There is no time-based reservation expiry.

## Build and verify

GuestKit is a required local path dependency (`guestkit/`, ignored by git); CI checks out
`zyvorai/zyvor-guestkit` at `GUESTKIT_REF` in `.github/workflows/ci.yml`.

```sh
npm --prefix frontend ci
npm --prefix frontend test
npm --prefix frontend run lint
npm --prefix frontend run build
cargo fmt --package veyron -- --check
RUST_MIN_STACK=8388608 cargo test --features kairon
cargo check --no-default-features
cargo clippy --all-targets -- -D warnings
cargo clippy --features kairon --all-targets -- -D warnings
make scripts-lint
```

The build embeds `frontend/dist`, so build the console before compiling Rust.
`tests/enterprise_concurrency.rs` races quota reservations against a mock API server.
Live cluster, VM boot, CSI snapshot, Windows/VSS, GPU and multi-cluster behaviour
require a lab and are not covered by unit tests or compilation.
