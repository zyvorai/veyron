<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0 -->
# Migrate VMs from VMware, Hyper-V and Azure

Existing VMs move to Kairon in two steps: [h2kvm](https://github.com/zyvorai/h2kvm) fixes and
converts the guest offline, then Veyron boots the disk as a Kairon `Machine`. No KubeVirt, no CDI
and no PVC upload are involved.

```
vSphere / ESXi / Azure / disk file
        │  h2kvm: export, offline repair (VirtIO drivers, fstab, bootloader), convert to qcow2
        ▼
  http(s) URL + SHA-256
        │  Veyron: POST /api/v1/imports  →  Kairon Machine (spec.image.source.httpURL)
        ▼
  kairon-node downloads into its digest-keyed cache, boots a per-VM overlay
```

## Before you start

- Kairon nodes need an image cache (`KAIRON_IMAGE_CACHE_DIR`, see [deploy.md](deploy.md#kairon-settings)).
- A Veyron API key with the **write** role.
- The converted disk must be reachable over HTTP(S) from the Kairon node. h2kvm can serve it itself.

## With h2kvm (recommended)

h2kvm 1.5.0 and later have a Kairon target:

```bash
h2kvmctl --config vmware-web01.yaml \
  --deploy-kairon \
  --kairon-veyron-url https://<veyron-host>:30151 --kairon-insecure \
  --kairon-namespace prod --kairon-cpus 4 --kairon-memory 8Gi \
  --kairon-serve 0.0.0.0:8099 --kairon-advertise-url http://<this-host>:8099 \
  --kairon-source-hypervisor vmware
```

h2kvm hashes the fixed disk, serves it at an unguessable path, asks Veyron to create the Machine
and waits until it is `Running`. If you already published the disk, pass `--kairon-image-url`
instead of `--kairon-serve`. All flags: h2kvm's `docs/deployment/kairon-deployment.md`.

## With the API or the console

Any qcow2, raw, OVA, VMDK, VHD or VHDX works:

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/imports" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"web01","namespace":"prod",
       "image_url":"https://files.example/web01.qcow2","sha256":"<sha256sum of the file>",
       "cpus":4,"memory":"8Gi",
       "forwards":[{"host_port":30022,"guest_port":22}],
       "source":{"hypervisor":"vmware","vm":"dc1/web01"}}'
```

| Field | Meaning |
|---|---|
| `format` | `qcow2` (default), `raw`, `ova`, `vmdk`, `vhd`, `vhdx`. Non-qcow2 formats are converted on the node |
| `repair` | Let FluxVM run its own offline VirtIO repair (for disks that did not go through h2kvm) |
| `secure_boot`, `tpm` | For Windows 11 and Windows Server 2022 or later |
| `forwards` | Node port forwards. Kairon applies them only at creation, so add SSH/RDP here if you need them |
| `start` | `false` creates the VM stopped |

### Preflight: `POST /api/v1/imports/validate`

Dry-run check of an import request; nothing is created. The body takes the same fields as
`POST /imports` (all optional here, so a missing field is reported instead of rejected) plus
`guest_os` (for example `windows-2022`) and `probe`. The response is always `200` with a report:

```json
{"success": true, "data": {
  "valid": true,
  "errors":   [{"field": "sha256", "code": "INVALID_SHA256", "message": "..."}],
  "warnings": [{"field": "format", "code": "CONVERSION_ON_NODE", "message": "..."}],
  "normalized": {"name": "web01", "namespace": null, "image_url": "https://...", "sha256": "sha256:ab..",
                 "format": "qcow2", "cpus": 2, "memory": "4Gi"},
  "probe": {"reachable": true, "status": 200, "content_length": 490000000, "error": null},
  "size_bytes": 490000000
}}
```

It checks the name, SHA-256 syntax, format, CPUs and memory, and applies a URL policy: only `http(s)`,
no credentials in the URL, and no loopback, link-local (cloud metadata) or unspecified host. Private
LAN addresses are allowed, since h2kvm serves from there. Warning codes: `PLAINTEXT_HTTP`, `PRIVATE_HOST`,
`CONVERSION_ON_NODE`, `WINDOWS_SECURE_BOOT_TPM` (Windows 11 and Server 2022 or later without `secure_boot`
and `tpm`), `WINDOWS_SECURE_BOOT_TPM_HINT` (the source VM name looks like Windows and `guest_os` is unset),
`IMAGE_CACHE_UNCONFIRMED` (`KAIRON_IMAGE_CACHE_DIR` is not set in Veyron's environment; Veyron cannot see the
nodes' own settings), and, with `"probe": true`, `PROBE_UNREACHABLE`, `PROBE_REDIRECT`, `SIZE_UNKNOWN`.

With `"probe": true` Veyron sends one `HEAD` (8 s timeout, no redirects, connection pinned to the resolved
address) to read `Content-Length`. That shows reachability from Veyron, not from the Kairon node, so an
unreachable probe is a warning, not an error.

### Network and storage mapping profiles

A VM that sat on vSphere port group `DMZ` should not land on Kairon's user-mode network. A **mapping profile** is
a named table, stored as a ConfigMap (`import-mapping-<name>`, label `veyron.io/type=import-mapping`, in Veyron's
namespace), that translates source names to Kairon:

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/import-mappings" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"dc1","description":"DC1 vSphere",
       "networks":{"VM Network":{"mode":"macvtap","bridge":"eth1"},
                   "DMZ":{"mode":"tap","bridge":"br-dmz"}},
       "datastores":{"ds-ssd":"fast"},
       "default_pool":"bulk"}'

curl -sk -X POST "https://$HOST:30151/api/v1/imports" ... \
  -d '{"name":"web01","image_url":"...","sha256":"...","mapping":"dc1",
       "source":{"hypervisor":"vmware","network":"DMZ","datastore":"ds-ssd"}}'
```

- `networks` maps a source network to `{"mode": "tap" | "macvtap" | "user", "bridge": "..."}`; `tap` and `macvtap`
  need `bridge` (the host bridge, or the parent interface for macvtap). The result is `spec.network.mode` and
  `spec.network.bridge` on the Machine. `forwards` still apply.
- `datastores` maps a source datastore to a pool name. Kairon's `Machine` has no pool field, so the pool is
  recorded on the Machine as the annotation `veyron.io/import-storage-pool` and returned as `storage_pool`; it does
  not place the disk by itself.
- `default_network` and `default_pool` are used for names the profile does not list. With no default, an
  unlisted `source.network` or `source.datastore` is refused (`400 UNMAPPED_SOURCE`) instead of silently landing
  on the user-mode network. A request with no `source.network` gets no network change unless the profile has a
  `default_network`.
- An unknown profile is `400 MAPPING_NOT_FOUND`. `POST /imports/validate` accepts `mapping` and reports the same
  two problems as errors.
- Without `mapping`, `source.network` and `source.datastore` are only recorded as annotations.
- Import rows return `mapping`, `source_network`, `source_datastore`, `storage_pool` and `network`.

`PUT /import-mappings/:name` replaces a profile (the path name wins), `DELETE` removes it; existing Machines keep
what was applied. The Veyron ServiceAccount already has ConfigMap write access.

### Lifecycle: phase, retry, delete

Every import row (`GET /imports`, `GET /imports/:ns/:name`, and the response to `POST /imports`) now carries:

| Field | Meaning |
|---|---|
| `import_id` | `imp-xxxxxxxx`, assigned at creation and kept across retries. Also the annotation `veyron.io/import-id` |
| `phase` | `pending`, `downloading`, `converting`, `running`, `stopped`, `failed` or `terminating` |
| `terminal` | `true` for `running`, `stopped`, `failed`: nothing changes without an operator action |
| `retryable` | `true` when `phase` is `failed` |
| `retries` | How many times the import was retried |
| `message` | Kairon's status message, when there is one |

`status` (Kairon's own phase string) is unchanged. `phase` is derived from it: `Running` is `running`,
`Halted`/`Stopped`/`Paused` is `stopped`, any phase or condition reason containing `fail` or `error` is `failed`, and a
set deletion timestamp is `terminating`. Kairon has no dedicated download phase, so `downloading` and `converting`
are guesses from the status message and only ever refine `pending`.

```bash
# Recreate a failed import from its own spec (same import_id, retries + 1). 202 with the new row.
curl -sk -X POST "https://$HOST:30151/api/v1/imports/prod/web01/retry" -H "X-API-Key: $VEYRON_API_KEY"
# Delete an imported VM. 202; 404 NOT_AN_IMPORT for a VM that was not created by an import.
curl -sk -X DELETE "https://$HOST:30151/api/v1/imports/prod/web01" -H "X-API-Key: $VEYRON_API_KEY"
```

Kairon cannot re-run a failed download in place, so retry deletes the Machine, waits up to 15 s for it to
disappear, and creates it again. It refuses an import that is not `failed` with `409 NOT_RETRYABLE` unless you pass
`?force=true`, and answers `409 STILL_TERMINATING` if the old Machine is held by a finalizer (the delete was issued;
call retry again). Delete is idempotent while the VM is already terminating.

In the console, **Images → New** opens the same import form (name, disk URL, SHA-256, format).

`GET /api/v1/imports?namespace=all` lists imported VMs with their source hypervisor, source VM and
status. They are ordinary VMs afterwards: console, snapshots, backups, stop/start and the rest
of day-2 operations work on them.

## Events, webhooks and audit

**Phase-change webhook.** Add `import.phase_changed` (or `import.*`, or `*`) to a webhook's `events`
(`POST /api/v1/webhooks`) to receive one POST per import phase change, signed like every other Veyron webhook:

```json
{"event": "import.phase_changed", "timestamp": "2026-11-19T10:15:00Z", "source": "veyron",
 "data": {"import_id": "imp-3f9a1c02", "name": "web01", "namespace": "prod",
          "phase": "running", "previous_phase": "pending", "message": ""}}
```

Veyron has no controller loop watching Machines, so a change is noticed when something reads the import:
`GET /imports`, `GET /imports/:ns/:name` (the console polls these), retry or delete. The last phase an observer saw
is stored in the annotation `veyron.io/import-last-phase`; a different current phase emits one event and updates it.
Consequences: nothing fires while nobody is looking, several transitions between two reads arrive as one, and two
readers racing on the same change can both emit it. Delivery is best effort, in the background, and uses the
existing webhook URL policy (HTTPS, no private or loopback addresses). Retry sends `failed` to `pending`; delete sends
`terminating`.

**Audit.** Create, delete, retry and each batch row write an entry to the SOC event log (`/soc/events`,
`/soc/audit`) with `action` `import.create`, `import.delete`, `import.retry` or `import.batch-create`, the caller as
`actor`, `target` `namespace/name`, and the label `import_id`, so an audit entry joins to its `GET /imports` row and
its webhooks. Failures are recorded with `outcome: failure` and the error code in the `detail` label. Dry runs and
validation are not audited.

## Batch and wave imports

`POST /api/v1/imports/batch` lands many VMs in one call, in ordered **waves** (database tier first, then app tier, then
web tier). Each VM carries the same fields as `POST /imports` plus an optional `wave` (default 1):

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/imports/batch" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"namespace":"prod","mapping":"dc1","concurrency":4,"stop_on_failure":true,
       "vms":[{"name":"db01","wave":1,"image_url":"https://files.example/db01.qcow2","sha256":"...",
               "source":{"network":"DMZ","datastore":"ds-ssd"}},
              {"name":"app01","wave":2,"image_url":"...","sha256":"..."}]}'
```

| Field | Meaning |
|---|---|
| `vms` | Up to 200 import requests, each with an optional `wave` |
| `csv` | Alternative to `vms`: CSV text, see below |
| `namespace`, `mapping` | Defaults for rows that do not set their own |
| `concurrency` | Machines created at once within a wave. Default 4, capped at 16 |
| `stop_on_failure` | Default `true`: if any VM in a wave fails, later waves are `skipped` |
| `dry_run` | Validate and build every Machine (including mappings), create nothing |

Waves run in ascending order and a wave starts only after the previous one finished. The gate is **creation**, not
boot: "created" means Kubernetes accepted the Machine, so watch `GET /imports` for `phase: running` before
cutting over. Every VM gets its own result; a bad row never hides the others. The response is `200` when every VM
landed and `207` when the plan ran but some VMs failed or were skipped. A malformed plan as a whole (empty, over
200 VMs, bad CSV, both `vms` and `csv`) is `400`.

```json
{"success": false, "data": {
  "dry_run": false, "total": 3, "succeeded": 1, "failed": 1, "skipped": 1, "concurrency": 4,
  "waves": [{"wave": 1, "created": 1, "failed": 0, "skipped": 0},
            {"wave": 2, "created": 0, "failed": 1, "skipped": 0},
            {"wave": 3, "created": 0, "failed": 0, "skipped": 1}],
  "results": [
    {"index": 0, "name": "db01", "namespace": "prod", "wave": 1, "status": "created", "import_id": "imp-3f9a1c02", "error": null},
    {"index": 1, "name": "app01", "namespace": "prod", "wave": 2, "status": "failed", "import_id": null,
     "error": {"code": "ALREADY_EXISTS", "message": "VM prod/app01 already exists"}},
    {"index": 2, "name": "web01", "namespace": "prod", "wave": 3, "status": "skipped", "import_id": null,
     "error": {"code": "WAVE_BLOCKED", "message": "an earlier wave failed and stop_on_failure is set"}}]}}
```

`status` is `created`, `valid` (dry run), `failed` or `skipped`. Per-VM error codes: `INVALID_REQUEST`,
`MAPPING_NOT_FOUND`, `UNMAPPED_SOURCE`, `ALREADY_EXISTS`, `DUPLICATE` (same namespace and name twice in the plan),
`IMPORT_FAILED`, `WAVE_BLOCKED`.

**CSV.** A header row, then one VM per line. Columns, in any order: `name`, `image_url`, `sha256`, `namespace`,
`format`, `cpus`, `memory`, `wave`, `mapping`, `repair`, `secure_boot`, `tpm`, `start`, `forwards`
(`hostPort:guestPort` pairs separated by `;`), and the source fields `hypervisor`, `vm`, `tool`, `network`,
`datastore`. Empty cells are omitted, fields may be double-quoted, booleans are `true/false/yes/no/1/0`. Pass it as
`"csv": "..."` in the batch body, or `POST /imports/csv-to-json` with the CSV as the raw request body to get the
equivalent JSON plan (`{"data": {"vms": [...]}}`) without creating anything.

## Things to know

- **Stop keeps the disk.** Stopping a Kairon VM powers it off (`Halted`); start boots the same disk.
- **Snapshots before cutover.** Imported VMs have no PVC, so a snapshot is a VM-state checkpoint on
  the node; restoring it rewinds the VM in place.
- **Reaching the VM.** A guest on Kairon's default user-mode network (`10.0.2.15`) is reachable only
  through the `forwards` given at import, or by importing it onto a bridged (`tap`/`macvtap`) network.
- **Windows RDP.** `POST /vms/:ns/:name/guest-agent/enable-rdp` works once the QEMU guest agent is
  running in the guest; see [windows.md](windows.md).

Verified end to end on 2026-10-07: h2kvm served a 490 MB qcow2, Veyron created the Machine, and
Kairon downloaded, verified and booted it to `Running` in about 30 seconds.
