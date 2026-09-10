# Veyron — Complete page index

Every primary navigable console page.

_Generated: 2026-09-10 · 21 routes_

Regenerate: `node scripts/customer-docs/generate-page-index.mjs`

## Overview

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Mission Control | `mission` | Mission Control — full-bleed fleet summary: machines running, host count, quick create, and per-namespace tone accents. | [Open](pages/overview/mission.md) |
| ConsoleHub | `console` | ConsoleHub — quick links into VNC/serial console sessions and day-2 actions for running VMs. | [Open](pages/overview/console.md) |
| Monitoring | `monitoring` | Capacity headroom (free CPU/memory, node count, live-migration support) and installed KubeVirt/CDI platform versions. | [Open](pages/overview/monitoring.md) |
| Topology | `topology` | Cluster map as nodes and edges from the topology API. | [Open](pages/overview/topology.md) |

## Compute

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Virtual machines | `vms` | VirtualMachine inventory — create, start/stop/pause/restart, migrate, snapshot, clone, and inspect KubeVirt VMs. | [Open](pages/compute/vms.md) |
| Hosts | `hosts` | Kubernetes node inventory — status, kernel, uptime, cordon/uncordon, reboot. | [Open](pages/compute/hosts.md) |
| GPUs | `gpus` | Per-node GPU inventory — passthrough/MIG/vGPU classification and allocatable counts. | [Open](pages/compute/gpus.md) |
| Images & ISOs | `images` | ContainerDisk/ISO image catalog plus CDI DataSources, with publish/delete. | [Open](pages/compute/images.md) |
| Pods | `pods` | Pod inventory across all namespaces, including virt-launcher pods, with logs and delete. | [Open](pages/compute/pods.md) |
| Template Foundry | `templates` | Template Foundry — the built-in OS template catalog plus VMProfiles, with clone/delete. | [Open](pages/compute/templates.md) |
| Migrations | `migrations` | Live-migration job inventory and status. | [Open](pages/compute/migrations.md) |

## Storage & network

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Storage | `pvcs` | PersistentVolumeClaim inventory plus StorageClass/storage-pool summary, with resize/delete. | [Open](pages/storage-network/pvcs.md) |
| Snapshots | `snapshots` | VM disk snapshot inventory, with restore/delete. | [Open](pages/storage-network/snapshots.md) |
| Backups | `backups` | Backup job inventory, with run/restore/delete. | [Open](pages/storage-network/backups.md) |
| Atlas | `atlas` | Ceph-backed storage volumes and snapshots from the Atlas storage control-plane integration. | [Open](pages/storage-network/atlas.md) |
| Networks | `networks` | NetworkAttachmentDefinition (Multus) inventory, with create/delete. | [Open](pages/storage-network/networks.md) |
| PacketWolf | `network-brain` | PacketWolf/Cilium network-intelligence integration health. | [Open](pages/storage-network/network-brain.md) |
| DR & Velero | `dr` | Cluster backup and failback controls (Velero), dry-run friendly where the API allows. | [Open](pages/storage-network/dr.md) |

## Security

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Alerts | `alerts` | Active Kubernetes/KubeVirt alerts and events, with resolve. | [Open](pages/security/alerts.md) |
| Security | `soc` | Security detections surfaced by the SOC pipeline, with acknowledge. | [Open](pages/security/soc.md) |

## System

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Settings | `settings` | Console theme, self-healing policy, and integration status (PacketWolf, Atlas, SSO) for this deployment. | [Open](pages/system/settings.md) |

## Related

- [Customer docs home](README.md)
- [Page-by-page guides](pages/README.md)
