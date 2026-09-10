# Networks

## Purpose

NetworkAttachmentDefinition (Multus) inventory, with create and delete.

## When to use it

- Define an additional network a VM can attach to beyond the cluster's default pod network
- Check which NADs exist, their CIDR/VLAN, and how many VMs are attached
- Remove a network definition that's no longer needed

## How to get there

- Left rail: **Storage & network → Networks**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | NetworkAttachmentDefinition name |
| Type | Network type (e.g. bridge, macvlan) |
| CIDR | Subnet, when defined |
| VLAN | VLAN id, when defined |
| Attached | Count of VMs currently attached |
| Status | Network status |

On a cluster with no custom networks defined yet, this page shows an empty state: "No networks — Nothing matches this view."

Select a row to open the **Inspector** (right panel):

- Type
- CIDR
- VLAN
- Gateway
- Namespace

## What you can do

- **delete** — remove the network definition (fails safely if VMs are still attached)
- The **+ New** button in the top bar opens a form to create a NetworkAttachmentDefinition

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](../compute/vms.md)
- [Topology](../overview/topology.md)
