# Monitoring

## Purpose

Monitoring shows how much headroom the cluster has right now, and which KubeVirt/CDI platform
versions are actually installed — the two questions you ask before sizing a new VM or planning an
upgrade.

## When to use it

- Before creating a large VM, to check there's enough free CPU/memory.
- To confirm whether live migration is available on this cluster.
- To check the installed KubeVirt/CDI versions without SSHing into a node.

## How to get there

- Left rail: **Overview → Monitoring**
- This is a full-bleed page — the left rail and Inspector auto-collapse while you're on it. Click
  the panel-toggle icon (top-left, next to the Veyron wordmark) to bring the rail back if you need
  it without leaving the page.

## What you'll see

- A hero with a **Refresh** button.
- A stats strip: **CPU free** (cores), **Memory free** (GiB), **Nodes** (count), and whether
  **Live migration** is supported on this cluster.
- A "Platform stack" section listing each installed component (currently `kubevirt` and `cdi`) as
  a version + deployment phase, e.g. `v1.9.0 · Deployed`.

## What you can do

- Click **Refresh** to re-pull capacity headroom and platform versions.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Hosts](../compute/hosts.md)
- [Topology](topology.md)
