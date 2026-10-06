<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Veyron AI

![Veyron AI: MCP server, proposals, sandboxes, investigations, forecasts and in-cluster models](assets/readme-ai.jpg)

Veyron ships an AI layer that reads the cluster, explains what it sees and proposes changes. It
never changes anything on its own: every change becomes a **proposal** that a person with the right
role approves. The same tools are offered to the console assistant, to MCP clients (Claude, Cursor,
your own agents) and to the background investigator.

| Feature | Where | Needs an LLM |
|---|---|---|
| MCP server | `POST /mcp` | No (your agent brings the model) |
| Console assistant | Cmd+K → Ask, assistant panel | Yes |
| Proposals (approve / reject) | **AI → Proposals** | No |
| Agent sandboxes | **AI → Sandboxes**, `/api/v1/sandboxes` | No |
| VM from a sentence | New VM sheet → "Describe it" | Optional (rules work without) |
| Search | Cmd+K → Find | Optional |
| Incident investigations | **AI → Investigations** | Optional (evidence is gathered without) |
| Policy from a sentence | Policies → "Describe a policy" | Yes |
| Forecasts | Mission Control → Forecast | No |
| Tools from other MCP servers | Settings → AI tools from MCP servers | No |
| Run a model in the cluster | **AI → Models** | No (it *is* the model) |

## Choosing the model

Veyron works with any OpenAI-compatible chat endpoint. The active model is chosen in this order:

1. **Settings → Veyron AI model** (stored in the `veyron-ai-llm` ConfigMap, API key in the
   `veyron-ai-llm` Secret): either an in-cluster model from **AI → Models**, or a custom URL.
2. Environment variables on the API Deployment:

| Variable | Purpose |
|---|---|
| `VEYRON_AI_URL` | Base URL, e.g. `https://api.openai.com/v1` or `http://vllm.ai:8000/v1` |
| `VEYRON_AI_API_KEY` | Bearer key (optional for local servers) |
| `VEYRON_AI_MODEL` | Model name |
| `VEYRON_AI_MAX_TOOL_ROUNDS` | Tool-call rounds per question (default 8) |
| `VEYRON_AI_TIMEOUT_SECS` | Per-request timeout (default 30 s, or 300 s while an in-cluster model is selected) |
| `VEYRON_AI_RATE_LIMIT_PER_MIN` | Per-caller chat rate limit |
| `VEYRON_AI_NAMESPACE` | Where proposals and investigations are stored (default: API namespace) |
| `VEYRON_AI_PROPOSAL_TTL_HOURS` | Waiting proposals expire after this many hours (default 72) |
| `VEYRON_AI_PREDICTIVE=0` | Turn off the forecast tick |
| `VEYRON_AI_INVESTIGATOR=0` | Turn off automatic investigations |

The model needs tool calling. Every replica re-reads the setting once a minute, so a change in the
console takes effect cluster-wide without a restart.

## Safety model

- **Read tools** run with the caller's role. Output that came from outside Veyron (logs, flows,
  results from other MCP servers) is wrapped as untrusted data so the model treats it as content,
  not instructions.
- **Change tools** never act. They create a proposal with the exact API calls to make, a summary,
  the minimum role for each step, and whether it can be undone.
- **Approving** needs the step's role (`write` or `admin`). Over MCP only `admin` callers may
  approve, unless `VEYRON_MCP_ALLOW_SELF_APPROVE=1` lets `write` callers approve their own.
- A proposal nobody decides on within `VEYRON_AI_PROPOSAL_TTL_HOURS` (72 h) is marked **Expired**
  and can no longer be approved, so a stale plan never runs against a cluster that has moved on.
- Approved steps run through the normal API router, so they pass the same auth, audit trail and
  validation as a console click. Every chat, tool call and approval is in the audit trail.

## MCP server

`POST /mcp` speaks MCP over Streamable HTTP (JSON or SSE replies). Authenticate as for any API
call. Tools are filtered by the caller's role.

```json
{
  "mcpServers": {
    "veyron": {
      "url": "https://<node-ip>:30151/mcp",
      "headers": { "X-API-Key": "<key>" }
    }
  }
}
```

For clients that only speak stdio, `veyron mcp --url https://<node-ip>:30151 --api-key <key>
--insecure` bridges stdio to the HTTP endpoint (`--insecure` accepts the self-signed cert).

Resources: `veyron://cluster/capabilities`, `veyron://proposals`, `veyron://vms/<ns>/<name>`.

## Tools from other MCP servers

Settings → **AI tools from MCP servers** (admin only) registers external MCP servers so the
assistant can use their tools too (GitHub, Grafana, a ticketing system...).

