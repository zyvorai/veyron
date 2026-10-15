# Fork Lab: live-fork a running VM

Kairon + FluxVM can turn one **Running** VM into N live copies. The parent is
paused for a few milliseconds while FluxVM snapshots it; each child restores
from that snapshot with the parent's memory, CPU state and a copy-on-write copy
of its disk, so processes that were running keep running in every child.

Use it to rehearse a risky upgrade on identical copies, race a bug across
variants, or fan an AI agent out to several trials, then delete the losers.

## API

| Route | Role | Purpose |
|---|---|---|
| `POST /api/v1/vms/:ns/:name/fork` `{count, name_prefix?}` | Write | Create `count` children `<prefix>-<n>` (prefix defaults to the VM name). `202` with the children. |
| `GET /api/v1/vms/:ns/:name/forks` | ReadOnly | Children of the VM (label `kairon.zyvor.dev/forked-from`). |
| `DELETE /api/v1/vms/:ns/:name` | Write | Delete a child like any VM. |

Children are ordinary Machines carrying the annotation
`kairon.zyvor.dev/fork-from` and are pinned to the parent's node.

Errors: `422 INVALID_COUNT` / `INVALID_NAME_PREFIX`, `404 NOT_FOUND`,
`409 FORK_BLOCKED` (not Running, has volumes/disks/device claims, qemu or
cloud-hypervisor backend, no node yet), `409 FORK_LIMIT`,
`501 KAIRON_REQUIRED` on KubeVirt.

## Limits

- The parent must run on the FluxVM `flux-vm` backend (or be a sandbox Machine)
  and have only its boot disk. Children keep the parent's MAC/IP and hostname.
- At most `VEYRON_FORK_MAX` children per VM (default 10, hard cap 32).
- Stopping a child discards its runtime; starting it again forks the parent's
  *current* state.

## Console and AI

The VM action **Fork live** asks for a copy count. The AI tool `fork_vm` drafts
a proposal that replays `POST …/fork` once approved ("fork prod-db 3 times and
run the migration on each").

## E2E

`scripts/test-vm-daily-ops-remote.sh` checks validation (`count=0` → 422) and,
when the test VM is forkable, fork → list → delete; otherwise it records the
`409 FORK_BLOCKED` contract as a skip.

## Agent Lab (sandboxes)

The same primitive powers agent sandboxes on the Kairon backend (`docs/ai.md`):

| Route | Purpose |
|---|---|
| `POST /api/v1/sandboxes/:id/fork` `{count}` | Branch a ready sandbox into N live copies (`VEYRON_SANDBOX_MAX_FORKS`, default 5; total cap `VEYRON_SANDBOX_MAX_BRANCH_TOTAL`, default 10). |
| `POST /api/v1/sandboxes/:id/pick` `{keep}` | Keep one member of the branch group; the parent and other copies are deleted. |
| `GET …/checkpoints`, `POST …/undo` `{name?}`, `POST …/pin` `{name}` | Every exec is preceded by a VM-state checkpoint (`VEYRON_SANDBOX_AUTO_CHECKPOINT`, ring of `VEYRON_SANDBOX_CKPT_KEEP`=5, pinned ones kept). Undo restores in place. |
| `GET …/ledger` | Per-sandbox run history (actor, command, exit, duration, checkpoint) + summary. Commands are truncated to 500 chars; output is never stored. `VEYRON_SANDBOX_LEDGER_KEEP`=200. |
| `GET/PUT/DELETE …/egress` | Default-deny allowlist (`allow_cidrs/ports/fqdns/dns/sni/icmp`) via a kairon `MachineNetworkPolicy`; `enforced` is true only once kairon reports the Machine applied. |
| `GET/POST/DELETE /api/v1/agent-sessions…` | Kairon `AgentSession` CRUD; 501 `AGENT_SESSION_UNAVAILABLE` without the CRD. Needs a FluxVM agent-session node (Mac today); verbs (rewind/branch/bisect) are not proxied. |

Idle suspend: set `VEYRON_SANDBOX_IDLE_SECS` (and `VEYRON_SANDBOX_IDLE_ACTION`, default `Pause`) to write
`spec.lifecycle` on new sandboxes; the next exec wakes a `paused` sandbox. Warm pool: set
`VEYRON_SANDBOX_KAIRON_POOL=<MachinePool>` to claim via `MachineClaim` instead of creating a Machine.
AI tools: `sandbox_fork`, `sandbox_pick`, `sandbox_undo`.
