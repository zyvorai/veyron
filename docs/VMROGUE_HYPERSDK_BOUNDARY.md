# Veyron vs HyperSDK product boundary

**See also:** [Documentation index](README.md) · [HyperSDK](https://zyvor.dev/hypersdk)

VMRogue is the **KubeVirt operations platform**: run, govern, observe, and automate virtual machines **already on Kubernetes**.

[HyperSDK](https://zyvor.dev/hypersdk) owns **cross-hypervisor migration and portability** (VMware, Hyper-V, bulk import pipelines, virt-v2v-style workflows).

## Responsibility split

| Capability | VMRogue | HyperSDK |
|------------|---------|----------|
| VM lifecycle (start/stop/resize/snapshots) | Yes | No |
| KubeVirt **live migration** (node drain, VMI migration CRDs) | Yes | No |
| Multus / Cilium / per-VM network policy | Yes | No |
| Tenant isolation, quotas, enterprise SSO | Yes | No |
| GitOps export of KubeVirt YAML | Yes | Optional |
| VMware / Hyper-V **source** migration | **No** — use HyperSDK | Yes |
| VMDK / VHD conversion, virt-v2v orchestration | **No** | Yes |
| Bulk hypervisor import wizards | **No** | Yes |
| CDI DataVolume import **into** the cluster | Yes (target platform) | Often paired |

## What stays in VMRogue

- `POST /api/v1/vms/{ns}/{name}/migrate` — KubeVirt live migration on the cluster
- CDI / DataVolume / golden-image GitOps ([WINDOWS_PACKER_GITOPS_PIPELINE.md](WINDOWS_PACKER_GITOPS_PIPELINE.md))
- Snapshot schedules, Velero discovery, DR failover APIs

## Deprecated in Veyron (migration → HyperSDK)

The following remain for backward compatibility but emit deprecation warnings and will be removed in a future release:

| Surface | Replacement |
|---------|-------------|
| CLI `vmrogue import` (hypervisor manifest bulk path) | HyperSDK migration + CDI import |
| TUI “hypervisor migration wizard” (non-KubeVirt sources) | HyperSDK |
| Docs/runbooks for VMware→KubeVirt one-shot | HyperSDK + [WINDOWS_KUBEVIRT_PRODUCTION.md](WINDOWS_KUBEVIRT_PRODUCTION.md) for Windows targets |

**Note:** `vmrogue import` of a **KubeVirt VirtualMachine YAML** into a namespace is still supported for GitOps handoff; hypervisor-specific import paths are not.

## Positioning statement

> **HyperSDK** gets workloads **onto** Kubernetes. **VMRogue** keeps them **running safely** at scale.

When scoping features, ask: *Is this operating KubeVirt on-cluster, or moving VMs from another hypervisor?* Only the former belongs in VMRogue.
