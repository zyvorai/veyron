# ConsoleHub

## Purpose

ConsoleHub is a one-click launcher for live consoles into your running guests — no need to open
the Virtual machines page and find the console action on a specific row first.

## When to use it

- You know a VM is running and just want its display, fast.
- You want to see every guest with an open console option in one grid, instead of scanning a
  table.

## How to get there

- Left rail: **Overview → ConsoleHub**
- This is a full-bleed page — the left rail and Inspector auto-collapse while you're on it. Click
  the panel-toggle icon (top-left, next to the Veyron wordmark) to bring the rail back if you need
  it without leaving the page.

## What you'll see

- If no VMs are currently running: an empty state ("No running guests — start a machine, then
  open its console from here") with a **Create a machine** button.
- If VMs are running: a card grid, one card per running VM, showing its name, status, guest OS,
  IP address, and an **Open** button.

## What you can do

- Click **Open** on any card to open a live display (VNC/serial) session for that VM.
- Click **Create a machine** from the empty state to open the new-VM form.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Virtual machines](../compute/vms.md)
