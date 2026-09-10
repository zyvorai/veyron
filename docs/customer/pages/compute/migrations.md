# Migrations

## Purpose

Live-migration job inventory and status.

## When to use it

- Watching the progress of a live migration you just started from the Virtual machines page
- Checking why a migration job didn't complete

## How to get there

- Left rail: **Compute → Migrations**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| ID | Migration job name |
| Status | Job state |
| VM | The VM being migrated |
| Source | Source node |
| Target | Target node |
| Progress | Migration progress |
| Started | Age since the job started |

On a single-node cluster this page is always empty — live migration needs a second schedulable
node with matching capacity, so there's nothing to migrate to (see the "Live migration" tile on
[Monitoring](../overview/monitoring.md), which reports `no` under those conditions).

Select a row to open the **Inspector** (right panel):

- VM
- Source
- Target
- Type
- Progress

## What you can do

- **delete** — remove a completed or stale migration job's record

Migrations themselves are started from a VM row's **migrate** action on the
[Virtual machines](vms.md) page, not from here.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
- [Monitoring](../overview/monitoring.md)
