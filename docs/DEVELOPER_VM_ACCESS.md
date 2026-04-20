# Developer access to KubeVirt VMs (SSH and consoles)

This guide explains how **application developers** and **platform engineers** typically reach guest shells on VMs managed by VMRogue, and how that relates to **VMRogue’s dashboard** and **API**.

## What VMRogue shows today

### Web dashboard

On **Virtual Machines**, each running Linux VM includes an **SSH** block (fleet roster and expanded detail) with:

1. **TCP / guest IP** — Standard OpenSSH when the VMI reports a guest address, e.g.  
   `ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null <user>@<guest-ip>`  
   The default **user** is a best-effort guess from the VM name (`ubuntu` for names containing `ubuntu`, otherwise `root` unless the name suggests Windows, in which case the UI points to VNC/RDP instead of SSH).

2. **Through the Kubernetes API** — KubeVirt’s client, e.g.  
   `virtctl ssh --user <user> <vm-name> -n <namespace>`  
   Use the **Copy** buttons to paste exact commands.

### REST API

- `GET /api/v1/vms?namespace=<ns>|all` — VM list includes **guest IP and node** when a matching **VirtualMachineInstance** exists (merged in one pass; no per-VM N+1 probe for IPs in the list handler).
- `GET /api/v1/vms/:ns/:name` — VM detail includes the same plus **VMI status** (interfaces, etc.).

Windows guests are not SSH targets in the UI; use **VNC** from the dashboard or **RDP** from a host that can reach the guest network.

---

## Recommended ways developers connect

### 1. Baseline: `virtctl ssh` (works with kubeconfig only)

If a developer already has **`kubectl`** access to the cluster, install the KubeVirt **`virtctl`** binary (see the [KubeVirt user guide — virtctl](https://kubevirt.io/user-guide/user_workloads/accessing_virtual_machines/)).

Example:

```bash
virtctl ssh --user ubuntu sample-ubuntu -n default
```

**Why this is the default recommendation**

- Traffic is carried over mechanisms supported by KubeVirt; you do **not** need your laptop to route to the overlay **guest IP**.
- Access is aligned with **existing** Kubernetes RBAC and audit patterns.

**Requirements**

- Valid kubeconfig (and any VPN / Zero Trust fronting the **API server** your org uses).
- `virtctl` installed and compatible with your KubeVirt version.

### 2. Plain `ssh user@guest-ip` (when IP is routable)

When the guest IP (shown in VMRogue) is reachable from the developer’s machine—**same L2**, **corporate VPN**, **split tunnel** to the pod/overlay network, or another approved path—standard SSH is fine:

```bash
ssh ubuntu@10.0.0.251
```

**Caveat:** Many clusters only expose guest IPs **inside** the cluster or a specific segment. If `ping` / `ssh` to that IP fails from a laptop, use option **1** or **3**.

### 3. Bastion or jump host

Some teams place a small **bastion** VM or pod in a DMZ or well-known network segment. Developers SSH to the bastion, then SSH (or `virtctl`) onward according to policy. This centralizes logging, MFA, and firewall rules.

---

## Why VMRogue does not auto-create NodePort (or LB) for SSH

- A **NodePort `Service`** targets **Pod** endpoints in Kubernetes. Reaching **SSH inside the guest** depends on your **KubeVirt + CNI** design (default pod network, Multus, bridge, etc.). It is not universally “one Service per VM.”
- Exposing SSH on **node IPs + wide port ranges** is a **major security** and **policy** decision (many clusters forbid it or scope it with OPA/Gatekeeper).
- **LoadBalancer / external IPs** are platform-specific. **[Cilium](https://docs.cilium.io/)** can provide LB IPAM, BGP, and related datapath features; **MetalLB** is another common pattern on bare metal. VMRogue stays agnostic: your platform team chooses how **routable** IPs are assigned; VMRogue documents **commands**, not your firewall topology.

If you standardize on a specific pattern (e.g. “all devs use VPN + guest IP” or “all devs use `virtctl ssh`”), document that in your internal runbook and optionally extend VMRogue or your GitOps layer with **opt-in** automation (Services, LB annotations) that matches **your** network model.

---

## CLI (outside the dashboard)

VMRogue’s CLI includes SSH helpers that resolve guest IP when possible, with fallback to `virtctl`:

```bash
vmrogue ssh <vm-name> --user root
```

See `vmrogue ssh --help` for flags and namespace handling.

---

## Summary

| Approach              | Typical need                         | Notes                                      |
|-----------------------|--------------------------------------|--------------------------------------------|
| `virtctl ssh`         | Laptop has API access + `virtctl`    | Good default for distributed teams         |
| `ssh user@guest-ip`   | Laptop routes to guest / overlay     | Needs network design + often VPN           |
| Dashboard VNC         | Break-glass / Windows / no SSH       | Browser console via KubeVirt VNC subresource |
| Bastion               | Strict egress / centralized access   | Operational overhead, strong control       |

For questions about **Cilium**, **BGP**, or **LoadBalancer** classes, refer to your cluster’s networking documentation; VMRogue surfaces **observed guest IPs** and **copy-paste SSH / virtctl** commands so developers can use whichever path your platform supports.
