# Kairon / FluxVM operations

Routes available when the VM backend is Kairon (the default). On KubeVirt they answer
`501`. Every route lives under `/api/v1`; roles follow `src/api/auth_context.rs`
(GET = read-only unless noted).

## Through the kairon-node relay

Needs `VEYRON_KAIRON_NODE_URL` and `KAIRON_NODE_CONSOLE_TOKEN` (the `veyron-kairon` Secret).
The relay URL must contain `{node_ip}`; pin the node's relay with `--console-port` or the
port changes on the next deploy.

| Route | Notes |
|---|---|
| `POST /vms/:ns/:name/files/put`, `files/get` | Guest file transfer, up to 16 MiB. vsock agent first, base64 over guest exec as fallback. |
| `POST /vms/:ns/:name/diagnostics`, `GET/DELETE …/:token`, `GET …/:token/:file` | Incident bundle. `memory-dump` is Admin. Download needs Write. |
| `POST /vms/:ns/:name/freeze`, `thaw`, `GET frozen`, `POST quiesced-snapshot` | The snapshot always thaws, even on failure (207 when the thaw itself failed). |
| `GET /vms/:ns/:name/pressure`, `cpuset` | Guest PSI and pinned CPUs. |
| `GET /vms/:ns/:name/network/{effective,stats,flows,drop-reasons,drops}` | eBPF view; `stats`/`flows` 502 when the map is not pinned for the VM. |
| `POST/GET /vms/:ns/:name/network/capture`, `GET …/capture/:token` | pcap capture (Write). |
| `POST /vms/:ns/:name/egress-check` | Would this guest reach a host. Vault credentials in the reply are redacted. |
| `GET /vms/:ns/:name/runtime-capabilities` | Node-level capabilities. |
| `GET /vms/:ns/:name/tunnel?port=N` | TCP-over-WebSocket (ws ticket auth, Write). |
| `POST /vms/:ns/:name/fork/plan` | Dry run of a fork: refusals and predicted cost. Body fields: `count` (1–32), `name_prefix`, `identity`, `network`, `parallelism`, `commands`. |
| `GET /vms/:ns/:name/runtime-events` | FluxVM lifecycle log for the VM (`since`, `event`, `limit`). `GET …/events` is the Kubernetes events route. |
| `GET /nodes/:name/runtime/memory` | Host memory and PSI. |
| `GET /nodes/:name/branches`, `…/branches/:a/diff/:b`, `POST …/branches/:id/start-plan` | FluxVM branches (read and dry run). |
| `POST /nodes/:name/branches/:id/start`, `DELETE …/branches/:id` (Admin) | Start VMs from a saved branch (same body as fork) or delete it. |
| `GET /nodes/:name/runtime/topology`, `…/runtime/pressure` | Host NUMA/CPU topology and PSI. |
| `GET /nodes/:name/egress-nftables` | `{"snippet": …}` — the nftables redirect rules FluxVM expects on the host. |
| `GET /nodes/:name/sandbox-templates` | Prebaked sandbox templates the node starts in under a second. |
| `GET /vms/:ns/:name/runtime-events/stream` | Live FluxVM events as server-sent events. Veyron's 30 s HTTP timeout still applies, so clients (`EventSource`) reconnect about every 30 s. |
| `GET/POST /agent-sessions/:ns/:name/ops/*rest` | Allow-listed AgentSession verbs: exec, checkpoint, restore, rewind, fork, promote, events, artifacts, trajectory, preview(s), jobs, git, approvals, grants, probe. `approvals`/`grants`/`promote` are Admin. Binary and credential verbs (screenshot, pty, tokens, ssh) are not relayed. |

The relay routes for fork plan, host memory, events and branches exist from kairon
`feat/relay-flux-routes` (`internal/consoleproxy/flux_ext.go`); older kairon-node builds
answer 404, which Veyron reports as 502.

## Through the Machine CRDs

