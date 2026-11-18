<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
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

In the console, **Images → New** opens the same import form (name, disk URL, SHA-256, format).

`GET /api/v1/imports?namespace=all` lists imported VMs with their source hypervisor, source VM and
status. They are ordinary VMs afterwards: console, snapshots, backups, stop/start and the rest
of day-2 operations work on them.

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
