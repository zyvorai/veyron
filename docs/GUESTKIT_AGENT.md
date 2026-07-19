# GuestKit Agent Deployment (`veyron agent`)

Deploy the [Zyvor GuestKit](https://github.com/hypersdk/guestkit) in-guest agent
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
                         [--bundle-url URL] [--iso URL] [--wait] [--dry-run]
veyron agent status <vm>
veyron agent verify <vm>
```

| Command  | What it does |
|----------|--------------|
| `deploy` | Installs the agent into a VM. `--spawn` creates the VM first. `--wait` blocks until the agent connects and prints its version/capabilities. `--dry-run` renders manifests without applying. |
| `status` | Prints guest OS, `AgentConnected`, agent version, and RPC method count. |
| `verify` | Exercises the agent's JSON-RPC surface (version, capability list, a live metrics probe). |

The agent binary / ISO default to the published GuestKit release
(`guestkit-agent-v0.3.14`); override with `--bundle-url` (Linux `guestkitd`) or
`--iso` (Windows bundle ISO).

## Linux

`--spawn` creates an Ubuntu VM whose cloud-init downloads the static `guestkitd`
from the release and runs it on the QGA channel via a `guestkit-agent` systemd
unit. The VM needs egress (Veyron's internet policy is applied automatically).

```console
$ veyron agent deploy gk-linux --spawn --os linux --wait
==> Creating VM 'gk-linux' with GuestKit agent (guestkit-agent-v0.3.14)
    agent source: https://github.com/hypersdk/guestkit/releases/download/guestkit-agent-v0.3.14/guestkitd
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
