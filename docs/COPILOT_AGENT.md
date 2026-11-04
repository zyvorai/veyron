# Copilot agent: search, plan, approve, execute

The console's one-stop assistant. Operators search every resource, ask questions, and have the agent
draft VM/node operations that they approve explicitly. Requires `VEYRON_AI_MODE=agent` and an
OpenAI-compatible endpoint (`VEYRON_AI_URL`, `VEYRON_AI_API_KEY`); without an LLM the keyword
fallback still answers read-only questions, and no plans are drafted.

## Console

- **Ctrl/Cmd+K** — command palette. Instant search over VMs, pods, nodes (hosts) and PVCs.
  Filters: `kind:vm|pod|node|pvc`, `ns:<name>`, `status:<text>`, `os:<text>`. Enter opens the
  resource; the first row, or a `>` prefix, sends the text to the agent.
- **Sparkles button (top bar)** — agent drawer: chat, evidence, tools used, and **plan cards**.

## Safety model

1. **The LLM never mutates.** `propose_vm_action` and `propose_node_action` only *draft* a plan
   (max 3 plans per turn, max 25 steps each). Nothing runs until a human approves it.
2. **Caller's role always.** Chat is ReadOnly-accessible, but plans are drafted only for Write+
   callers. Execute/cancel require Write; each step must also satisfy the role of its equivalent
   route (e.g. delete/migrate steps may require Admin). The agent can never exceed the caller.
3. **Single-use, expiring.** A plan is a ConfigMap (`veyron.io/type=copilot-plan`,
   `veyron-plan-<id>`) in the API default namespace, claimed atomically on execute (no double run),
   expiring after `VEYRON_AI_PLAN_TTL_SECS` (410 when expired, 409 when already used).
4. **Creator or admin.** Only the plan's creator, or an Admin, can execute or cancel it.
5. **Destructive steps** (`delete`) require the caller to type each VM name back
   (`confirm_names`).
6. **Migration guard.** `migrate` steps are checked with the same eligibility logic as the direct
   API (e.g. GPU passthrough VMs); blocked steps are flagged in the plan.
7. **Stop on error** by default (`stop_on_error: true`); per-step results are returned.

Actions: VM `start|stop|restart|pause|unpause|migrate|delete`; node `cordon|uncordon`.

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
outcome, and labels `plan_id`, `via=copilot-agent`, `prompt`. Actor is the API key name, `primary`,
or `bearer`.

**Limitation:** JWT/OIDC bearer callers share the single actor `bearer`; per-user attribution needs
the token subject plumbed into `AuthContext`.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `VEYRON_AI_MODE` | `routing` | Set `agent` for the tool-calling loop |
| `VEYRON_AI_MAX_TOOL_ROUNDS` | `6` | Max LLM↔tool rounds per turn |
| `VEYRON_AI_PLAN_TTL_SECS` | `900` | Plan lifetime before it expires |

## RBAC

Plans use ConfigMaps in the API namespace; the `veyron` ClusterRole already grants the needed
verbs (`deploy/k8s.yaml`, `charts/veyron/templates/clusterrole.yaml`). Nothing new to add.

## Not yet implemented

Snapshot, hotplug and self-heal plan steps; per-VM cooldown loop guard; audit actor in the generic
HTTP middleware (still `api`); unattended auto-execution (deliberately out of scope).
