# Copilot agent: search, plan, approve, execute

The console's one-stop assistant. Operators search every resource, ask questions, and have the agent
draft VM/node operations that they approve explicitly. Requires `VEYRON_AI_MODE=agent` and an
OpenAI-compatible endpoint (`VEYRON_AI_URL`, `VEYRON_AI_API_KEY`) or Anthropic's Messages API
(`VEYRON_AI_PROVIDER=anthropic`, below); without an LLM the keyword
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
9. **Blast radius.** A plan that touches more targets than `VEYRON_AI_BULK_CONFIRM_THRESHOLD` (default 5) records
   its target count; the console then asks the user to type that number, and `execute` refuses with 400 unless the body
   carries a matching `confirm_count`. Deletes still need every name typed as well.
10. **Four-eyes approval (opt-in).** With `VEYRON_AI_FOUR_EYES` set, plans of the chosen kind need a second person
    before they can run. The value is a comma list of `destructive` (plans with a delete), `admin` (plans that need
    the Admin role) and `all`; unset or `off` disables it. The plan records `needs_approval` when it is drafted, so
    changing the setting never alters plans that already exist. Another user with Write calls
    `POST .../plans/:id/approve` (at least the role the plan needs); the creator can never approve their own plan,
    not even as an admin. `execute` answers 403 until it is approved. Approving does not run the plan: the creator
    (or an admin) still executes it. Approval is audited (`plan_approved`). The console shows an
    "Awaiting your approval" list in the assistant, and the creator's card updates when the approval arrives.

