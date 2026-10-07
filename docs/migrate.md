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