| Route | Notes |
|---|---|
| `GET/PUT /vms/:ns/:name/lifecycle` | `idleAction` (Pause, Hibernate, Halt, Stop, Delete), `idleTimeoutSeconds`, `maxLifetimeSeconds`, `wakeOnConnect`. `{}` clears. |
| `GET/PUT /vms/:ns/:name/placement` | Affinity, anti-affinity, preferred variants, spread, tolerations. Admission-webhook denials return 422. |
| `GET/PUT/DELETE /vms/:ns/:name/network-policy` | `MachineNetworkPolicy` `<vm>-netpol`. Unknown fields are refused. A policy still being removed reads as 404 and a `PUT` answers 409. |
| `POST /vms/:ns/:name/migrate` | Extra body fields: `mode`, `bandwidth_mbps`, `max_downtime_ms`, `multifd_channels`, `fallback_to_cold`, `migration_network`, `adaptive`. |
| `POST /vms/:ns/:name/migrations/:mig/recovery` (Admin) | `action`, `acknowledged_diagnosis`, `reason` — resolves a migration stuck at an ambiguous commit point. |
| `GET /kairon/advice/migrations` | Migration advisor: running VMs to move off nodes whose PSI or free memory is over threshold (CPU 25 %, memory 10 %, IO 30 % stalled, or under 10 % memory free), target chosen by free memory with headroom, at most 3 VMs per node, passthrough VMs excluded. Advice only; each entry carries the `migrate` body. |
| `POST /kairon/fleet/:kind`, `DELETE /kairon/fleet/:kind/:ns/:name` (Admin) | Create or delete a fleet object (`ha-profiles`, `balance-policies`, `autoscalers`, `recovery-plans`, `backup-groups`, `import-plans`, `template-versions`, `template-claims`, `virtual-networks`, `network-claims`). Body `{name, namespace, spec}`; Kairon's admission webhook validates the spec (422 with its message). Usage ledgers, node-fence requests and action approvals are read-only here: approvals need Kairon's authenticated flow. |
| `GET /vms/:ns/:name/screenshot[?max_width=]`, `POST /vms/:ns/:name/input` (Admin) | PNG of a running **Mac (`vz`)** VM's display with `x-fluxvm-screen-*` headers, and keyboard/mouse input (`{"action":"click","x":..,"y":..}` or `{"actions":[...]}`, up to 100). Other backends answer 502. FluxVM has no display-resize or USB endpoints, so those are not offered. |
| `GET /sandbox-templates` | Prebaked sandbox templates per Kairon-capable node (`templates` is the union). `POST /sandboxes` takes `template`; the sandbox then runs on FluxVM's `flux-vm` backend with the vsock agent, and exec/files use it. |
| `GET /kairon/:kind?namespace=` | Read-only: `claims`, `quotas`, `disruption-budgets`, `migration-policies`, `backup-policies`, `protection-plans`, `accelerator-profiles`, `node-topologies` (core group `kairon.zyvor.dev`; the last two are cluster-scoped) and the fleet group `fleet.kairon.zyvor.dev/v1alpha1`: `virtual-networks`, `network-claims`, `balance-policies`, `autoscalers`, `recovery-plans`, `ha-profiles`, `backup-groups`, `import-plans`, `template-versions`, `template-claims`, `usage-ledgers`, `node-fence-requests`, `action-approvals`. |
| `POST/GET /containers`, `GET/DELETE /containers/:ns/:name` | OCI container Machines (`spec.sandbox.oci`). FluxVM runs these on **macOS hosts only**; they get a `kubernetes.io/os=darwin` selector unless `node_selector` says otherwise. No exec or files yet. Env names that look like credentials produce a warning: Machine objects are not secret. |

## Network access (per-VM egress)

A guest on QEMU user-mode (slirp) networking has no host-visible interface: its traffic is
re-originated by the VM's QEMU process, which FluxVM governs with an eBPF hook on the VM's
cgroup. That hook used to allow loopback only, so such a guest reached nothing at all — not the
package mirror, not Veyron (no guest agent, no exec/files). It is now policy-driven:
`spec.network.egress` on the Machine = `{mode, rules[]}`, `mode` one of `loopback` (default),
`allowlist`, `audit` (allow everything, record what an allow-list would refuse) or `open`; a rule is
`{cidr, proto: any|tcp|udp, port, note}`. IPv4 only. Because the kernel returns only the longest
matching prefix, a narrow rule inherits the ports of covering rules, and one network may carry at
most **4 distinct (proto, port)** after that; Veyron validates exactly like the node and answers 422
with the offending network.

**The Machine spec is the source of truth.** kairon-node reconciles it over FluxVM, so Veyron writes
by patching `spec.network.egress` (pinned to the `resourceVersion` it read; 409 on a concurrent edit)
and uses the relay only to read live state, observed flows and suggestions, and for node defaults.

