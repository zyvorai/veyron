# Using the Dashboard

Veyron Mission Control is a hash-routed SPA under `/dashboard` (`#page`).

## Surfaces

| Element | Purpose |
|---------|---------|
| **Mission Control** | Home wall and fleet signals |
| **Finder / Browse** | Sidebar + mega-menu navigation |
| **Spotlight (`⌘K`)** | Jump to any labeled page |
| **Favorites** | Fleet Constellation, Reactor, Gallery, vCentre, ConsoleHub, VM Capsule |
| **Settings** | Auth, theme, AI providers |

## Browse vs act

Inventory and metrics pages are safe to explore. Create / power / migrate / policy apply actions follow role gates (`veyron-admin` / `veyron-write` for OIDC groups).

## Related

- [Getting Started](getting-started.md)
- [Page-by-page guides](pages/README.md)

## Operate from the console (UX)

1. Open this route from the nav or command palette and wait for live API data.
2. Use filters/search when present; drill into a row for detail.
3. For mutating actions: confirm role gates and impact before applying.
4. **Empty / fail:** Check service health, auth, and that required CRDs/backends for this domain are installed.
5. **Success:** Live data loads; created/updated objects appear without error toasts.

