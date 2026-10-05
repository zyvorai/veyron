<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# GPU virtual machines

Give a VM a whole NVIDIA GPU through PCI passthrough. This works with any NVIDIA card, consumer
GeForce included, and needs no license: the VM owns the physical device.

> **A passthrough GPU VM never live-migrates.** The device state lives on the card. All migration
> routes return `409` with a list of blockers and suggest a cold move: stop the VM, then start it on
> another node. `force: true` overrides this, at your own risk.

## 1. Prepare the host

```bash
./scripts/cluster/prepare-gpu-node.sh <host> <user>           # dry run: IOMMU and group isolation
./scripts/cluster/prepare-gpu-node.sh <host> <user> --apply   # bind to vfio-pci, then reboot
./scripts/cluster/preflight-node.sh --gpu <host> <user>       # every NVIDIA function bound to vfio-pci
```

The script binds the GPU and its audio and USB sibling functions together. It checks that the IOMMU
group isolates the card, and prints the BIOS and kernel command-line fix if IOMMU is off; it never
edits the bootloader itself. It uses `softdep` files rather than hard blacklists, so deleting one
file hands the card back to the host.

## 2. Advertise the GPU to the cluster

```bash
./scripts/cluster/enable-gpu-passthrough.sh <host> <user> --apply
```

Afterwards the node's allocatable resources list `nvidia.com/gpu`, and
`GET /api/v1/platform/capabilities` reports `day2_ops.gpu_passthrough: true` with a `gpus` section.
`GET /api/v1/gpus` lists the inventory per node (passthrough, MIG, vGPU).

With Kairon, GPUs reach the VM as Kubernetes DRA device claims on the `Machine`.

## 3. Create a GPU VM

```bash
curl -sk -X POST https://<host>:30151/api/v1/vms \
  -H "X-API-Key: $VEYRON_API_KEY" -H 'Content-Type: application/json' \
  -d '{"name":"gpu-vm","template":"ubuntu-24.04","gpu":{"count":1}}'

veyron create gpu-vm --template ubuntu-24.04 --gpu 1
veyron create gpu-vm --template ubuntu-24.04 --gpu nvidia.com/GRID_T4-2Q:2
```

- `gpu` takes `{count, resource_name, vgpu_profile}`; `resource_name` defaults to `nvidia.com/gpu`.
- If no node advertises the resource, the API returns `422 GPU_RESOURCE_UNAVAILABLE` and lists what
  is available. Pass `"force": true` when a node is about to join.
- For NVIDIA resources Veyron hides KVM from the guest, which avoids driver Error 43.

Inside the guest, install the standard NVIDIA driver and run `nvidia-smi`.

## Troubleshooting

| Symptom | Fix |
|---|---|
| VM pending with `Insufficient nvidia.com/gpu` | The PCI ids don't match `lspci -nn`, or the card isn't on vfio-pci |
| `no available resource for device` | Another VM holds the card; one card serves one VM |
| Host console goes dark after prep | Expected: the host gave its only GPU away. Keep IPMI or SSH access |
| Migration returns 409 | By design. Use a cold move |

**vGPU (licensed, datacenter cards)** is the next phase. Live migration of vGPU VMs stays off
unless `VEYRON_VGPU_LIVE_MIGRATION=1` is set after verifying that the host stack supports it.
