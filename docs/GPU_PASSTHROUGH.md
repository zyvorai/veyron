# GPU Passthrough (Phase 1)

Whole-GPU PCI passthrough into KubeVirt VMs, managed through Veyron. Works on
**any** NVIDIA card (consumer GeForce included) with **no licensing** — the VM
owns the physical GPU.

> **The one rule that shapes everything: a passthrough-GPU VM can NEVER
> live-migrate.** The device state lives on the physical card. Veyron's API
> refuses migration with a structured `409` (`GPU_PASSTHROUGH_NOT_MIGRATABLE`)
> and suggests the cold move: stop the VM, start it with node placement.
> True live migration needs NVIDIA **vGPU** — licensed, datacenter GPUs only
> (Phase 2, see [NEO_CLOUD_GPU_ROADMAP.md](NEO_CLOUD_GPU_ROADMAP.md)).

## Pipeline

```
BIOS (VT-d/IOMMU on)
  → prepare-gpu-node.sh  (vfio-pci binding, reboot)
  → preflight-node.sh --gpu  (verify)
  → enable-gpu-passthrough.sh  (KubeVirt feature gates + permittedHostDevices)
  → node advertises nvidia.com/gpu
  → POST /api/v1/vms {"gpu": {"count": 1}}
```

## 1. Host prep

```bash
# Dry run first (prints the plan, checks IOMMU + group isolation):
./scripts/cluster/prepare-gpu-node.sh <host> <user>
# Commit + reboot:
./scripts/cluster/prepare-gpu-node.sh <host> <user> --apply
ssh <user>@<host> reboot
```

What it does:

- Detects every NVIDIA PCI function (`lspci -Dnn -d 10de:`) — the GPU **and**
  its audio/USB sibling functions; all same-card functions must go to vfio
  together (`V9S_GPU_INCLUDE_AUDIO=0` to exclude).
- Verifies IOMMU. If off, it prints the exact fix (BIOS VT-d/AMD-Vi +
  `intel_iommu=on iommu=pt` / `amd_iommu=on` on the kernel cmdline) and **never
  edits the bootloader itself**.
- Checks IOMMU **group isolation**: a group is passed through whole, so a GPU
  sharing its group with, say, a SATA controller drags it into the VM. Try
  another PCIe slot first. ACS-override kernel patches exist but collapse the
  isolation IOMMU is meant to provide — do not use them on multi-tenant hosts.
- `--apply` writes `/etc/modprobe.d/vfio-veyron.conf`
  (`options vfio-pci ids=10de:xxxx,…` + `softdep nouveau/nvidia pre: vfio-pci`)
  and `/etc/modules-load.d/vfio-veyron.conf`, then regenerates the initramfs.
  Softdeps (not hard blacklists) mean deleting that one file later hands the
  GPU back to host drivers — the path to a Phase-2 vGPU host.

Verify after the reboot:

```bash
./scripts/cluster/preflight-node.sh --gpu <host> <user>
# every NVIDIA function: "bound to vfio-pci"
```

## 2. Cluster config

```bash
./scripts/cluster/enable-gpu-passthrough.sh <host> <user>          # dry-run
./scripts/cluster/enable-gpu-passthrough.sh <host> <user> --apply
```

- Merges KubeVirt feature gates `GPU`, `HostDevices` (merge — never clobbers
  existing gates).
- Merges `spec.configuration.permittedHostDevices.pciHostDevices`:
  `{pciVendorSelector: "10DE:XXXX", resourceName: "nvidia.com/gpu"}` per
  detected id (override with `V9S_GPU_PCI_IDS` / `V9S_GPU_RESOURCE_NAME`).
  Keyed on the selector — re-runs are idempotent.
- Phase 1 needs **no extra device plugin**: KubeVirt's built-in PCI host-device
  plugin serves the resource. `V9S_INSTALL_NVIDIA_KUBEVIRT_DP=1` additionally
  deploys the NVIDIA kubevirt-gpu-device-plugin (image pinned in
  `scripts/cluster/versions.env`) on `veyron.io/gpu-passthrough=true` nodes —
  only needed on the road to vGPU/mdev.
- Also available via the single entry point:
  `V9S_INSTALL_GPU_PASSTHROUGH=1 ./scripts/cluster/install-cluster-prereqs.sh`.

Verify: `kubectl get node <n> -o jsonpath='{.status.allocatable}'` shows
`"nvidia.com/gpu": "1"`, and `GET /api/v1/platform/capabilities` reports
`day2_ops.gpu_passthrough: true` plus a `gpus` section.

## 3. Create a GPU VM

```bash
curl -sk -X POST https://<host>:30151/api/v1/vms \
  -H "X-API-Key: $VEYRON_API_KEY" -H "Content-Type: application/json" \
  -d '{"name":"gpu-vm","template":"ubuntu22","gpu":{"count":1}}'
```

- `gpu` sugar: `{count, resource_name (default nvidia.com/gpu), vgpu_profile}`.
  Explicit list form: `"gpus": [{"name":"gpu0","device_name":"nvidia.com/gpu"}]`.
- The API **preflights** the request: if no node advertises the resource it
  returns `422 GPU_RESOURCE_UNAVAILABLE` listing what is available
  (`"force": true` to override, e.g. when a node is about to join).
- The converter automatically sets the KubeVirt **KVM-hidden** feature for
  NVIDIA resources — NVIDIA guest drivers refuse to load when they detect KVM
  (Error 43). Set `features.kvm_hidden` explicitly to override.
- GitOps path: `VeyronVM.spec.gpus` / `spec.hostDevices` on the operator CRD.
- Dashboard: Forge wizard shows a GPU picker when the cluster has GPU
  resources; the Nodes page gains a **GPU Inventory** card; GPU VMs carry a
  `⬢ GPU×N` badge.

Inside the guest: `lspci | grep -i nvidia`, install the standard NVIDIA driver
(`apt install nvidia-driver-550` or the .run installer), then `nvidia-smi`.

## 4. Day 2

- **Maintenance / moving a GPU VM**: `virtctl migrate`-style flows are refused
  (409 with `blockers[]`). Cold move: stop the VM → cordon the source node (or
  set `scheduling.node_selector`) → start the VM.
- **Windows guests**: templates already carry `kvm_hidden`-friendly Hyper-V
  enlightenments; RDP (not VNC) for interactive GPU work.
- **Monitoring**: passthrough hands the whole card to the guest — host-side
  `nvidia-smi`/DCGM cannot see it. GPU metrics must come from inside the guest
  (GuestKit/agent).

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| VM Pending, `Insufficient nvidia.com/gpu` | Node doesn't advertise the resource — check `permittedHostDevices` ids match `lspci -nn`, and the GPU is on vfio-pci (`preflight-node.sh --gpu`) |
| virt-launcher: `no available resource for device` | Another VM holds the GPU (whole-card = one VM at a time), or the audio sibling function wasn't given to vfio |
| Windows/NVIDIA driver Error 43 | KVM not hidden — Veyron defaults it on for `nvidia.com/*`; verify `features.kvm.hidden: true` in the emitted VM |
| Node console dead after prep | Expected — the host gave its only GPU to vfio. Keep IPMI/serial/SSH access; use iGPU for the host where available |
| `422 GPU_RESOURCE_UNAVAILABLE` on create | Run `enable-gpu-passthrough.sh`, or the device plugin/virt-handler hasn't re-scanned yet (give it ~1 min) |
| Migration returns 409 | By design — passthrough VMs never live-migrate; use the cold move |
