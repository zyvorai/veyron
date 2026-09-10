# Backups

## Purpose

Backup job inventory, with run, restore, and delete.

## When to use it

- Check whether a scheduled backup actually ran and succeeded
- Kick off an ad-hoc backup before a maintenance window
- Restore a VM (or its data) from a prior backup

## How to get there

- Left rail: **Storage & network → Backups**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Backup job name |
| Target | What the backup covers (VM name, or backup destination) |
| Items | Item count included in the backup |
| Size | Backup size |
| Last run | Age since the backup last ran |
| Status | Backup status (e.g. Healthy) |

On a cluster with no backups configured yet, this page shows an empty state: "No backups — Nothing matches this view."

Select a row to open the **Inspector** (right panel):

- Target
- Schedule
- Retention

## What you can do

- **run** — trigger the backup job now
- **restore** — restore from this backup
- **delete** — remove the backup record

This page is the native KubeVirt/VM-snapshot-driven backup listing. For cluster-wide backups via the Velero integration specifically (schedules, off-cluster storage, failback), see [DR & Velero](dr.md) instead — they're two different backup mechanisms that can both be in use.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [DR & Velero](dr.md)
- [Snapshots](snapshots.md)
