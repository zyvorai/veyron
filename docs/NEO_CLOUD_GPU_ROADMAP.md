# Neo-Cloud GPU Roadmap

Veyron as a GKE-class GPU cloud on KubeVirt: API-driven GPU VMs, golden images,
hard multi-tenant isolation — starting from one lab node with a consumer GPU
and growing into a two-rack region.

## Vision

A "neo-cloud": NVIDIA GPU workloads on Kubernetes with the operational
smoothness of a hyperscaler — one API/dashboard (Veyron) to forge, snapshot,
protect, meter, and eventually live-migrate GPU VMs. The two racks are the
first region; Veyron is the control plane.

## Path decision (recorded)

Two architectures can deliver "GPU workloads that survive node maintenance":

| | **Path A — VM-in-Kubernetes (KubeVirt + vGPU)** | Path B — CRIU/CUDA checkpoint of raw containers |
|---|---|---|
| Isolation | Hard (hypervisor) — real multi-tenancy | Namespace-level only |
| Migration story | A decade of hypervisor live-migration engineering; vGPU migration exists at the libvirt layer | CUDA checkpoint API is young; containerd has no reliable GPU checkpointing |
| Performance | Small virtualization overhead | Bare-metal |
| Maturity | Production (this repo runs it today) | Bleeding edge, heavy upstream work |
| Cost | NVIDIA vGPU licensing for Phase 2 | Engineering time, upstream risk |

**Decision: Path A.** Veyron *is* Path A — the entire platform (operator,
converter, snapshots, DR, RBAC, dashboard) already manages KubeVirt VMs.
**Revisit trigger for Path B:** NVIDIA's CUDA checkpoint/restore API
(`cuda-checkpoint`) reaching production quality inside CRIU + containerd; if
that lands, a Path-B "burst tier" could complement (not replace) the VM tier.

## Hard constraints (physics & licensing — plan around these)

1. **A PCI-passthrough VM can never live-migrate.** The GPU's state lives on
   the physical card; there is no way to serialize it from the host. Veyron
   enforces this: migration requests return `409` with
   `GPU_PASSTHROUGH_NOT_MIGRATABLE` and a cold-move suggestion.
2. **vGPU (the thing that CAN live-migrate) is licensed and hardware-gated.**
   NVIDIA vGPU / AI Enterprise requires datacenter GPUs (A/L/H-series;
   GeForce is excluded) plus per-GPU or per-CCU licensing and a license server.
   **Phase-1 consumer hardware cannot carry into Phase 2.**
3. **MIG requires Ampere or newer datacenter parts** (A100/A30/H100/L40S…).
4. **Live migration also needs shared RWX storage and same-driver hosts** —
   storage locality rules stay in force even with vGPU (no cross-site
   migration; see [MULTI_SITE_MESH.md](MULTI_SITE_MESH.md)).
5. **etcd does not span WANs.** Single control plane at site 1; site 2 joins
   as workers.

## Phase roadmap

### Phase 1 — Passthrough on what we have (now)

Whole-GPU PCI passthrough on the lab node(s), consumer GPUs, zero licensing.

- Host prep + cluster config: [GPU_PASSTHROUGH.md](GPU_PASSTHROUGH.md)
  (`prepare-gpu-node.sh`, `preflight-node.sh --gpu`,
  `enable-gpu-passthrough.sh`).
- API: `POST /vms {"gpu":{count}}`, `GET /gpus`, capability flags
  (`day2_ops.gpu_passthrough`), migration gate with cold-move guidance.
- Maintenance model: **pet-with-downtime** — stop/start moves, planned windows.
- **Exit criteria:** GPU VM created via the API; `nvidia-smi` works in-guest;
  daily-ops E2E stays green; migration of a GPU VM cleanly refused.

### Phase 2 — vGPU + true live migration (hardware + license gated)

- **Prerequisites (all must hold before code is trusted):**
  1. Datacenter GPUs (e.g. L40S/A100) in both racks.
  2. NVIDIA vGPU host driver + license server (CLS/DLS); budget line-items:
     per-GPU vGPU licenses, support contract.
  3. GPU Operator in vgpu mode exposing mdev resources
     (`nvidia.com/GRID_…`), KubeVirt `permittedHostDevices.mediatedDevices`.
  4. **Verified** KubeVirt support for live-migrating mdev VMIs at our pinned
     version — upstream KubeVirt ~1.8 generally does **not** migrate
     mdev-attached VMIs; this must be probed (capability flag
     `vgpu_live_migration`), never assumed.
