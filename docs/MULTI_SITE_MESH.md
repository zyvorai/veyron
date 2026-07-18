# Two-Rack / Multi-Site Mesh

How Veyron's k3s cluster spans two racks (or two sites) over a **host-level
WireGuard underlay**, with a single control plane and per-site storage.

```
        Site 1 (rack A)                        Site 2 (rack B)
  ┌────────────────────────┐            ┌────────────────────────┐
  │ k3s SERVER  10.88.0.1  │◄─WireGuard─►│ worker      10.88.0.2  │
  │ (control plane + work) │  udp/51820  │ worker      10.88.0.3  │
  │ Rook-Ceph (site-1)     │            │ Ceph/Atlas (site-2)    │
  │ zone=site-1            │            │ zone=site-2            │
  └────────────────────────┘            └────────────────────────┘
       VMs migrate within site-1             VMs migrate within site-2
                  cross-site = DR (Velero / Atlas), never live migration
```

## Design decisions (and when to revisit)

| Decision | Choice | Revisit when |
|---|---|---|
| Mesh | **Host-level WireGuard**, static peers (`wg-veyron`, `10.88.0.0/24`, udp/51820) | Sites are NAT-heavy or peers churn → use Tailscale (`V9S_MESH_BACKEND=tailscale`) |
| Control plane | **Single server at site 1**; site 2 = workers only | Never run etcd quorum across a WAN — >10 ms RTT etcd is a known failure mode. Revisit only with metro dark fiber (<2 ms) |
| CNI | Existing **Cilium native routing** rides the underlay unchanged (`AllowedIPs` covers mesh IPs; pod CIDRs route node-to-node) | If pod-CIDR routes across the tunnel prove brittle → switch Cilium to vxlan tunnel mode |
| Encryption | WireGuard underlay only; Cilium transparent encryption **off** (no double encryption) | Compliance requires pod-level encryption inside a site |
| Storage | **Per-site pools** — no stretch Ceph over WAN. Site 1: Rook-Ceph. Site 2: own Rook cluster or Atlas-fronted external Ceph | <2 ms inter-site RTT makes a stretch cluster debatable |
| Migration | Within-site only (VMs zone-pinned) | vGPU Phase 2 does not change this — storage locality still rules |

Why an underlay at all: the k3s control plane (6443/tcp), kubelet (10250), and
Cilium health ports must be mutually reachable across sites regardless of CNI.
Cilium's own WireGuard feature encrypts pod traffic but does not create that
reachability. `--flannel-backend=wireguard-native` is not an option here —
flannel is disabled in this stack (Cilium is the CNI).

## Runbook

### 0. Prereqs

- Site-1 k3s server reachable on a static IP; udp/51820 open from site 2.
- Every worker passes `./scripts/cluster/preflight-node.sh <host> <user>`
  (add `--gpu` for GPU nodes — see [GPU_PASSTHROUGH.md](GPU_PASSTHROUGH.md)).

### 1. Initialize the mesh hub (once, site-1 server)

```bash
./scripts/cluster/join-remote-worker.sh --init-server <site1-public-ip> root
```

Installs `wireguard-tools`, creates `wg-veyron` = `10.88.0.1/24`, and re-binds
k3s to the mesh IP (`node-ip` / `advertise-address` in
`/etc/rancher/k3s/config.yaml` — survives k3s upgrades), keeping the public IP
as `node-external-ip`.

### 2. Join each site-2 worker

```bash
V9S_SERVER_HOST=<site1-public-ip> V9S_SITE=site-2 \
  ./scripts/cluster/join-remote-worker.sh <worker-ip> root
```

Per worker this: runs the node preflight → installs WireGuard → allocates the
next free mesh IP → exchanges peers with the server (idempotent per pubkey) →
verifies handshake + 1420-MTU path → installs the k3s agent
(`K3S_VERSION` from `scripts/cluster/versions.env`, `K3S_URL=https://10.88.0.1:6443`,
`--node-ip <mesh-ip>`) → labels the node `topology.kubernetes.io/zone=site-2`
(+ `veyron.io/gpu-passthrough=true` with `V9S_GPU_PREFLIGHT=1`).

Tailscale variant: `V9S_MESH_BACKEND=tailscale V9S_TAILSCALE_AUTHKEY=tskey-…` —
same flow, mesh IPs come from the tailnet.

### 3. Verify

```bash
kubectl get nodes -o wide          # worker Ready, INTERNAL-IP = 10.88.0.x
wg show                            # handshakes < 2 min on both ends
kubectl -n kube-system exec ds/cilium -- cilium status   # health OK
# pod-to-pod across the tunnel:
kubectl run t1 --image=busybox --overrides='{"spec":{"nodeSelector":{"topology.kubernetes.io/zone":"site-2"}}}' --restart=Never -- sleep 300
kubectl exec t1 -- ping -c2 <pod-ip-on-site-1>
```

First rehearsal can be a spare box (or a VM) joined as a fake "site-2" before
rack 2 exists. This flow was verified end-to-end (2026-07-18) with two KubeVirt
VMs on the lab node as fake site-1/site-2 — two rehearsal caveats came out of it:

- **`V9S_SKIP_PREFLIGHT=1`** — VMs have no `/dev/kvm`, so the KubeVirt node
  preflight must be skipped in rehearsal mode. Real workers must pass it.
- **Guest k3s CIDRs must not collide with the host cluster's.** A rehearsal
  k3s server inside a KubeVirt VM defaults to service CIDR `10.43.0.0/16` —
  the same as the host cluster — so the guest's kube-proxy hijacks the host
  DNS ClusterIP (`10.43.0.10`) that the guest itself resolves through: DNS
  blackhole. Install the rehearsal server with
  `--cluster-cidr 10.44.0.0/16 --service-cidr 10.45.0.0/16 --cluster-dns 10.45.0.10`.
  (Also applies to any real worker that is itself a VM on another KubeVirt cluster.)

### 4. Storage locality

- VM root disks: RBD block (RWX) per site; `vmStateStorageClass`: CephFS per
  site — `./scripts/cluster/adapt-existing-cluster.sh` picks this split.
- Site-suffixed StorageClasses (`ceph-rbd-site2`) carry `allowedTopologies` on
  `topology.kubernetes.io/zone` so PVCs bind site-locally.
- `adapt-existing-cluster.sh` warns when node zones span multiple values but
  every StorageClass is zone-agnostic.

### 5. Zone-pin VMs

Create VMs with a zone selector so the scheduler — and any live migration —
stays within a site:

```json
{ "name": "web-1", "scheduling": { "node_selector": { "topology.kubernetes.io/zone": "site-2" } } }
```

Cross-site recovery is a **DR flow** (Velero backups, or Atlas RBD
`export-diff` to the other site's RGW), never a live migration.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| No WG handshake | udp/51820 blocked toward the server's public IP |
| Handshake OK, node NotReady | k3s agent joined via public IP — check `--node-ip` is the mesh IP |
| Pods on worker can't reach site-1 pods | Cilium native-routing pod CIDR not covered by `AllowedIPs` (`10.88.0.0/24` covers only node IPs; node-to-node pod routes go via the node mesh IPs — check `cilium status` and node routes) |
| Small transfers fine, big ones hang | MTU — lower `MTU` in `wg-veyron.conf` on both ends (1380) |
| Cilium agent on worker can't reach the API | `k8sServiceHost: 127.0.0.1` requires the k3s agent's local apiserver LB; confirm `k3s-agent` is healthy, or set `k8sServiceHost: 10.88.0.1` in the Cilium values |
