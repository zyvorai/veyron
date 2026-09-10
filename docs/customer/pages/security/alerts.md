# Alerts

## Purpose

Active Kubernetes and KubeVirt alerts — real cluster Events (scheduling failures, crashed VMs,
failed readiness probes, manifest-apply errors) surfaced as an actionable list.

## When to use it

- Something looks wrong with a VM or the cluster and you want to know why
- You want a single place to triage "what's currently firing" before digging into a specific page
- You're about to acknowledge/close out a known, already-handled issue

## How to get there

- Left rail: **Security → Alerts**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | The Kubernetes reason string, e.g. `FailedScheduling`, `FailedCreate`, `Stopped`, `Unhealthy`, `ApplyManifestFailed` |
| Status | `firing` while the underlying condition is still active |
| Severity | e.g. `warning` |
| Message | The real Event message — for example `0/1 nodes are available: 1 node(s)...` or `The VirtualMachineInstance crashed.` |
| Source | The reporting component, e.g. `default-scheduler`, `virt-handler`, `kubelet`, `daemonset-controller`, `deploy` |
| Fired | When the event was last reported |

These are genuine live cluster Events — not synthetic UI state. An alert here means something in
Kubernetes or KubeVirt actually reported that condition.

Select a row to open the **Inspector** (right panel):
- Severity
- Message (full, unclipped)
- Source
- Fired (exact timestamp)

The Inspector also has an **Events** tab showing related recent events for that alert (often "No
recent events" if the underlying condition hasn't recurred).

## What you can do

- **resolve** — mark the alert handled and clear it from the active list. This doesn't change
  anything in the cluster itself; it only dismisses the alert here. If the underlying condition
  (e.g. a pod still failing to schedule) is still happening, a fresh alert can reappear.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Security](soc.md)
