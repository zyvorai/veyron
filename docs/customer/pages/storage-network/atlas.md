# Atlas

## Purpose

Ceph-backed (and NFS/ZFS-backed) storage volumes and snapshots surfaced through the Atlas storage control-plane integration.

## When to use it

- See every storage volume Atlas manages across its backends (Ceph RBD, NFS, ZFS) in one place
- Check which volume backs a given VM's PVC, and which pool/backend it lives on
- Review Atlas-level snapshots (separate from KubeVirt's native `VolumeSnapshot` objects)

## How to get there

- Left rail: **Storage & network → Atlas**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

This page only shows data when Atlas is configured (`VEYRON_ATLAS_URL` set on the Veyron API deployment). If it isn't configured, the table is empty and [Settings](../system/settings.md) shows the Atlas integration as not configured.

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Volume name (an export path for NFS, a pool/name pair for RBD, a dataset path for ZFS) |
| Status | Volume state (e.g. available) |
| Namespace | Kubernetes namespace, when this volume backs a PVC |
| Size | Total volume size |
| Backend | The storage pool/backend id this volume lives on |
| PVC | The Kubernetes PVC name this volume backs, when applicable |

Not every volume is tied to a Kubernetes object — NFS and ZFS volumes exported for general use may show no Namespace/PVC, while a Ceph RBD volume backing an in-cluster PVC shows both.

Select a row to open the **Inspector** (right panel):

- Namespace
- Backend
- PVC
- Size

## Secondary table

Below the main table, **Atlas snapshots** lists Atlas-level snapshots taken of these volumes: name, status, source volume, size, and age.

## What you can do

This page is read-only inventory — there are no row actions here. To take a Ceph-backed snapshot of a specific VM's disk, open that VM on the [Virtual machines](../compute/vms.md) page and use the **Ceph snap** action in its Inspector panel; that action calls Atlas directly for the selected VM.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Storage](pvcs.md)
- [Virtual machines](../compute/vms.md)