- Server list: ConfigMap `veyron-ai-mcp-servers`; bearer tokens: Secret `veyron-ai-mcp-tokens`.
- Tools appear as `ext.<server>.<tool>`.
- A tool counts as read-only only if the server marks it `readOnlyHint` or you list it under
  `read_only_tools`. Other tools are hidden unless **Allow write tools** is on, and even then each
  call becomes an admin-approved proposal.
- Requests carry an `x-veyron-mcp-client` header; a Veyron MCP server receiving it only offers its
  own tools, so two Veyrons pointed at each other can't loop.

## Agent sandboxes

Disposable Ubuntu VMs (Python 3 included) where agents run code. Exec and file transfer go over
the guest agent, so a sandbox works with no network. A warm pool keeps one VM ready so a claim
takes seconds.

- Namespace `veyron-sandboxes`, with a deny-all egress NetworkPolicy. `internet: true` on create
  lifts it for that sandbox.
- Sandbox VMs are left out of `GET /api/v1/vms` (and so the VM list, Mission Control and search)
  unless you list the `veyron-sandboxes` namespace or pass `include_sandboxes=true`.
- Owners see only their own sandboxes; admins see all. TTL 5–1440 minutes (default 30); the leader
  tick deletes expired sandboxes.
- Env: `VEYRON_SANDBOX_NAMESPACE`, `VEYRON_SANDBOX_POOL_SIZE` (1), `VEYRON_SANDBOX_MAX_PER_OWNER`
  (3), `VEYRON_SANDBOX_MAX_TTL_MINUTES` (1440), `VEYRON_SANDBOX_TEMPLATE` (`ubuntu-24.04`),
  `VEYRON_SANDBOX_MEMORY` (`1Gi`), `VEYRON_SANDBOX_WAIT_SECS` (240).
- On a `--features kairon` build with `VEYRON_VM_BACKEND=kairon`, sandboxes are Firecracker
  machines instead, forked from `VEYRON_SANDBOX_KAIRON_PARENT` when set.

## Running a model in the cluster

**AI → Models → Deploy model** creates an Ubuntu VM that serves an OpenAI-compatible endpoint on
port 8000 behind a ClusterIP Service:

| Runtime | Console default | API default (no `model` given) | Needs |
|---|---|---|---|
| llama.cpp | Qwen2.5 1.5B Instruct (GGUF, q4_k_m) | Qwen2.5 0.5B Instruct (GGUF, q4_k_m) | 4 CPU, 8 GiB, 30 GiB disk, 16k context |
| vLLM | `Qwen/Qwen2.5-7B-Instruct` | `Qwen/Qwen2.5-1.5B-Instruct` | a GPU, 8 CPU, 32 GiB, 100 GiB disk |

Tool calling gets noticeably more reliable above 1B parameters; the 0.5B model is fine for a
smoke test, not for the assistant.

The model downloads on first boot (status **Installing** until `/v1/models` answers). When it's
**Ready**, **Use for Veyron AI** points the assistant at it. Endpoint:
`http://<name>-veyron-xp.<namespace>.svc:8000/v1`. Deleting the model VM clears the setting if it
was in use.

## Custom cloud-init on Linux templates

Linux templates already carry cloud-init that installs GuestKit. Cloud-init sent with
`POST /api/v1/vms` is now **merged** into it instead of replacing it: `packages`, `write_files`,
`runcmd`, `bootcmd`, `ssh_authorized_keys`, `users` and `mounts` are appended, other keys take your
value, and a `#!` script runs once as an extra `runcmd`. GuestKit keeps working.

## Cluster requirement: guest DNS with Cilium

KubeVirt guests reach cluster Services (DNS, the GuestKit download, model endpoints) through the
virt-launcher pod's NAT. With Cilium's kube-proxy replacement, socket load balancing must be
limited to the host namespace or these packets are never translated, and guests can't resolve
anything:

```yaml
# scripts/cluster/cilium-k3s-values.yaml
socketLB:
  hostNamespaceOnly: true
```

On an existing cluster without Helm:

```bash
kubectl -n kube-system patch cm cilium-config --type merge \
  -p '{"data":{"bpf-lb-sock-hostns-only":"true"}}'
kubectl -n kube-system rollout restart ds/cilium
```

`cilium-dbg status --verbose` should report `Socket LB Coverage: Hostns-only`. Without this,
sandboxes never become ready and GuestKit never connects.

## Testing

```bash
./scripts/verify-veyron-remote.sh HOST            # includes the AI + MCP smoke section
VEYRON_API_KEY=... ./scripts/test-ai-remote.sh HOST
# Optional: VEYRON_E2E_LLM=1 (chat), VEYRON_E2E_MODEL=1 (deploy a llama.cpp model, use it, delete it)
```
