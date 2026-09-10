# Topology

## Purpose

Topology shows the cluster as a map of nodes and edges, pulled live from the topology API — a
quick visual sanity check of the cluster's shape.

## When to use it

- To confirm how many nodes are in the cluster and that they're all Ready.
- To check for topology edges (relationships between nodes) when diagnosing placement or
  networking issues.

## How to get there

- Left rail: **Overview → Topology**
- This is a full-bleed page — the left rail and Inspector auto-collapse while you're on it. Click
  the panel-toggle icon (top-left, next to the Veyron wordmark) to bring the rail back if you need
  it without leaving the page.

## What you'll see

- A hero with a **Refresh** button.
- A **Nodes** section listing each node's readiness status, name, and kind (e.g. "Ready
  `nldw4-4-16-36` Node").
- An **Edges** section listing topology edges between nodes. On a single-node cluster this is
  typically empty (0 edges).

## What you can do

- Click **Refresh** to re-pull the topology map.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Monitoring](monitoring.md)
- [Hosts](../compute/hosts.md)
