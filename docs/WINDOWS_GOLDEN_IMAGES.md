<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
# Windows golden images through Veyron

The end-to-end path from a Microsoft ISO to a fleet of sysprepped Windows VMs,
driven by the Veyron API. This replaces the "build it by hand with `virtctl`"
limitation called out in [WINDOWS_KUBEVIRT_PRODUCTION.md](WINDOWS_KUBEVIRT_PRODUCTION.md).

## The model

```
ISO  ──upload──►  DataVolume  ──install+sysprep──►  builder VM
                                                        │
                                                     publish
                                                        ▼
                              golden PVC (immutable, versioned)
                                        ▲
                                   DataSource  ◄── the stable catalog handle
                                        │
                          VMs clone from it (dataVolumeTemplates)
```

Two rules make this safe:

- **VMs never share a disk.** A `dataSource` disk emits a per-VM
  `dataVolumeTemplates` entry, so each VM CDI-clones its own copy, garbage-collected
  with the VM. Referencing a PVC or DataVolume *by name* (`pvc` / `dataVolume` disk
  types) points two VMs at one disk and corrupts it — only use those for a disk that
  genuinely is shared and read-only.
- **The DataSource is the only thing VMs name.** Golden PVCs are immutable and
  versioned (`windows-2022-golden-2026-07`). Publishing a new version repoints the
  DataSource, so the next clone picks it up and existing VMs are untouched.

## 0. Preconditions

```bash
./scripts/preflight-veyron-remote.sh <host> 30151
```

`windows_golden_images` must be **true**. It requires CDI plus a **shared**
(non-node-local) StorageClass — on `local-path`, every VM disk is pinned to one node.
If the gate fails, run `./scripts/cluster/adapt-existing-cluster.sh <host> <user> --apply`.

## 1. Upload the ISO

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/images/upload" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"win2022-iso","namespace":"vm-images","size":"8Gi",
       "storage_class":"zyvor-rbd-prod","volume_mode":"filesystem"}'
```

Returns a CDI upload `token` and the `upload_proxy` URL. PUT the ISO bytes there
with `Authorization: Bearer <token>`. (`virtctl image-upload` remains equivalent and
is fine to use instead.)

## 2. Build and seal the builder VM

Create a Windows VM with the ISO attached, install Windows (load the **`viostor`**
driver from the VirtIO CD when no disk appears — the root disk is virtio), install
the VirtIO drivers and QEMU Guest Agent, then sysprep:

```powershell
C:\Windows\System32\Sysprep\Sysprep.exe /generalize /shutdown /oobe /mode:vm
```

Use `runStrategy: RerunOnFailure` on the builder so it does **not** restart after
sysprep's clean shutdown. Do not start it again once generalized.

The VirtIO driver CD is attached automatically to Windows VMs and is pinned **by
digest** (`scripts/cluster/versions.env`), so the driver set cannot shift under a
running fleet.

## 3. Publish

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/images/publish" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"win2022-builder-root","namespace":"vm-images",
       "data_source":"windows-server-2022","version":"2026-07"}'
```

Publish **refuses a DataVolume that is not `Succeeded`** — shipping a half-imported
disk would hand every cloned VM a corrupt image. `force: true` overrides.

This route is **Admin**-only: it repoints the DataSource that every future clone uses.

Check the catalog: `GET /api/v1/images/datasources`.

## 4. Create VMs from the image

```bash
curl -sk -X POST "https://$HOST:30151/api/v1/vms" \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"win01","namespace":"customer-a","template":"windows-2022",
       "disk_size":"150Gi",
       "image":{"name":"windows-server-2022","namespace":"vm-images"},
       "sysprep_secret":"win01-sysprep"}'
```

- `image` clones the root disk from the catalog instead of booting a blank disk.
- `sysprep_secret` names a Secret with an **`autounattend.xml`** key. It is mounted as
  a sysprep **CD-ROM** — Windows Setup only reads answer files from optical media.
  The operator now **fails the reconcile** if the Secret is missing that key, rather
  than booting a VM to an interactive prompt.

Per-VM unattend Secret:

```bash
kubectl create secret generic win01-sysprep -n customer-a \
  --from-file=autounattend.xml=./win01-autounattend.xml
```

Never commit answer files — they carry the local administrator password.

## 5. Cross-namespace clone RBAC (required)

CDI refuses a cross-namespace clone unless the creating ServiceAccount can `create`
on `datavolumes/source` in the **source** namespace. Without it the DataVolume hangs
in a permission error and the VM never gets a disk.

```bash
kubectl apply -f deploy/k8s/bootstrap/cdi-golden-image-cloner.yaml
```

Copy the RoleBinding per tenant namespace.

## Rolling a new image version

1. Build a new builder VM, seal it, publish with a new `version`.
2. `POST /images/publish` with the same `data_source` → the DataSource repoints.
3. Existing VMs keep their disks. New VMs clone the new version.
4. To roll back, publish the previous PVC under the same `data_source`.

## Domain join

Set `windows.domainJoinSecretRef` on a **VeyronVM** pointing at a Secret whose key
holds JSON:

```json
{
  "domain":   "corp.example.com",
  "ou":       "OU=Servers,DC=corp,DC=example,DC=com",
  "username": "CORP\\joinsvc",
  "password": "..."
}
```

The operator renders a Windows unattend **`<Identification>`** block into an
operator-managed Secret (`<vm>-veyron-sysprep`) and mounts it as sysprep media.

**This is not cosmetic.** The join credential used to be rendered into a PowerShell
`Add-Computer` block in cloud-init userData. Cloudbase-Init *logs the userdata script
it runs*, so the domain password ended up in plaintext in
`cloudbase-init.log` inside the guest — and in the config-drive volume, permanently.
Via `<Identification>`, Windows consumes the credential during the specialize pass and
replaces it with `*SENSITIVE*DATA*DELETED*` in the cached
`C:\Windows\Panther\unattend.xml`.

The password must still reach Windows somehow — there is no mechanism that avoids
that. **Use a least-privilege delegated join account** that can join computers to the
target OU and nothing else.

If you also set `sysprepSecretRef`, your answer file is **merged**, not replaced: the
join is injected into your existing `specialize` pass. The operator refuses (rather
than guessing) if your file already contains an `UnattendedJoin` component or is not a
recognizable unattend document.

## Known gaps

- **Live migration** needs ≥2 nodes and an RWX StorageClass; the gate reports
  `live_migration: false` until both hold.
- **Persistent TPM/EFI** (Windows 11 BitLocker) needs the `VMPersistentState` feature
  gate *and* `vmStateStorageClass`. `adapt-existing-cluster.sh` sets both.
