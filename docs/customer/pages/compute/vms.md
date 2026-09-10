# Virtual machines

## Purpose

VirtualMachine inventory — create, start/stop/pause/restart, migrate, snapshot, clone, and
inspect KubeVirt VMs.

## When to use it

- Your daily home base for the VM fleet: power state, sizing, and network address at a glance
- Creating a new VM from a template
- Running day-2 lifecycle actions (stop, pause, snapshot, clone) on an existing VM

## How to get there

- Left rail: **Compute → Virtual machines**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | VM name |
| Status | Power/health state (Running, Stopped, Starting, …) |
| vCPU | Allocated virtual CPUs |
| Memory | Allocated memory, in GiB |
| Disk | Root disk size, in GB |
| Host | The node the VM's virt-launcher pod is running on |
| IP | Guest network address |
| Uptime | Age since the VM last started |

Select a row to open the **Inspector** (right panel):

- Guest OS
- Host
- IP
- Uptime
- Namespace

## What you can do

- **start** — power on a stopped VM
- **stop** — graceful power-off
- **restart** — graceful restart (stop, then start)
- **pause** — freeze the running VM in place without powering off
- **unpause** — resume a paused VM
- **migrate** — live-migrate to another node (needs a second schedulable node with matching capacity)
- **console** — open a VNC/serial console session
- **snapshot** — take a point-in-time disk snapshot
- **clone** — create a new VM from this one
- **delete** — remove the VM
- The **+ New** button in the top bar opens the create-VM form (name + template)

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Template Foundry](../compute/templates.md)
- [Snapshots](../storage-network/snapshots.md)
