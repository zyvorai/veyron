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

With Kairon, images come from the Machine image catalog (HTTP or OCI sources, pinned by digest).
Today's build uses CDI DataVolumes and DataSources, which needs a shared StorageClass;
`./scripts/preflight-veyron-remote.sh` reports whether `windows_golden_images` is available.

## Unattended setup and domain join

`sysprep_secret` names a Secret with an `autounattend.xml` key, mounted as CD-ROM media because
Windows Setup only reads answer files from optical drives. A missing key fails the request instead
of booting into an interactive prompt.

For domain join, put the domain, OU and a least-privilege join account in a Secret. Veyron renders
it into the unattend `<Identification>` block, never into cloud-init, so the password stays out of
in-guest logs. See [`examples/windows-domain-join/`](../examples/windows-domain-join).

## Remote Desktop

```bash
# Turn on RDP inside a running guest (guest agent must be connected)
curl -sk -X POST -H "X-API-Key: $VEYRON_API_KEY" \
  "https://$HOST:30151/api/v1/vms/default/win01/guest-agent/enable-rdp"

# Reach it from inside the cluster network
curl -sk -X PUT -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  "https://$HOST:30151/api/v1/vms/default/win01/rdp-expose" -d '{"service_type":"ClusterIP"}'
```

Public RDP (`NodePort` or `LoadBalancer`) is refused with `403` unless `VEYRON_ALLOW_PUBLIC_RDP=1`:
a bare port 3389 puts Windows sign-in on the network with no gateway, MFA or TLS. Use a VPN, a
zero-trust gateway or `kubectl port-forward`. Connect with Microsoft Remote Desktop, or
`sdl-freerdp` on macOS.

## Packer pipeline

For fleets, build images in CI with Packer and QEMU, publish the QCOW2 and manage VMs with
GitOps. Reference manifests and a config-drive patch script live in
[`examples/windows-kubevirt-gitops/`](../examples/windows-kubevirt-gitops), and an end-to-end
example is in [`examples/windows-golden-image/`](../examples/windows-golden-image).
