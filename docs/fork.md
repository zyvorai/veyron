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
