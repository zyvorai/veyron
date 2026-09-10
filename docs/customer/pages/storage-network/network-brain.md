# PacketWolf

## Purpose

Health of the PacketWolf/Cilium network-intelligence integration.

## When to use it

- Confirm PacketWolf is reachable before relying on network-intelligence data elsewhere
- Diagnose why network-brain data isn't showing up

## How to get there

- Left rail: **Storage & network → PacketWolf**
- This is a full-bleed page — the left rail and Inspector auto-collapse while you're on it. Click the panel-toggle icon (top-left, next to the Veyron wordmark) to bring the rail back if you need it without leaving the page.

## What you'll see

A hero header ("Network Brain — PacketWolf — Cilium / PacketWolf integration health for the console.") with a **Refresh** button, followed by a **Connection** status section showing **Reachable** and a health check row: **Healthy — Health — Probe succeeded**.

This page only reports the *unauthenticated health probe* (`/health`). It can show "Reachable"/"Healthy" even when PacketWolf isn't fully usable: if only a static API key is configured instead of a username/password, the deeper proxied calls (network overview, flow data used elsewhere in the console) will fail with `401 Unauthorized`, because PacketWolf's real auth model is a JWT bearer token obtained via login, not a static key. If network-intelligence data looks missing elsewhere despite this page showing healthy, check that `VEYRON_PACKETWOLF_USERNAME`/`VEYRON_PACKETWOLF_PASSWORD` are set (not just `VEYRON_PACKETWOLF_API_KEY`).

This page only shows real data when PacketWolf is configured (`VEYRON_PACKETWOLF_URL` set). If it isn't configured, the connection status reflects that.

## What you can do

- **Refresh** — re-run the health probe

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Settings](../system/settings.md)
- [Topology](../overview/topology.md)