**Cilium rewrites before the filter.** Socket LB turns a ClusterIP/NodePort destination into the
backend pod address first, so the filter sees `<pod ip>:5151`, not the node IP. Platform rules
therefore cover the pod network (`VEYRON_POD_CIDR`, else the pod's `POD_IP` /16, else `10.0.0.0/8`)
on the API port, plus the service IP (`VEYRON_API_CLUSTER_IP` / `VEYRON_API_SERVICE_HOST`), the node
NodePort and cluster DNS. `GET /egress/presets` returns them resolved for this cluster. `hostPort`
traffic to the node IP is not rewritten; guests use the NodePort.

| Route | |
|---|---|
| `GET /egress/presets[?template=]` | Isolated · Platform only · Platform + package mirrors (Ubuntu: Canonical archive ranges; others: public web ports, since their mirrors sit on CDNs) · Web access (public addresses on 80/443 + DNS, private ranges stay closed) · Private networks · Learn mode (audit) · Open. Each has `risk`, `mode`, resolved `rules`, `needs`. |
| `GET /vms/:ns/:name/egress` | Policy, what the node enforces (`programmed`), observed refused flows, `problems[]` with ready-made `fix` (blocked guest agent, drift, not enforced, open), pending timed `grant`. |
| `PUT /vms/:ns/:name/egress` (Admin) | `{preset?, mode?, rules?, expires_in?}`. A preset plus `rules` adds to it; `rules` alone replaces. `expires_in` (`30m`, `2h`, `1d`) records a ConfigMap `veyron.io/type=egress-grant` and the scheduler tick restores the previous policy. |
| `DELETE /vms/:ns/:name/egress` (Admin), `DELETE …/egress/grant` (Admin) | Follow the node default again; end a timed change now. |
| `GET …/egress/observed`, `POST …/egress/suggest` (Write) | Destinations refused (or, in audit mode, that would be); the rules that would allow them, with a validated preview of the merged policy. Applying is a `PUT` with the rules you accept. |
| `GET/POST …/egress/explain` | `{host, port, proto}` → per-address verdict, the allowing rule, or the single rule that would fix it. |
| `GET /egress/overview[?live=true]`, `POST /egress/apply` (Admin) | Fleet counts by mode; apply a preset/rules to a `selector` (`namespaces`, `names`, `labels`, or `all`). **Dry run by default**, at most 500 VMs. |
| `GET/PUT /nodes/:name/egress-defaults[?apply_running=true]` (Admin) | The node default for VMs without a policy of their own. |

`POST /vms` takes `network_access: {preset, mode, rules}`. Without it, a **Linux** VM on **user-mode**
networking gets the default preset (`platform-packages`, so the agent connects out of the box):
namespace annotation `veyron.io/default-egress-preset`, else `VEYRON_VM_DEFAULT_EGRESS_PRESET`
(`none` = inherit the node default). Bridged guests, Windows, ISO installs and VMs of unknown origin
are left to the node. Sandboxes get `platform-packages` (with internet) or `platform`
(`VEYRON_SANDBOX_EGRESS_PRESET` overrides). Every change goes through the mutating-request audit
trail. Needs FluxVM `bbd5fc38`+ and a kairon-node that reconciles the field; older nodes answer 404/502
on the live reads, which Veyron reports as "update FluxVM" instead of failing.

## Defaults

New VMs get a `spec.lifecycle` from the namespace annotations `veyron.io/default-idle-seconds`,
`veyron.io/default-idle-action` and `veyron.io/default-max-lifetime-seconds`, else from
`VEYRON_VM_DEFAULT_IDLE_SECS`, `VEYRON_VM_DEFAULT_IDLE_ACTION` and
`VEYRON_VM_DEFAULT_MAX_LIFETIME_SECS` (minimum 60 s; an idle timeout alone pauses, which is
reversible). A lifecycle set on the VM itself always wins.

## AI tools

Read: `vm_runtime_events`, `vm_pressure`, `vm_network_drops`, `vm_lifecycle`,
`vm_network_policy`, `vm_egress`, `egress_explain`, `egress_overview`, `node_runtime_memory`,
`migration_advice`, `kairon_resources`. Proposal-only changes: `set_vm_lifecycle`,
`set_vm_network_policy`, `set_vm_egress` (Admin), `quiesced_snapshot_vm`. They are also served over
`POST /mcp` with the same role filtering.

## Securing the relay

The kairon-node console relay (default `:8090`, or whatever `--console-port` pins) serves guest
exec, files, console and the TCP tunnel behind a single bearer token, and listens on every
interface. Close it to the outside on every node:

```bash
sudo scripts/cluster/restrict-kairon-relay.sh --port 22230            # Cilium pods (10.0.0.0/8)
sudo scripts/cluster/restrict-kairon-relay.sh --port 22230 --allow 10.42.0.0/16   # flannel k3s
sudo scripts/cluster/restrict-kairon-relay.sh --port 22230 --remove   # undo
```

It installs an `iptables`/`ip6tables` chain `KAIRON_RELAY` that accepts loopback, the node's own
address (Veyron's pod reaches the relay at `https://{node_ip}:PORT`) and the `--allow` ranges, drops
the rest, and a oneshot `kairon-relay-firewall.service` that restores it at boot. Verify from
outside the cluster (`nc -z NODE PORT` should time out) and through Veyron
(`GET /api/v1/nodes/NODE/runtime/memory` should still return 200). Applied on the 175 test node on
2026-10-21; a node whose relay port was found open to the internet should also rotate
`KAIRON_NODE_CONSOLE_TOKEN`.

## Drills

`scripts/nightly-kairon-drill.sh HOST` runs the full E2E and exits non-zero on failure, posting a
summary to `VEYRON_DRILL_WEBHOOK` when set. Run it from cron or CI.

## Not covered

Display resize and USB listing do not exist in FluxVM. `kaironctl mcp serve` is stdio-only and is not bridged: spawning a configured command inside the API pod would let anyone who can edit the MCP server list run code there; run it beside `veyron mcp` in your MCP client instead, or use Veyron's own `POST /mcp` tools. FluxVM
per-VM disk throttles and balloon are managed through the Machine spec (`appliedDiskThrottles`),
not relayed.

## Testing

`scripts/test-vm-daily-ops-remote.sh HOST` covers all of the above on a live node. The guest
file, quiesced-snapshot and eBPF stats checks are skipped when the guest has no agent or the
map is not pinned. `cargo test` also builds the full router (`full_router_builds_without_overlapping_routes`):
axum panics on overlapping routes, which otherwise shows up only as a crash loop after deploy.
