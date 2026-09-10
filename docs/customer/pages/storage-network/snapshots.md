# Snapshots

## Purpose

VM disk snapshot inventory, with restore and delete.

## When to use it

- Before a risky change to a VM (OS update, config change), take a snapshot first
- Roll a VM back to a known-good point after something breaks
- Clean up old snapshots that are no longer needed

## How to get there

- Left rail: **Storage & network → Snapshots**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Snapshot name |
| Machine | The VM this snapshot was taken from |
| Size | Snapshot size |
| Taken | Age since the snapshot was created |
| Status | Snapshot status (e.g. Ready) |

On a cluster with no snapshots yet, this page shows an empty state: "No snapshots — Nothing matches this view."

Select a row to open the **Inspector** (right panel):

- Machine
- Size
- Taken

## What you can do

- **restore** — roll the source VM back to this snapshot
- **delete** — remove the snapshot

Snapshots here use KubeVirt's native `VolumeSnapshot` flow. If Atlas (Ceph-backed storage) is configured, Ceph-level snapshotting is available separately from a VM's Inspector panel — see [Atlas](atlas.md).

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Storage](pvcs.md)
- [Atlas](atlas.md)
