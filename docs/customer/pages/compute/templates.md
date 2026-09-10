# Template Foundry

## Purpose

Template Foundry — the built-in OS template catalog plus VMProfiles, with clone/delete.

## When to use it

- Picking an OS template when creating a VM
- Checking a template's default sizing (vCPU/memory/disk) before basing a VM on it
- Browsing VMProfiles (named sizing presets) available to the catalog

## How to get there

- Left rail: **Compute → Template Foundry**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Template name (e.g. `almalinux`, `ubuntu-22.04`) |
| OS | Guest OS family |
| vCPU | Default vCPU count |
| Memory | Default memory, in GiB |
| Disk | Default disk size, in GB |
| Used by | Number of VMs created from this template |

Sizing comes from whichever backend source answers first: the built-in template registry (which
carries real vCPU/memory/disk presets) or, when your cluster has synced a VMTemplate CRD catalog
(`veyron catalog sync`), that catalog instead — which only carries family/description/tags, no
sizing. When the catalog source wins, vCPU/Memory/Disk correctly show `—` rather than a
misleading `0`.

Select a row to open the **Inspector** (right panel):

- OS
- Cloud-init
- Used by
- Tags

A second table, **VMProfiles**, lists named sizing presets:

| Column | Shows |
|---|---|
| Name | Profile name |
| Family | OS family the profile targets |
| vCPU | vCPU count |
| Memory | Memory |

## What you can do

- **clone** — duplicate a template
- **delete** — remove a template
- The **+ New** button in the top bar opens the create form

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
