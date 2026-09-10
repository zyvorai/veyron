# Storage

## Purpose

PersistentVolumeClaim inventory plus a StorageClass/storage-pool summary, with resize and delete.

## When to use it

- Check which VM disks are bound to which StorageClass and how full they are
- Confirm a StorageClass exists and which one is the cluster default before creating a VM
- Resize a claim that's running low on space, or clean up an orphaned PVC

## How to get there

- Left rail: **Storage & network → Storage**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | PVC name |
| Status | Bound / Pending / Lost |
| Namespace | Namespace the PVC lives in |
| Class | StorageClass name |
| Size | Requested capacity |
| Used | Usage meter (needs `VEYRON_PROMETHEUS_URL` wired for real numbers; otherwise shows an estimate) |
| Access | Access mode (e.g. ReadWriteOnce) |

Select a row to open the **Inspector** (right panel):

- Namespace
- Class
- Volume (bound PersistentVolume name)
- Access mode
- Size

## Secondary table

Below the main table, **Storage pools** lists every StorageClass in the cluster: name, class, provisioner, total capacity, a used-capacity meter, and volume count. This is where you confirm which class is marked `(default)` — on a fresh cluster that's usually your CSI driver's block class (e.g. `ceph-rbd`), not `local-path`.

## What you can do

- **resize** — grow a PVC's requested capacity (shrinking isn't supported by Kubernetes)
- **delete** — remove a PVC (only safe once nothing references it)
- The **+ New** button in the top bar opens a form to create a standalone PVC

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Snapshots](snapshots.md)
- [Atlas](atlas.md)
