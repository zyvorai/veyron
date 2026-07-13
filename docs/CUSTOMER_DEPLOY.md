<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
# Veyron — Customer Deployment & Go-Live Runbook

Turns "works in our lab" into "works at the customer." Follow top to bottom; the
**acceptance gate** at the end must pass before sign-off.

## 0. Prerequisites (verify on the customer cluster)

The tested baseline lives in **`scripts/cluster/versions.env`** — the single source of
version truth for every installer and bundle. Do not pin components anywhere else.

| Component | Baseline | Why this version |
|---|---|---|
| Kubernetes / k3s | `v1.35.6+k3s1` | KubeVirt supports the latest **three** Kubernetes releases as of its own release, so KubeVirt 1.8.x is tested on **1.33–1.35**. A newer k3s leaves that window. |
| KubeVirt | `v1.8.4` | current stable |
| CDI | `v1.65.0` | current stable |
| Cilium | `1.19.5` | current stable |
| Storage | shared CSI with clone + snapshot + expand | node-local storage pins each VM to one node |

Two automated gates replace the old manual `kubectl` checklist:

```bash
# Host level: /dev/kvm, vhost-net, tun, vmx/svm, virt-host-validate, swap, time sync
./scripts/cluster/preflight-node.sh <host> <user>

# Cluster level: graded pass/fail against the go-live requirements (exits non-zero)
VEYRON_API_KEY='<key>' ./scripts/preflight-veyron-remote.sh <host> 30151
```

The cluster gate **hard-fails** on: no KubeVirt, no node advertising
`devices.kubevirt.io/kvm`, no default StorageClass, a **node-local default
StorageClass** (e.g. `local-path` — pins every VM disk to one node and loses it with
the node), no **VolumeSnapshotClass object** (the CRD alone is not enough), or a
snapshot class whose driver matches no StorageClass provisioner.

It **warns** (degrades gracefully) on: live hotplug, storage-class migration, live
migration, Windows golden images, persistent TPM/EFI, Velero.

To remediate an existing cluster in place:

```bash
./scripts/cluster/adapt-existing-cluster.sh <host> <user>          # dry-run
./scripts/cluster/adapt-existing-cluster.sh <host> <user> --apply
```

It moves the default StorageClass off node-local storage, **merges** the missing
KubeVirt feature gates, sets `vmStateStorageClass`, and adds a VolumeSnapshotClass for
any CSI driver lacking one. It will not silently paper over a single-node cluster or a
degraded Ceph — those are reported and left to a human.

Windows golden images (ISO upload → sysprep → versioned image → cloned VMs) are
documented in [WINDOWS_GOLDEN_IMAGES.md](WINDOWS_GOLDEN_IMAGES.md).

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
# 1) Readiness + security posture + Day-2 capabilities (fast, seconds).
#    Add --smoke to also run the full functional smoke test (~2-3 min).
VEYRON_API_KEY='<key>' ./scripts/preflight-veyron-remote.sh <host> 30151 --smoke

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
