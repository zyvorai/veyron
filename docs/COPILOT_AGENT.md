# Copilot agent: search, plan, approve, execute

The console's one-stop assistant. Operators search every resource, ask questions, and have the agent
draft VM/node operations that they approve explicitly. Requires `VEYRON_AI_MODE=agent` and an
OpenAI-compatible endpoint (`VEYRON_AI_URL`, `VEYRON_AI_API_KEY`); without an LLM the keyword
fallback still answers read-only questions, and no plans are drafted.

## Console

In the Veyron AI drawer (`Assistant.jsx`), Write+ users get a **Plan actions** checkbox. When on, the
message goes to the Copilot agent (`/experience/copilot/chat`) instead of the streaming `/ai/chat`
agent. Its answer and evidence are shown, and any drafted plans appear as approval cards
(`CopilotPlanCard.jsx`): steps with blocked reasons, destructive steps needing typed VM names,
Approve & run / Cancel, and per-step results. The default mode (streaming `/ai` agent with its own
proposals, see `docs/ai.md`) is unchanged.

## Safety model

1. **The LLM never mutates.** `propose_vm_action` and `propose_node_action` only *draft* a plan
   (max 3 plans per turn, max 25 steps each). Nothing runs until a human approves it.
2. **Caller's role always.** Chat is ReadOnly-accessible, but plans are drafted only for Write+
   callers. Execute/cancel require Write; each step must also satisfy the role of its equivalent
   route (e.g. delete/migrate steps may require Admin). The agent can never exceed the caller.
3. **Single-use, expiring.** A plan is a ConfigMap (`veyron.io/type=copilot-plan`,
   `veyron-plan-<id>`) in the API default namespace, claimed atomically on execute (no double run),
   expiring after `VEYRON_AI_PLAN_TTL_SECS` (410 when expired, 409 when already used).
   Plans stay readable for an hour after expiry, then the next plan drafted prunes them
   (best effort); the SOC audit events are the permanent record.
4. **Creator or admin.** Only the plan's creator, or an Admin, can execute or cancel it.
5. **Destructive steps** (`delete`) require the caller to type each VM name back
   (`confirm_names`).
6. **Migration guard.** `migrate` steps are checked with the same eligibility logic as the direct
   API (e.g. GPU passthrough VMs); blocked steps are flagged in the plan.
7. **Stop on error** by default (`stop_on_error: true`); per-step results are returned.
8. **Per-target cooldown.** After a step succeeds (or is still running) on a VM or node, any other
   plan's step on the same target is refused for `VEYRON_AI_PLAN_COOLDOWN_SECS` (default 60,
   `0` disables). New plans show such steps as blocked ("cooling down … retry in Ns") and execute
   re-checks. Failed or skipped steps never lock a target. It reads the plan ConfigMaps, so it is
   best effort: two plans executed in the same instant on different API replicas can both pass.

Actions: VM `start|stop|restart|pause|unpause|migrate|delete|snapshot`; node `cordon|uncordon`.
`snapshot` takes a `<vm>-snap-<timestamp>` snapshot (a disk snapshot, or a VM-state checkpoint on
Kairon Machines without PVCs). It needs Write, is not destructive, and is exempt from the cooldown in
both directions, so a plan can "snapshot, then change" without waiting.
Resize: the `propose_vm_resize` tool drafts `hotplug` steps (`cpus` = CPU sockets, `memory` such as
`512Mi`/`4Gi`/`1Ti`, at least one, up to 25 VMs). It needs Write and runs the same backend call as
`POST /vms/:ns/:name/hotplug`: KubeVirt applies it live under LiveUpdate (otherwise at next restart);
Kairon hotplugs live, grow-only, within the Machine's `maxCpu`/`maxMemory`. Shrinking or exceeding the
maximums is rejected by the backend when the step runs, and the plan shows that error on the step.

## API

| Route | Role | Purpose |
|---|---|---|
| `GET /api/v1/search?q=&limit=&namespace=` | ReadOnly | Cross-resource search |
| `POST /api/v1/experience/copilot/chat` | ReadOnly | Agent turn; `plans[]` in response for Write+ |
| `GET /api/v1/experience/copilot/plans/:id` | ReadOnly | Inspect a plan |
| `POST /api/v1/experience/copilot/plans/:id/execute` | Write (+ per step) | Body `{confirm_names, stop_on_error}` |
| `DELETE /api/v1/experience/copilot/plans/:id` | Write | Cancel |

## Audit

Each executed step is appended to the SOC event store with actor, action, target, namespace/VM,
outcome, and labels `plan_id`, `via=copilot-agent`, `prompt`. Actor is the caller's `AuthContext.subject` (API key name, local user, or JWT/OIDC `sub`).

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `VEYRON_AI_MODE` | `routing` | Set `agent` for the tool-calling loop |
| `VEYRON_AI_MAX_TOOL_ROUNDS` | `6` | Max LLM↔tool rounds per turn |
| `VEYRON_AI_PLAN_TTL_SECS` | `900` | Plan lifetime before it expires |
| `VEYRON_AI_PLAN_COOLDOWN_SECS` | `60` | Per-VM/node cooldown between plan steps (`0` = off) |

## RBAC

Plans use ConfigMaps in the API namespace; the `veyron` ClusterRole already grants the needed
verbs (`deploy/k8s.yaml`, `charts/veyron/templates/clusterrole.yaml`). Nothing new to add.

## Not yet implemented

Snapshot, hotplug and self-heal plan steps; audit actor in the generic
HTTP middleware (still `api`); unattended auto-execution (deliberately out of scope).
