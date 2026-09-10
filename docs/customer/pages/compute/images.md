# Images & ISOs

## Purpose

ContainerDisk/ISO image catalog plus CDI DataSources, with publish/delete.

## When to use it

- Browsing golden images available to boot VMs from
- Publishing an uploaded image as a versioned golden image (promotes it to a DataSource other VMs
  can clone from)

## How to get there

- Left rail: **Compute → Images & ISOs**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Image name |
| Type | Image kind (ISO, qcow2, …) |
| Size | Image size |
| OS | Guest OS the image targets |
| Added | Age since the image was added |

On a cluster with nothing uploaded yet, this page shows an empty "No images" state — that's
expected, not an error.

Select a row to open the **Inspector** (right panel):

- Type
- Size
- OS
- Source
- Namespace

A second table, **DataSources**, lists the CDI DataSource objects images publish into:

| Column | Shows |
|---|---|
| Name | DataSource name |
| Namespace | Owning namespace |
| Source | Provisioning source |
| Size | Total size |

## What you can do

- **publish** — promote a successfully-uploaded image into a versioned golden image + DataSource
  that VMs can clone from
- **delete** — remove the image
- The **+ New** button in the top bar opens the upload form

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
- [Storage](../storage-network/pvcs.md)
