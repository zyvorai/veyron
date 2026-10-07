<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Windows guests

Veyron runs Windows Server and Windows desktop VMs with the right hardware shape, unattended setup,
golden images and safe Remote Desktop.

## Templates

The `windows*` templates (`src/templates/mod.rs`) set:

- UEFI firmware, plus TPM on Windows Server 2022 and Windows 11;
- Hyper-V enlightenments and a Windows-friendly clock;
- virtio disk and NIC, with the VirtIO driver CD attached and pinned by digest in
  `scripts/cluster/versions.env`;
- RNG, a USB tablet and IO threads.

Windows templates boot a blank root disk on purpose. The same template serves both an ISO install
and a clone from a golden image.

## Golden images

The model is: install once, seal with sysprep, publish a versioned image, and clone a private disk
for every VM.

1. **Upload** the ISO or QCOW2: `POST /api/v1/images/upload`.
2. **Install and seal** a builder VM: load the `viostor` driver from the VirtIO CD when no disk
   shows, install the drivers and the QEMU guest agent, then run
   `Sysprep.exe /generalize /shutdown /oobe /mode:vm`.
3. **Publish** (admin only): `POST /api/v1/images/publish`. Veyron refuses an image that hasn't
   finished importing.
4. **Create VMs** from the catalog:

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/vms" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"win01","namespace":"customer-a","template":"windows-2022","disk_size":"150Gi",
       "image":{"name":"windows-server-2022","namespace":"vm-images"},
       "sysprep_secret":"win01-sysprep"}'
```

VMs never share a disk; each one gets its own clone. Publishing a new version changes what future
clones receive and leaves existing VMs alone.

The upload and publish routes above use CDI DataVolumes and DataSources, so they only work with the
legacy `VEYRON_VM_BACKEND=kubevirt`; on Kairon they return `501 KUBEVIRT_ONLY`.

On Kairon, a Windows image is an HTTP(S) disk pinned by SHA-256. Publish the sealed QCOW2 (or a
VMDK/VHDX/OVA) anywhere the nodes can reach, then create the VM with `POST /api/v1/imports`. Kairon
caches the disk once per node and gives every VM its own overlay, so VMs still never share writes:

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/imports" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"win01","namespace":"customer-a","image_url":"https://images.example/ws2022-v3.qcow2",
       "sha256":"<sha256sum of the file>","cpus":4,"memory":"8Gi","secure_boot":true,"tpm":true}'
```

Migrating existing Windows VMs from VMware or Hyper-V works the same way through h2kvm, which
injects the VirtIO drivers offline first; see [migrate.md](migrate.md).

## Installing from an ISO on Kairon

Kairon attaches ISOs as SATA CD-ROMs, which Windows Setup reads without extra drivers. Upload the
Windows ISO and the virtio-win driver ISO once, then install as many VMs as you like:

```bash
for f in ws2022.iso virtio-win.iso; do
  curl -sk -X PUT -H "X-API-Key: $VEYRON_API_KEY" --data-binary @"$f" \
    "https://$HOST:30151/api/v1/image-store/$f"
done
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/machine-images" -d '{"name":"ws2022-iso","from_store":"ws2022.iso","os":"windows"}'
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/machine-images" -d '{"name":"virtio-win","from_store":"virtio-win.iso","os":"windows"}'

curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/vms" \
  -d '{"name":"win01","template":"windows-2022","iso":"ws2022-iso","disk_size":"80Gi"}'
```

The VM starts with a blank 80 GiB disk, the Windows ISO and (because the template is Windows and a
`virtio-win` image exists) the driver CD. Open the console, load `viostor` from the driver CD when
Setup shows no disk, and finish the install. OVMF boots the installed disk afterwards, so the ISO
can stay attached, but a VM with install media can't live-migrate. To make the result a golden
image, sysprep it and publish its disk as a `MachineImage` of kind `disk`.

The console does the same: **Images → New → Upload a file**, then **VMs → New → Boot from →
Install from ISO**.

## Unattended setup and domain join

`sysprep_secret` names a Secret with an `autounattend.xml` key, mounted as CD-ROM media because
Windows Setup only reads answer files from optical drives. A missing key fails the request instead
of booting into an interactive prompt.

For domain join, put the domain, OU and a least-privilege join account in a Secret. Veyron renders
it into the unattend `<Identification>` block, never into cloud-init, so the password stays out of
in-guest logs. See [`examples/windows-domain-join/`](../examples/windows-domain-join).

## Remote Desktop

```bash
# Turn on RDP inside a running guest (guest agent must be connected; on Kairon this runs
# the same PowerShell through the kairon-node relay)
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" \
  "https://$HOST:30151/api/v1/vms/default/win01/guest-agent/enable-rdp"

# Reach it from inside the cluster network
curl -sk -X PUT -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/vms/default/win01/rdp-expose" -d '{"service_type":"ClusterIP"}'
```

On Kairon, `rdp-expose` needs a port forward declared when the VM was created (`forwards` on
`POST /imports`) or a bridged guest with a routable address. A guest on Kairon's default user-mode
network (`10.0.2.15`) gets a `409` explaining why.

Public RDP (`NodePort` or `LoadBalancer`) is refused with `403` unless `VEYRON_ALLOW_PUBLIC_RDP=1`:
a bare port 3389 puts Windows sign-in on the network with no gateway, MFA or TLS. Use a VPN, a
zero-trust gateway or `kubectl port-forward`. Connect with Microsoft Remote Desktop, or
`sdl-freerdp` on macOS.

## Packer pipeline

For fleets, build images in CI with Packer and QEMU, publish the QCOW2 and manage VMs with
GitOps. Reference manifests and a config-drive patch script live in
[`examples/windows-kubevirt-gitops/`](../examples/windows-kubevirt-gitops), and an end-to-end
example is in [`examples/windows-golden-image/`](../examples/windows-golden-image).
