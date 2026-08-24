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
KubeVirt feature gates, sets `vmStateStorageClass`, adds a VolumeSnapshotClass for
any CSI driver lacking one, and (on k3s) enables `nonroot-devices` so containerd lets
CDI's non-root importer pods open Block-mode PVCs (see callout below). It will not
silently paper over a single-node cluster or a degraded Ceph — those are reported and
left to a human.

**Block-mode DataVolumes silently break without a containerd setting.** CDI's importer
pods run as a non-root UID by default. Unless containerd is configured with
`device_ownership_from_security_context = true` (k3s: `nonroot-devices: true` in
`/etc/rancher/k3s/config.yaml`, then restart k3s), every blank Block-mode
DataVolume — including the Windows golden-image builder's root disk — fails with
`blockdev: cannot open /dev/cdi-block-volume: Permission denied` and sits in
`ImportInProgress`/`CrashLoopBackOff` forever. This is **not** caught by the standard
capability gate (it checks that the feature/CRDs exist, not that a real Block PVC can
actually import), so verify it explicitly on any customer cluster that will use Windows
golden images or any other Block-mode storage: `adapt-existing-cluster.sh` now checks
and fixes this for k3s automatically; on non-k3s hosts it's flagged as a manual step
(edit `/etc/containerd/config.toml`, restart containerd). See
[block_cri_ownership_config.md](https://github.com/kubevirt/containerized-data-importer/blob/main/doc/block_cri_ownership_config.md)
upstream.

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

- **API key**: never ship the default `Admin@321`. Set a strong key via the
  `veyron-api-key` Secret / `VEYRON_API_KEY`, or multi-key RBAC `VEYRON_API_KEYS`
  (`name:key:role`). Consider OIDC (`VEYRON_OIDC_*`) for SSO against a real IdP
  (Keycloak/Auth0/Okta) — deploy with `./scripts/deploy-remote.sh <host> <user> --with-oidc`
  (writes a dedicated `veyron-oidc` Secret, keeps the client secret out of `--set`/shell
  history) or the Helm chart's `oidc.*` values. Full walkthrough with a worked Keycloak
  example: [OIDC_SSO.md](OIDC_SSO.md). **Confidential IdP clients (Keycloak's default) need
  `VEYRON_OIDC_CLIENT_SECRET` set** — PKCE alone is not sufficient for a confidential client
  and the login will silently fail without it.
- **TLS**: the pod's init container generates a **self-signed** cert. Replace with
  a real cert for anything customer-facing.
- **RBAC**: the API `ClusterRole` is intentionally broad (KubeVirt/CDI/Cilium/
  Velero/nodes-patch/kubevirts-patch). For locked-down sites, trim
  `cilium.io/*`, `gateway.networking.k8s.io/*`, CDI `*`, and the upgrade/node
  verbs, and use a separate read-only role for dashboards.
- Day-2 mutations are **Write**; cluster-wide/destructive ops (node cordon,
  upgrade, DR failback, orphan delete) are **Admin**; destructive ones are
  **dry-run by default** (`?confirm=true` / `dry_run:false` to apply).

## 4. Acceptance gate — one command (must print GO)

Run the single customer-readiness gate against the customer's cluster. It chains
every phase (node preflight → acceptance gate → read-only smoke → security,
data-safety, resilience, upgrade/ops → lifecycle E2E → dashboard sweep), auto-
detects cluster shape, and writes a **GO / NO-GO** sign-off report.

```bash
# Full RBAC coverage: supply three keyed roles so the security phase can PROVE
# enforcement (readonly/write denials) rather than skip it.
VEYRON_API_KEYS='ro:RO_KEY:readonly,wr:WRITE_KEY:write,admin:ADMIN_KEY:admin' \
VEYRON_RO_KEY='RO_KEY' VEYRON_WRITE_KEY='WRITE_KEY' VEYRON_ADMIN_KEY='ADMIN_KEY' \
VEYRON_API_KEY='ADMIN_KEY' \
  ./scripts/customer-readiness.sh <host> 30151 \
    --ssh-user <user> --report-dir ./signoff
```

**Sign off only when `./signoff/readiness-report.md` prints `GO`.** Lab passes do
not transfer — feature gates, storage, CNI, scale and guest images differ — which
is exactly why this gate runs against the customer's real cluster.

### 4a. What GO means (SKIP / WARN / FAIL semantics)

The verdict is **`GO` iff zero failures and the P1 acceptance gate passed**;
warnings and skips never block.

| Phase | Covers |
|---|---|
| P0 node preflight | `/dev/kvm`, vhost-net, virt-host-validate, swap, time sync (SSH; needs `--ssh-user`) |
| P1 acceptance gate | KubeVirt + shared default StorageClass + a VolumeSnapshotClass object — **the only fail-fast point** (stops before any mutation) |
| P2 read-only smoke | ~90 read-only API GETs |
| P3 security & multi-tenancy | RBAC 403/200 enforcement matrix, auth/TLS posture, namespace scoping, cross-ns clone RBAC |
| P4 data safety | snapshot→mutate→restore data round-trip, DR export/apply, orphan reclaim, Velero |
| P5 resilience | live migration (node A→B, IP unchanged), cordon/drain, self-healing |
| P6 upgrade & ops | upgrade path, monitoring scrape, day-2 hotplug seen by guest, version drift |
| P7 lifecycle E2E | create/start/stop/pause/expose/snapshot in the test namespace |
| P8 dashboard | zero console/page errors across the SPA |
| P9 (`--full`) | Windows golden-image clone-boot (long) |

- **FAIL → NO-GO**: a capability the cluster *advertises* is broken, or a security
  guarantee is violated (a read-only key mutates; restore loses data; migration
  drops the VM).
- **WARN** (never blocks): default `Admin@321` key, self-signed cert, version
  drift, degraded Ceph — review, but they don't stop go-live.
- **SKIP / N/A** (never blocks): single-node → migration/drain SKIP with reason
  `single_node`; Velero/Prometheus/GuestKit absent; an opt-in flag unset. Every
  skip carries a machine reason so the report never hides gaps.

Multi-node checks (live migration, drain) only run on a **≥2-node** cluster; on a
single-node lab they SKIP honestly. The version baseline is
[`scripts/cluster/versions.env`](../scripts/cluster/versions.env).

### 4b. Opt-in destructive checks (maintenance window)

Safe-by-default: every test resource lives in the disposable `veyron-readiness`
namespace, is named `veyron-readiness-*`, is guarded by an ownership check before
any delete, and is torn down by a LIFO cleanup trap — **existing customer VMs are
never touched**. The genuinely disruptive checks are opt-in:

| Flag | Enables |
|---|---|
| `VEYRON_READY_ALLOW_DRAIN=1` | cordon/drain a node (uncordon auto-registered as cleanup) |
| `VEYRON_READY_ALLOW_ORPHAN_DELETE=1` | real `confirm=true` orphan-PVC reclaim |
| `VEYRON_READY_ALLOW_HEAL=1` | kill a *test* VMI and prove self-healing recovers it |
| `VEYRON_READY_ALLOW_UPGRADE=1` | same-version no-op operator bounce (assert VM survives) |
| `VEYRON_READY_ALLOW_VELERO=1` | real Velero backup→restore |
| `VEYRON_READY_ALLOW_CLONE_RBAC=1` | throwaway tenant namespace for cross-ns clone RBAC |
| `--allow-destructive` | flip all of the above on at once |

Run the safe-by-default form first; add flags only inside a scheduled maintenance
window.

**Real snapshot data round-trip:** the built-in `ubuntu-22.04` test template is
containerDisk-rooted (ephemeral), so the in-guest data-return proof SKIPs by
default. To exercise a genuine PVC-backed restore (write marker → snapshot →
delete → restore → assert marker returns), point it at a published golden image:
`VEYRON_READY_DATASOURCE=<CDI DataSource name>`. Without it the snapshot
create→ready→restore lifecycle still runs; only the data assertion skips.

### 4c. Run one phase (debugging)

The legacy single-purpose scripts still work standalone when you want to isolate a
failure the gate surfaced:

```bash
VEYRON_API_KEY='<key>' ./scripts/preflight-veyron-remote.sh <host> 30151 --smoke
VEYRON_API_KEY='<key>' ./scripts/test-vm-daily-ops-remote.sh <host> 30151
VEYRON_API_KEY='<key>' ./scripts/dashboard-console-check.sh --host <host>
VEYRON_API_KEY='<key>' ./scripts/test/security-rbac-remote.sh <host> 30151
```

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
