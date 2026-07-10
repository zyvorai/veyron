<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
# Veyron — Customer Deployment & Go-Live Runbook

Turns "works in our lab" into "works at the customer." Follow top to bottom; the
**acceptance gate** at the end must pass before sign-off.

## 0. Prerequisites (verify on the customer cluster)

| Requirement | Check | Needed for |
|---|---|---|
| Kubernetes ≥ 1.28 | `kubectl version` | base |
| **KubeVirt** installed + `Deployed` | `kubectl get kubevirt -n kubevirt` | VMs (required) |
| **CDI** (optional) | `kubectl get cdi` | image import / DataVolumes |
| A default/known **StorageClass** | `kubectl get sc` | VM disks |
| A **VolumeSnapshotClass** | `kubectl get volumesnapshotclass` | snapshots/backups |
| **Velero** (optional) | `kubectl get crd backups.velero.io` | off-cluster backups |
| container runtime that can pull the Veyron image | — | install |

## 1. KubeVirt feature gates for full Day-2 (important)

Several Day-2 ops are **version/feature-gated** and will otherwise be unavailable
(they fail gracefully, not silently corrupt). Enable on the KubeVirt CR
(`spec.configuration.developerConfiguration.featureGates`):

- `VMLiveUpdateFeatures` → **live CPU/memory hotplug**
- `VolumesUpdateStrategy` (a.k.a. volume migration) → **storage-class disk migration**

After install, `GET /api/v1/platform/capabilities` reports exactly which Day-2
ops this cluster supports — the preflight script (§4) prints it.

## 2. Install

**Recommended (customer): Helm chart** with an image the cluster can pull:
```bash
# push the image to a registry the cluster can reach, then:
helm upgrade --install veyron charts/veyron -n veyron-system --create-namespace \
  --set image.repository=<registry>/veyron --set image.tag=<ver> \
  --set apiKey=<STRONG_KEY>
helm upgrade --install veyron-operator charts/veyron-operator -n veyron-system
```
Validate templates first: `helm template charts/veyron | kubectl apply --dry-run=client -f -`.

**Lab / air-gapped (build on the node):** `./scripts/deploy-remote.sh <host> <user>`
(rsync + remote podman build + import + rollout — dev workflow, not for production).

RBAC lives in three synced places — see CLAUDE.md § "API service account and RBAC".

## 3. Security hardening (do NOT skip)

- **API key**: never ship the default `CHANGE_ME`. Set a strong key via the
  `veyron-api-key` Secret / `VEYRON_API_KEY`, or multi-key RBAC `VEYRON_API_KEYS`
  (`name:key:role`). Consider OIDC (`VEYRON_OIDC_*`) for SSO.
- **TLS**: the pod's init container generates a **self-signed** cert. Replace with
  a real cert for anything customer-facing.
- **RBAC**: the API `ClusterRole` is intentionally broad (KubeVirt/CDI/Cilium/
  Velero/nodes-patch/kubevirts-patch). For locked-down sites, trim
  `cilium.io/*`, `gateway.networking.k8s.io/*`, CDI `*`, and the upgrade/node
  verbs, and use a separate read-only role for dashboards.
- Day-2 mutations are **Write**; cluster-wide/destructive ops (node cordon,
  upgrade, DR failback, orphan delete) are **Admin**; destructive ones are
  **dry-run by default** (`?confirm=true` / `dry_run:false` to apply).

## 4. Acceptance gate (must pass — run against the customer cluster)

```bash
# 1) Readiness + capabilities + smoke test (one command)
VEYRON_API_KEY='<key>' ./scripts/preflight-veyron-remote.sh <host> 30151

# 2) Full VM daily-ops E2E (create/start/stop/pause/expose/snapshot + Day-2 Tier C)
VEYRON_API_KEY='<key>' ./scripts/test-vm-daily-ops-remote.sh <host> 30151

# 3) Dashboard has zero console/page errors
VEYRON_API_KEY='<key>' ./scripts/dashboard-console-check.sh --host <host>
```
Sign off only when all three are green **on the customer's cluster** — lab passes
do not transfer, because feature gates, storage, CNI, scale and guest images differ.

## 5. Guest-dependent ops

Guest OS patching, `fstrim` reclaim, application-consistent snapshots, and
in-guest filesystem stats need a **guest agent in the VM image** (GuestKit on
Linux — bootstrapped by Veyron's Linux templates; QGA on Windows). Without it
these degrade to "unavailable" rather than erroring. Confirm the customer's
golden images carry the agent.

## 6. Optional integrations (auto-detected on deploy)

Prometheus/Grafana/Alertmanager, Loki, OpenCost, Argo CD, PacketWolf, and
**Atlas** (Ceph-backed disk snapshot/backup/restore) are wired automatically when
present (`scripts/lib/bootstrap-integrations.sh`) — see `docs/OPTIONAL_INTEGRATIONS.md`.
None are required for core VM management.
