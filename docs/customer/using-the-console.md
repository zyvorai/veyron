# Using the Console

The Veyron console is a single-page React app served at `/console`. It has no hash routing —
navigation is a left rail plus a top bar, and each page keeps its state without a URL change.

## Shell layout

| Element | Purpose |
|---------|---------|
| **Left rail** | Grouped navigation: Overview, Compute, Storage & network, Security, System |
| **Top bar** | Page title, **Search**, live/sync status, refresh, notifications bell, theme toggle, rail/Inspector toggles, **+ New** |
| **Content pane** | List (table) or grid (card) view for resource pages; full-bleed hero sections on Overview pages |
| **Inspector** (right panel) | Detail view for the selected row — opens automatically on click, closes on Escape or by re-clicking the row |

Swipe or use the dot indicator under full-bleed pages to move between adjacent pages in nav order.

## Full-bleed pages

Mission Control, ConsoleHub, Monitoring, Topology, DR & Velero, and PacketWolf render as
immersive, full-width marketing-style sections. On these pages the left rail and Inspector
auto-collapse so the sections span the true viewport width. Click the panel-toggle icon
(top-left, next to the Veyron wordmark) to bring the rail back without leaving the page; it
re-collapses automatically the next time you navigate to a full-bleed page.

## Theme

Light or dark, saved per-device (`localStorage`, not synced across browsers/devices). Toggle
from the top bar or the **Settings** page.

## Browse vs act

Inventory and status pages are safe to explore. Mutating actions (create, power, migrate,
delete, resolve/ack) follow the API's role gates — a read-only key can browse but not mutate.

## Related

- [Getting Started](getting-started.md)
- [Page-by-page guides](pages/README.md)
