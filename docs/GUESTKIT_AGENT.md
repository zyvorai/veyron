# GuestKit Agent Deployment (`veyron agent`)

Deploy the [Zyvor GuestKit](https://github.com/zyvorai/guestkit) in-guest agent
into Veyron-spawned VMs and verify it — Linux and Windows.

On KubeVirt the agent rides the QEMU guest-agent channel
(`org.qemu.guest_agent.0`) and answers the QGA protocol (`guest-info`,
`guest-exec`, `guest-network-get-interfaces`, `guest-fsfreeze-*`, …), so it
**doubles as the KubeVirt guest agent**: KubeVirt reports `AgentConnected` and
Veyron's guest-exec / GuestKit-RPC paths both work through it. The agent then
exposes ~80 JSON-RPC methods (evidence, doctor, migration assessment, live
metrics, integrity, security posture, file/service/storage ops, …).

## Commands

```
veyron agent deploy <vm> [--spawn] [--os linux|windows] [--template T]
                         [--bundle-url URL] [--iso URL] [--boot-pvc PVC]
                         [--cd-image REF] [--full-access] [--wait] [--dry-run]
veyron agent status  <vm>
veyron agent verify  <vm>
veyron agent methods <vm>
veyron agent rpc     <vm> <method> [--params '<json>']
```

| Command  | What it does |
|----------|--------------|
| `deploy` | Installs the agent into a VM. `--spawn` creates the VM first. `--wait` blocks until the agent connects and prints its version/capabilities. `--dry-run` renders manifests without applying. |
| `status` | Prints guest OS, `AgentConnected`, agent version, and RPC method count. |
| `verify` | Exercises the agent's JSON-RPC surface (version, capability list, a live metrics probe). |
| `methods` | Lists every RPC method the agent advertises (`getCapabilities`). |
| `rpc` | Invokes **any** agent method by name (the `guestkit.` prefix is optional). |

### Deploy flags

| Flag | Meaning |
|------|---------|
| `--spawn` | Create the VM first (Ubuntu for Linux; attach media for Windows). |
| `--bundle-url URL` | Linux agent binary (default: published `guestkitd`). |
| `--iso URL` | Windows agent ISO, imported via CDI (default: published bundle ISO). |
| `--boot-pvc PVC` | Boot an existing PVC as the Windows OS disk (e.g. a CDI-uploaded image). |
| `--cd-image REF` | Attach agent media as a CD from a containerDisk, or `pvc:<name>`. |
| `--full-access` | Also install the privileged executor helper (`guestkitd-exec`). Does **not** change the agent's security policy — capabilities stay gated by `/etc/guestkit/agent-policy.yaml`; an operator opts them in there. |

The agent binary / ISO default to the published GuestKit release
(`guestkit-agent-v0.3.14`); override with `--bundle-url` / `--iso`.

## Invoking the full agent surface

The agent exposes ~79 JSON-RPC methods. On KubeVirt they are reached over the
QGA channel through a single generic passthrough (`guestkit-rpc`), so **every**
method is callable — not just a hand-picked few:

```console
$ veyron agent methods gk-linux            # list all methods
$ veyron agent rpc gk-linux security.posture
$ veyron agent rpc gk-linux getEvidence
$ veyron agent rpc gk-linux packages.inventory
$ veyron agent rpc gk-linux migration.assess --params '{"target":"kvm"}'
```

Mutating and privileged methods (service control, storage expand, package
install, customization, migration repair, file ops, shell exec) are gated by
the agent's policy and are **off by default**; enable the ones you need in
`/etc/guestkit/agent-policy.yaml` inside the guest. Read-only methods (evidence,
health, metrics, inventories, posture, migration assessment) work out of the box.

## Linux

`--spawn` creates an Ubuntu VM whose cloud-init downloads the static `guestkitd`
from the release and runs it on the QGA channel via a `guestkit-agent` systemd
unit. The VM needs egress (Veyron's internet policy is applied automatically).

```console
$ veyron agent deploy gk-linux --spawn --os linux --wait
==> Creating VM 'gk-linux' with GuestKit agent (guestkit-agent-v0.3.14)
    agent source: https://github.com/zyvorai/guestkit/releases/download/guestkit-agent-v0.3.14/guestkitd
==> VM 'gk-linux' created and starting
==> Waiting for the GuestKit agent to connect (KubeVirt AgentConnected)…
    ✓ AgentConnected — GuestKit is answering the QGA channel
    ✓ GuestKit agent v0.3.14 (protocol 1.3) responding
    ✓ 79 RPC methods advertised

$ veyron agent verify gk-linux
GuestKit v0.3.14 (protocol 1.3)
Advertised RPC methods (79): guestkit.ping, guestkit.getEvidence, …
live metrics probe: {"cpu":{"usage_percent":4.76},"disk":{"usage_percent":83.9},…}
```

To install into an **already-running** Linux VM (that has a guest agent),
drop `--spawn`: Veyron installs via guest-exec.

## Windows

The GuestKit bundle ISO is imported into a PVC by CDI and attached as a CD-ROM;
Cloudbase-Init installs the MSI on first boot (registers the `GuestKitAgent`
service). Render the manifests to apply via GitOps:

```console
$ veyron agent deploy gk-win --os windows --dry-run
# GuestKit ISO import (CDI DataVolume) …
# Windows VM with GuestKit ISO attached + Cloudbase-Init install …
```

Spawning a Windows guest also requires a Windows OS disk/base image on the
cluster; apply the rendered DataVolume + VM once that base image is present.

## Notes

- **DNS under masquerade** — the Linux cloud-init pins working resolvers
  (cluster CoreDNS + public) so the release download resolves.
- **guest-exec disabled** — where KubeVirt's `guest-exec` subresource is off,
  `status`/`verify` fall back to `virsh qemu-agent-command` in the
  virt-launcher pod (the GuestKit JSON-RPC path), which always works.
- The public release is also registered as a fallback source in the standard
  Linux templates' GuestKit install list.
