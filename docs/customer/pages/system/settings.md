# Settings

## Purpose

Console theme, cluster self-healing policy, and the status of this deployment's optional
integrations (PacketWolf, Atlas, SSO) and installed platform versions.

## When to use it

- Switching between light and dark theme
- Turning on/testing self-healing for degraded VMs
- Checking whether an optional integration (PacketWolf, Atlas storage, SSO) is actually wired up
- Confirming the installed KubeVirt/CDI versions on this cluster

## How to get there

- Left rail: **System → Settings**
- Use the top-bar **Search** to jump here directly
- This is a full-bleed-adjacent page — the left rail stays available; Settings has its own layout
  rather than a table/Inspector pair

## What you'll see

**Appearance**
- Theme toggle (Light / Dark). Saved locally in your browser, per device — it doesn't sync across
  browsers or devices.

**Cluster**
- **Auto-heal degraded machines** — toggle that enables the self-healing policy (restarts
  Failed/Unknown VMs automatically when on).
- **Run self-healing now** — two buttons: **Dry-run** (reports which VMs would be restarted,
  changes nothing) and **Heal** (actually restarts unhealthy VMs). Dry-run is the safe default to
  check first.

**Integrations**
- One row per optional integration (PacketWolf, Atlas storage, Single sign-on), each with a
  status pill: **Healthy** when the integration is configured and its probe succeeded, or
  **Unknown** when it's either not configured (no `VEYRON_*_URL` set for that integration) or the
  probe hasn't resolved yet. This section loads asynchronously — if you navigate here and check
  immediately, integrations can briefly show "Unknown" while the status call is still in flight;
  give it a couple of seconds (or hit refresh) before reading it as a real outage.

**About**
- Installed platform versions — KubeVirt and CDI operator/observed/target versions and rollout
  phase (e.g. `v1.9.0 · Deployed`), plus a note that upgrading is a separate, explicitly guarded
  operation (this page only reports what's installed; it doesn't trigger an upgrade).

## What you can do

- Switch theme (Light/Dark)
- Toggle auto-heal on/off
- Trigger a self-healing dry-run or an actual heal pass
- Check integration health and installed platform versions at a glance

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Admin basics](../../admin-basics.md)
