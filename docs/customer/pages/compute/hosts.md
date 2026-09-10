# Hosts

## Purpose

Kubernetes node inventory — status, kernel, uptime, cordon/uncordon, reboot.

## When to use it

- Checking which nodes are Ready before scheduling more VMs
- Taking a node out of rotation for maintenance (cordon) and bringing it back (uncordon)

## How to get there

- Left rail: **Compute → Hosts**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Node name |
| Status | Ready / Cordoned / NotReady |
| CPU | CPU utilization meter |
| Memory | Memory utilization meter |
| VMs | Number of VMs currently on this node |
| Kernel | Kernel version |
| Uptime | Node age |

CPU/Memory utilization and the VMs count require live metrics wiring the platform API doesn't
expose yet — on a cluster without that wired up, those columns legitimately show `0%`/`—` rather
than real usage (verified against `GET /api/v1/nodes`, which currently returns capacity/allocatable
and version fields but no per-node usage or VM-count fields). This is a backend gap, not a
display bug.

Select a row to open the **Inspector** (right panel):

- Kernel
- Cores
- Memory
- VMs
- Uptime

## What you can do

- **cordon** — mark the node unschedulable (existing VMs keep running, no new ones land here)
- **uncordon** — make the node schedulable again
- **reboot** — reboot the node

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
- [Monitoring](../overview/monitoring.md)
