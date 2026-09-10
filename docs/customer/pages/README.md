# Page-by-page guides

Each guide follows: Purpose → When to use it → How to get there → What you can do → Related pages.

Every route is also listed in the [complete page index](../PAGE_INDEX.md).

## Compute

| Page | What it covers |
|------|----------------|
| [GPUs](compute/gpus.md) | Per-node GPU inventory — passthrough/MIG/vGPU classification and allocatable counts. |
| [Hosts](compute/hosts.md) | Kubernetes node inventory — status, kernel, uptime, cordon/uncordon, reboot. |
| [Images & ISOs](compute/images.md) | ContainerDisk/ISO image catalog plus CDI DataSources, with publish/delete. |
| [Migrations](compute/migrations.md) | Live-migration job inventory and status. |
| [Pods](compute/pods.md) | Pod inventory across all namespaces, including virt-launcher pods, with logs and delete. |
| [Template Foundry](compute/templates.md) | Template Foundry — the built-in OS template catalog plus VMProfiles, with clone/delete. |
| [Virtual machines](compute/vms.md) | VirtualMachine inventory — create, start/stop/pause/restart, migrate, snapshot, clone, and |

## Overview

| Page | What it covers |
|------|----------------|
| [ConsoleHub](overview/console.md) | ConsoleHub is a one-click launcher for live consoles into your running guests — no need to open |
| [Mission Control](overview/mission.md) | Mission Control is the console's home page — a full-bleed fleet summary that answers "is |
| [Monitoring](overview/monitoring.md) | Monitoring shows how much headroom the cluster has right now, and which KubeVirt/CDI platform |
| [Topology](overview/topology.md) | Topology shows the cluster as a map of nodes and edges, pulled live from the topology API — a |

## Security

| Page | What it covers |
|------|----------------|
| [Alerts](security/alerts.md) | Active Kubernetes and KubeVirt alerts — real cluster Events (scheduling failures, crashed VMs, |
| [Security](security/soc.md) | Security detections surfaced by the SOC (security operations) pipeline — a dedicated feed for |

## Storage Network

| Page | What it covers |
|------|----------------|
| [Atlas](storage-network/atlas.md) | Ceph-backed (and NFS/ZFS-backed) storage volumes and snapshots surfaced through the Atlas storage control-plane integration. |
| [Backups](storage-network/backups.md) | Backup job inventory, with run, restore, and delete. |
| [DR & Velero](storage-network/dr.md) | Cluster backup and failback controls via the Velero integration, dry-run friendly where the API allows. |
| [PacketWolf](storage-network/network-brain.md) | Health of the PacketWolf/Cilium network-intelligence integration. |
| [Networks](storage-network/networks.md) | NetworkAttachmentDefinition (Multus) inventory, with create and delete. |
| [Storage](storage-network/pvcs.md) | PersistentVolumeClaim inventory plus a StorageClass/storage-pool summary, with resize and delete. |
| [Snapshots](storage-network/snapshots.md) | VM disk snapshot inventory, with restore and delete. |

## System

| Page | What it covers |
|------|----------------|
| [Settings](system/settings.md) | Console theme, cluster self-healing policy, and the status of this deployment's optional |

---

21 guides. Regenerate: `node scripts/customer-docs/generate-guide-index.mjs`.
