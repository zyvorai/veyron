# Common workflows

## Create and console into a VM

1. [Template Foundry](pages/platform/app-store.md) or [Blueprint Studio](pages/platform/blueprint-studio.md)
2. [VMs](pages/compute/vms.md) — confirm Running
3. [ConsoleHub](pages/favorites/console-hub.md) — open VNC/serial

## Snapshot and protect

1. [Snapshots](pages/storage/snapshots.md)
2. [Backups](pages/storage/backups.md)
3. [Disaster Recovery](pages/ops/dr.md) when configured

## Investigate an incident

1. [Mission Control](pages/favorites/dashboard.md)
2. [Alerts](pages/observe/alerts.md) / [Incidents](pages/observe/incidents.md)
3. [Ask Zyra](pages/ask-zyra/ask-zyra.md)

## Related

- [Getting Started](getting-started.md)

## Operate from the console (UX)

1. Open this route from the nav or command palette and wait for live API data.
2. Use filters/search when present; drill into a row for detail.
3. For mutating actions: confirm role gates and impact before applying.
4. **Empty / fail:** Check service health, auth, and that required CRDs/backends for this domain are installed.
5. **Success:** Live data loads; created/updated objects appear without error toasts.

