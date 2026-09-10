# Pods

## Purpose

Pod inventory across all namespaces, including virt-launcher pods, with logs and delete.

## When to use it

- Checking the platform's own component pods (CDI, cert-manager, Cilium, …) alongside VM
  virt-launcher pods, in one place
- Pulling logs off a crashing or restarting pod
- Confirming restart counts before escalating a VM issue

## How to get there

- Left rail: **Compute → Pods**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | Pod name |
| Status | Phase (Running, Pending, …) |
| Namespace | Owning namespace |
| Ready | Ready container count |
| Restarts | Restart count |
| Node | Node the pod is scheduled on |
| Age | Pod age |

The Ready column currently always shows `—`: `GET /api/v1/pods` returns name/namespace/phase/
node_name/ip/containers/restarts/age, but no container-readiness field, so there's nothing for
the UI to show yet. This is a backend gap, not a display bug — Status and Restarts are accurate.

Select a row to open the **Inspector** (right panel):

- Namespace
- Node
- Image
- Restarts

## What you can do

- **logs** — view the pod's container logs
- **delete** — delete the pod (a controller-managed pod, like a virt-launcher, gets recreated)

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](vms.md)
- [Hosts](hosts.md)
