# Custom Dashboards

## Purpose

User-defined observability dashboards.

## When to use it

- Open this page when the job matches the purpose above
- Prefer **Mission Control** (`/dashboard`) first if you are unsure where to start
- Confirm API auth and namespace scope if lists look empty

## How to get there

- Route: `/dashboard#custom-dashboards`
- Nav: **Observe → Custom Dashboards** (Browse mega-menu, Finder, or Spotlight `⌘K`)

## Operate from the console (UX)

1. Open `/dashboard#custom-dashboards` and wait for live API data.
2. Use filters (namespace, label, status) when the page provides them.
3. Drill into a VM or resource row for detail, then jump to related surfaces (console, snapshots, policies).
4. For mutating actions (create VM, migrate, apply policy): review impact and role gates first.

If the page stays empty, check API health, auth (`VEYRON_API_KEY` / OIDC), KubeVirt CRDs, and that the workload namespace is selected.

## Related pages

- [Getting Started](../../getting-started.md)
- [Mission Control](../favorites/dashboard.md)
- [VMs](../compute/vms.md)
- [Page index](../../PAGE_INDEX.md)