- Code hooks already in place: `ClusterMigrationCaps.vgpu_live_migration`
  relaxes the migration gate for vGPU-only VMs; `vgpu_profile` sugar on the
  create API; `virtualGPUOptions` plumbed end-to-end.
- Same-driver-version host pairs, migration `MigrationPolicy` presets
  (bandwidth + auto-converge) for VRAM-heavy moves.

### Phase 3 — Multi-tenant density

- **MIG partitioning** (spatial) for inference fleets; **time-slicing** for
  dev/test; quota + billing via Veyron's existing cost handlers.
- RDMA/GPUDirect networking only becomes relevant here (multi-GPU training
  spanning hosts) — see the team appendix.

## Two-rack topology (decisions recorded)

Details and runbook: [MULTI_SITE_MESH.md](MULTI_SITE_MESH.md).

- **D1 Mesh:** host-level WireGuard underlay (static peers, `10.88.0.0/24`);
  Tailscale as the NAT-heavy alternative. *Revisit:* peer churn.
- **D2 Storage:** per-site Ceph pools, `allowedTopologies` zone pinning, no
  stretch cluster. *Revisit:* <2 ms metro fiber between racks.
- **D3 GPU permit list:** auto-detected from `lspci -nn`, explicit override
  via `V9S_GPU_PCI_IDS`.
- **D4 Device plugin:** KubeVirt built-in for Phase 1; NVIDIA
  kubevirt-gpu-device-plugin (pinned in `versions.env`) as the Phase-2
  stepping stone.
- Control plane: single k3s server at site 1; site-2 = workers; cross-site
  recovery is DR (Velero / Atlas RBD export-diff), never live migration.

## Capability matrix

| Capability | Phase 1 | Phase 2 | Phase 3 |
|---|---|---|---|
| Whole-GPU passthrough | ✔ | ✔ | ✔ |
| vGPU (mediated) | — | ✔ | ✔ |
| MIG partitioning | — | — | ✔ |
| Live migration (non-GPU VMs) | ✔ | ✔ | ✔ |
| Live migration (GPU VMs) | ✘ never (passthrough) | ✔ vGPU only, probed | ✔ |
| Snapshots / DR | ✔ (existing) | ✔ | ✔ |
| Multi-tenant GPU sharing | — (1 VM per card) | partial (vGPU profiles) | ✔ (MIG + quotas) |
| GPU metering / billing | in-guest metrics | vGPU host metrics | per-slice metering |

## Appendix — team & hiring (condensed)

| Role | What they own here | When needed |
|---|---|---|
| **Kernel & virtualization** (QEMU/KVM, vfio, libvirt, C/Rust) | Passthrough edge cases, vGPU host stack, migration tuning | Phase 1: existing team suffices (scripts encode the vfio path). Phase 2: 1 specialist |
| **GPU platform / MLOps** (NVIDIA stack, GPU Operator, CUDA drivers, MIG) | Driver/licensing lifecycle, mdev profiles, GPU observability | Phase 2 onward |
| **Kubernetes upstream** (Go, controller-runtime, KubeVirt, CRI) | Operator, schedulers, upstream KubeVirt gaps (mdev migration) | Continuous — this is the Veyron operator team |
| **RDMA / datacenter networking** (RoCE, InfiniBand, GPUDirect, PCIe topology) | Multi-host training fabric, VRAM-scale migration links | Phase 3 only — not needed for a two-rack passthrough/vGPU cloud |

Sourcing pools: hyperscaler cluster-management teams (GKE/Borg, EC2/EKS),
NVIDIA cloud-native/vGPU teams, Red Hat KubeVirt/OpenShift-Virt contributors,
AI-cloud startups (CoreWeave, Lambda).

## Open decisions log

| # | Decision | Status |
|---|---|---|
| D1 | WireGuard vs Tailscale | **WireGuard** (default), Tailscale supported via env |
| D2 | Cross-site storage policy | **Per-site pools**, revisit on metro fiber |
| D3 | PCI permit list | **Auto-detect + override** |
| D4 | Device plugin | **KubeVirt built-in** P1, NVIDIA DP for P2 |
| D5 | Phase-2 GPU model (which datacenter card) | open — decide at purchase time against vGPU/MIG needs |
| D6 | vGPU licensing model (per-GPU vs CCU) | open — with D5 |
