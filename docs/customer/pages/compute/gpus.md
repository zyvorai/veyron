# GPUs

## Purpose

Per-node GPU inventory — passthrough/MIG/vGPU classification and allocatable counts.

## When to use it

- Checking GPU capacity before creating a GPU-attached VM
- Confirming which nodes advertise which GPU product/type before scheduling

## How to get there

- Left rail: **Compute → GPUs**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Node | The node advertising the GPU resource |
| Status | Availability state |
| Product | GPU product name |
| Alloc | Allocatable count on that node |
| Type | Passthrough / MIG / vGPU classification |
| Memory | GPU memory |

On a cluster with no GPU hardware installed, this page shows an empty "No gpus" state — that's
expected, not an error.

Select a row to open the **Inspector** (right panel):

- Product
- Type
- Resources
- Memory
- Allocatable

## What you can do

This is a read-only inventory page — no row actions. To attach a GPU to a VM, set it when
creating the VM (see [Virtual machines](vms.md)); the create form rejects the request up front
if no node currently advertises the requested GPU resource.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
- [Hosts](hosts.md)
