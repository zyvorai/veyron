# DR & Velero

## Purpose

Cluster backup and failback controls via the Velero integration, dry-run friendly where the API allows.

## When to use it

- Confirm Velero is reachable before relying on cluster-wide backups
- Trigger a Velero backup ahead of a risky cluster change
- Restore from a Velero backup, or fail back a namespace/cluster after DR

## How to get there

- Left rail: **Storage & network → DR & Velero**
- This is a full-bleed page — the left rail and Inspector auto-collapse while you're on it. Click the panel-toggle icon (top-left, next to the Veyron wordmark) to bring the rail back if you need it without leaving the page.

## What you'll see

A hero header ("Disaster Recovery — DR & Velero — Cluster backup and failback controls. Dry-run friendly where the API allows.") with a **Refresh** button, followed by a status section showing whether **Velero** is reachable, and three action cards:

- **Create Velero backup** — dry-run when the API supports it
- **Restore from backup** — prompts you for a backup name
- **DR failback** — confirms before mutating anything

## What you can do

- **Refresh** — re-check Velero reachability
- **Create Velero backup** — trigger a new backup
- **Restore from backup** — restore a named backup
- **DR failback** — fail back after a disaster-recovery event (destructive; the console asks for confirmation first)

This page is Velero's cluster-level backup integration — distinct from the VM-snapshot-driven [Backups](backups.md) page, which lists native KubeVirt backup jobs instead. Velero needs an S3-compatible object-storage endpoint configured on the backend (this lab uses Rook-Ceph's RGW gateway for that).

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Backups](backups.md)
- [Snapshots](snapshots.md)