Actions: VM `start|stop|restart|pause|unpause|migrate|delete|snapshot|heal`; node `cordon|uncordon`.
`snapshot` takes a `<vm>-snap-<timestamp>` snapshot (a disk snapshot, or a VM-state checkpoint on
Kairon Machines without PVCs). It needs Write, is not destructive, and is exempt from the cooldown in
both directions, so a plan can "snapshot, then change" without waiting.
`heal` is a guarded restart: it needs Write like `restart`, but the step is blocked ("already healthy") unless
the VM's status is Failed, Error, Unknown, CrashLoopBackOff or unschedulable, checked when the plan is drafted
and again when it runs.
Bulk: the `propose_bulk_vm_action` tool applies one action (any VM action, including `hotplug` with `cpus`/`memory`)
to every VM matching a search query (`status:failed ns:prod`), across namespaces. The query must narrow the VMs, is
resolved once when the plan is drafted (approving later never widens it), and a query matching more than 25 VMs is an error,
never a silent truncation.

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
| `GET /api/v1/experience/copilot/plans?status=&limit=` | ReadOnly | Plans newest first: `pending` (default), `expired`, `executed`, `failed`, `cancelled`, `executing` or `all` |
| `POST /api/v1/experience/copilot/plans/:id/approve` | Write (+ the plan's role) | Four-eyes approval by someone other than the creator |
| `POST /api/v1/experience/copilot/plans/:id/undo` | Write (creator or admin) | Draft a new pending plan that reverses a plan that has run |
| `POST /api/v1/experience/copilot/plans/:id/execute` | Write (+ per step) | Body `{confirm_names, confirm_count, stop_on_error}` |
| `DELETE /api/v1/experience/copilot/plans/:id` | Write | Cancel |

### Search syntax

Free-text words must all match a name, namespace or label. Filters: `kind:vm|pod|node|pvc|snapshot|template|image`
(comma-separated, plural aliases accepted), `ns:<namespace>`, `status:<text>`, `os:<text>`. With no `kind:`
every kind is searched. Templates and images are cluster-wide catalogs, so an `ns:` filter excludes them.
Snapshots match their source VM's name. On Kairon, `os:` matches the Machine's image reference and the
catalog image's OS, family and version; images exist only on the Kairon backend.

## Undo

`POST .../plans/:id/undo` (or the **Undo** button on an executed or failed plan card) drafts a **new pending plan** that
puts things back: the inverse of every step that succeeded, in reverse order. Only pairs with a true inverse are
covered: `start`↔`stop`, `pause`↔`unpause`, `cordon`↔`uncordon`. `restart`, `heal`, `snapshot`, `migrate` and
`hotplug` leave nothing to put back (the previous value is not recorded) and `delete` cannot be taken back, so those
steps are listed in `skipped` and left out; a plan with nothing reversible answers 409. Steps that failed or never ran
are skipped too. The undo is an ordinary plan: only the original's creator or an admin can ask for it, it is audited and
announced like any other (prompt `undo <id>: …`), it is subject to the cooldown, blast-radius and four-eyes rules, and
nothing runs until someone approves and executes it.

## Notifications

Set `VEYRON_AI_PLAN_WEBHOOK_URL` to an HTTP(S) endpoint (a Slack, Mattermost, Teams or Discord incoming webhook, or
your own service) and Veyron posts a JSON message when a plan is drafted, approved, run, failed or cancelled. It is
how a second approver finds out a plan needs them: the `created` message says "needs a second approver" for plans
under the four-eyes policy.

```json
{"event": "plan.created", "text": "alice drafted a plan that needs a second approver: delete VM prod/db-1 [p-1a2b…]",
 "content": "…same line for Discord…", "actor": "alice", "source": "veyron-copilot", "timestamp": "…",
 "plan": {"id": "p-1a2b…", "status": "pending", "created_by": "alice", "min_role": "write", "steps": 1,
          "preview": ["delete VM prod/db-1"], "needs_approval": true, "approved_by": null,
          "confirm_count": null, "expires_at": "…", "prompt": "…"}}
```

`VEYRON_AI_PLAN_WEBHOOK_EVENTS` limits it to some of `created`, `approved`, `executed`, `failed`, `cancelled`
(comma list; default all). Delivery is best effort in the background: it never delays or fails a plan operation,
retries once on a network error or 5xx, does not retry a 4xx, and the URL (which may contain a token) is never logged.
Keep the URL in a Secret-backed environment variable; nothing secret is in the payload, but the prompt is.

**Signing.** Set `VEYRON_AI_PLAN_WEBHOOK_SECRET` and every delivery carries `X-Veyron-Timestamp` (Unix seconds) and
`X-Veyron-Signature: sha256=<hex>`, an HMAC-SHA256 of `"<timestamp>.<raw body>"` keyed with the secret. A receiver
recomputes it over the raw bytes it received, compares in constant time, and rejects a timestamp that is too old
(replay protection). Without a secret no signature headers are sent. Chat tools that accept only a URL ignore the headers.

```python
import hmac, hashlib
def valid(secret, ts, body, header):
    want = "sha256=" + hmac.new(secret.encode(), ts.encode() + b"." + body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(want, header)
```

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
| `VEYRON_AI_PLAN_WEBHOOK_URL` | (off) | HTTP(S) endpoint that receives a JSON message for plan events (see Notifications) |
| `VEYRON_AI_PLAN_WEBHOOK_EVENTS` | (all) | Comma list of `created`, `approved`, `executed`, `failed`, `cancelled` |
| `VEYRON_AI_PLAN_WEBHOOK_SECRET` | (off) | Signs each webhook delivery (`X-Veyron-Signature`, `X-Veyron-Timestamp`); see Notifications |
| `VEYRON_AI_FOUR_EYES` | (off) | `destructive`, `admin`, `all` (comma list): plans of that kind need a second approver |
| `VEYRON_AI_BULK_CONFIRM_THRESHOLD` | `5` | Plans with more targets than this need `confirm_count` (the count typed back) |
| `VEYRON_AI_PROVIDER` | (OpenAI-compatible) | `anthropic` talks to Anthropic's native Messages API (tool use). Also selected by an `sk-ant-` key or an `anthropic.com` URL |
| `VEYRON_AI_MAX_TOKENS` | `2048` | `max_tokens` per Anthropic request (the API requires one) |

With `VEYRON_AI_PROVIDER=anthropic`, set `VEYRON_AI_API_KEY` (or `ANTHROPIC_API_KEY`); the URL defaults to
`https://api.anthropic.com` and the model to `claude-sonnet-5-5` (`VEYRON_AI_MODEL` overrides, e.g.
`claude-haiku-5-5` for a cheaper loop). Veyron keeps the transcript in one provider-neutral shape and translates
at the HTTP boundary, so plans, role checks and audit behave the same on either provider. Only the Copilot agent
uses this setting; the `/ai` module has its own client.

## RBAC

Plans use ConfigMaps in the API namespace; the `veyron` ClusterRole already grants the needed
verbs (`deploy/k8s.yaml`, `charts/veyron/templates/clusterrole.yaml`). Nothing new to add.

## Not yet implemented

Audit actor in the generic HTTP middleware (still `api`); undo for steps without an inverse
(restart, heal, snapshot restore, migrate back); unattended auto-execution (deliberately out of scope).
