#!/usr/bin/env bash
# ============================================================================
# test-ai-remote.sh — Veyron AI end-to-end checks against a deployed API
# ============================================================================
# Covers: MCP JSON-RPC (initialize, tools/list, tools/call), a sandbox lifecycle
# (create → exec → file write/read → destroy), intent → VM proposal → approve →
# VM exists → delete, NL policy draft, an on-demand investigation, and the MCP
# client (Veyron registered as an external server of itself).
#
# Usage:
#   VEYRON_API_KEY='...' ./scripts/test-ai-remote.sh <host> [https_node_port]
#
# Environment:
#   VEYRON_E2E_NAMESPACE   Namespace for the throwaway VM (default: default)
#   VEYRON_E2E_SKIP_VM=1   Reject the intent proposal instead of creating a VM
#   VEYRON_E2E_SKIP_SANDBOX=1
#   VEYRON_E2E_MODEL=1     Also deploy a llama.cpp model server, wait until it
#                          serves (up to 30 min), point Veyron AI at it, chat, clean up
#   VEYRON_E2E_LLM=1       Chat through the configured LLM (needs VEYRON_AI_* on the API)
#
# Requires curl and python3. Exits 1 if any check fails.
# ============================================================================

set -euo pipefail

HOST="${1:-${DEPLOY_HOST:-}}"
PORT="${2:-${VEYRON_NODE_PORT:-30151}}"
KEY="${VEYRON_API_KEY:-$(source "$(dirname "${BASH_SOURCE[0]}")/lib/api-key.sh" && veyron_require_api_key "${HOST}" 2>/dev/null)}"
NS="${VEYRON_E2E_NAMESPACE:-default}"
if [[ -z "${HOST}" || -z "${KEY}" ]]; then
    echo "Usage: VEYRON_API_KEY=... $0 <host> [https_node_port]" >&2
    exit 1
fi
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

BASE="https://${HOST}:${PORT}"
FAIL=0
SUFFIX="$(date +%s | tail -c 6)"

ok() { echo "  ✔ $1"; }
bad() {
    echo "  ✗ $1"
    [[ -n "${2:-}" ]] && echo "$2" | head -c 600 | sed 's/^/    /' && echo ""
    FAIL=$((FAIL + 1))
}

api() { # method path [json] [max_time]
    local args=(-skS --connect-timeout 15 --max-time "${4:-120}" -H "X-API-Key: ${KEY}"
        -H 'Accept: application/json, text/event-stream' -X "$1")
    [[ -n "${3:-}" ]] && args+=(-H 'Content-Type: application/json' -d "$3")
    curl "${args[@]}" "${BASE}$2" || true
}

# jget '<python expr over d>' — read stdin JSON, print the expression ('' on error).
jget() {
    python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
    v = eval(sys.argv[1], {"d": d})
    print(v if not isinstance(v, (dict, list)) else json.dumps(v))
except Exception:
    print("")
' "$1"
}

mcp() { # method params-json
    api POST /mcp "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$1\",\"params\":$2}"
}

echo ""
echo "Veyron AI end-to-end → ${BASE}"
echo ""

# ── Status ──────────────────────────────────────────────────────────
status=$(api GET /api/v1/ai/status)
mode=$(echo "${status}" | jget 'd["data"]["mode"]')
if [[ -n "${mode}" ]]; then
    ok "AI status (mode=${mode}, tools=$(echo "${status}" | jget 'd["data"]["tools"]'))"
else
    bad "AI status" "${status}"
fi

# ── MCP server ──────────────────────────────────────────────────────
echo "  (MCP)"
init=$(mcp initialize '{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"1"}}')
[[ "$(echo "${init}" | jget 'd["result"]["serverInfo"]["name"]')" == "veyron" ]] && ok "initialize" || bad "initialize" "${init}"
tl=$(mcp tools/list '{}')
n=$(echo "${tl}" | jget 'len(d["result"]["tools"])')
[[ "${n:-0}" -gt 20 ]] && ok "tools/list (${n} tools)" || bad "tools/list" "${tl}"
call=$(mcp tools/call '{"name":"list_vms","arguments":{"namespace":"all"}}')
[[ "$(echo "${call}" | jget 'd["result"]["isError"]')" == "False" ]] && ok "tools/call list_vms" || bad "tools/call list_vms" "${call}"
chg=$(mcp tools/call "{\"name\":\"stop_vm\",\"arguments\":{\"namespace\":\"${NS}\",\"vm_name\":\"does-not-exist-${SUFFIX}\"}}")
if echo "${chg}" | grep -q 'proposal_id\|not found\|NotFound\|404'; then
    ok "tools/call stop_vm drafts a proposal or reports the missing VM (never acts)"
else
    bad "tools/call stop_vm" "${chg}"
fi

# ── Sandbox ─────────────────────────────────────────────────────────
if [[ "${VEYRON_E2E_SKIP_SANDBOX:-0}" != "1" ]]; then
    echo "  (sandbox)"
    sb=$(api POST /api/v1/sandboxes '{"ttl_minutes":15,"wait":true}' 330)
    sid=$(echo "${sb}" | jget 'd["data"]["id"]')
    if [[ -n "${sid}" ]]; then
        ok "create sandbox ${sid} (backend=$(echo "${sb}" | jget 'd["data"].get("backend","")'), status=$(echo "${sb}" | jget 'd["data"]["status"]'))"
        sst=$(echo "${sb}" | jget 'd["data"]["status"]')
        for _ in $(seq 1 60); do
            [[ "${sst}" == "ready" || "${sst}" == "failed" ]] && break
            sleep 5
            sst=$(api GET "/api/v1/sandboxes/${sid}" | jget 'd["data"]["status"]')
        done
        [[ "${sst}" == "ready" ]] && ok "sandbox ready" || bad "sandbox ready (status: ${sst:-unknown})"
        ex=$(api POST "/api/v1/sandboxes/${sid}/exec" '{"command":"uname -s && echo $((6*7))","timeout_secs":60}' 120)
        out=$(echo "${ex}" | jget 'd["data"]["stdout"]')
        [[ "${out}" == *Linux* && "${out}" == *42* ]] && ok "exec (exit $(echo "${ex}" | jget 'd["data"]["exit_code"]'))" || bad "exec" "${ex}"
        wr=$(api PUT "/api/v1/sandboxes/${sid}/files" '{"path":"/tmp/e2e.txt","content":"hello veyron"}')
        rd=$(api GET "/api/v1/sandboxes/${sid}/files?path=/tmp/e2e.txt")
        if echo "${rd}" | grep -q 'hello veyron\|aGVsbG8gdmV5cm9u'; then ok "write + read file"; else bad "write + read file" "${wr} ${rd}"; fi
        del=$(api DELETE "/api/v1/sandboxes/${sid}")
        echo "${del}" | grep -q '"success":true' && ok "destroy sandbox" || bad "destroy sandbox" "${del}"
    else
        bad "create sandbox" "${sb}"
    fi
fi

# ── Intent → proposal → VM ─────────────────────────────────────────
echo "  (intent → proposal)"
VM="ai-e2e-${SUFFIX}"
it=$(api POST /api/v1/ai/intent/vm "{\"text\":\"tiny ubuntu vm called ${VM} in namespace ${NS}\",\"submit\":true}" 120)
pid=$(echo "${it}" | jget 'd["data"]["proposal"]["id"]')
if [[ -n "${pid}" ]]; then
    ok "intent planned ${VM} (\$$(echo "${it}" | jget 'd["data"]["total_monthly"]')/mo) → proposal ${pid}"
    if [[ "${VEYRON_E2E_SKIP_VM:-0}" == "1" ]]; then
        rj=$(api POST "/api/v1/ai/proposals/${pid}/reject" '{}')
        [[ "$(echo "${rj}" | jget 'd["data"]["status"]')" == "rejected" ]] && ok "reject proposal" || bad "reject proposal" "${rj}"
    else
        ap=$(api POST "/api/v1/ai/proposals/${pid}/approve" '{"pre_snapshot":false}')
        [[ -n "$(echo "${ap}" | jget 'd["data"]["status"]')" ]] && ok "approve proposal" || bad "approve proposal" "${ap}"
        st=""
        for _ in $(seq 1 60); do
            st=$(api GET "/api/v1/ai/proposals/${pid}" | jget 'd["data"]["status"]')
            [[ "${st}" == "succeeded" || "${st}" == "failed" || "${st}" == "rolled_back" ]] && break
            sleep 5
        done
        [[ "${st}" == "succeeded" ]] && ok "proposal executed (${st})" || bad "proposal executed (${st:-timeout})" "$(api GET "/api/v1/ai/proposals/${pid}")"
        vm=$(api GET "/api/v1/vms/${NS}/${VM}")
        echo "${vm}" | grep -q '"success":true' && ok "VM ${NS}/${VM} exists" || bad "VM ${NS}/${VM} exists" "${vm}"

        echo "  (investigation)"
        inv=$(api POST /api/v1/ai/investigations/run "{\"namespace\":\"${NS}\",\"vm_name\":\"${VM}\"}" 180)
        [[ -n "$(echo "${inv}" | jget 'd["data"]["id"]')" ]] && ok "investigate ${VM} (mode=$(echo "${inv}" | jget 'd["data"]["mode"]'))" || bad "investigate ${VM}" "${inv}"

        dl=$(api DELETE "/api/v1/vms/${NS}/${VM}")
        echo "${dl}" | grep -q '"success":true' && ok "delete ${VM}" || bad "delete ${VM}" "${dl}"
    fi
else
    bad "intent → proposal" "${it}"
fi

# ── NL policy ───────────────────────────────────────────────────────
echo "  (policy)"
pd=$(api POST /api/v1/ai/policies/draft '{"description":"warn when a vm has more than 8 vcpus or exposes RDP"}' 120)
rules=$(echo "${pd}" | jget 'len(d["data"]["policy"]["rules"])')
[[ "${rules:-0}" -ge 1 ]] && ok "draft policy (${rules} rules, $(echo "${pd}" | jget 'd["data"]["preview"]["checked"]') VMs checked)" || bad "draft policy" "${pd}"

# ── Search + forecast ───────────────────────────────────────────────
se=$(api POST /api/v1/ai/search '{"query":"stopped vms in default"}')
[[ "$(echo "${se}" | jget 'd["data"]["filter"]["resource"]')" == "vms" ]] && ok "NL search" || bad "NL search" "${se}"
fc=$(api GET /api/v1/ai/forecast)
echo "${fc}" | grep -q '"success":true' && ok "forecast" || bad "forecast" "${fc}"

# ── MCP client (Veyron as its own external server) ─────────────────
echo "  (MCP client)"
prev=$(api GET /api/v1/ai/mcp-servers)
keep=$(echo "${prev}" | python3 -c '
import json, sys
fields = ["name","url","enabled","allow_write","read_only_tools","tls_verify","description"]
try:
    rows = json.load(sys.stdin)["data"] or []
except Exception:
    rows = []
print(json.dumps([{k: r[k] for k in fields if k in r} for r in rows if r.get("name") != "e2e-self"]))
')
with_self=$(KEEP="${keep}" KEY="${KEY}" python3 -c '
import json, os
rows = json.loads(os.environ["KEEP"])
rows.append({"name": "e2e-self", "url": "https://127.0.0.1:5151/mcp", "tls_verify": False, "token": os.environ["KEY"]})
print(json.dumps({"servers": rows}))
')
put=$(api PUT /api/v1/ai/mcp-servers "${with_self}")
conn=$(echo "${put}" | jget '[s for s in d["data"] if s["name"]=="e2e-self"][0]["connected"]')
[[ "${conn}" == "True" ]] && ok "register e2e-self (connected)" || bad "register e2e-self" "${put}"
ext=$(mcp tools/call '{"name":"ext.e2e-self.list_templates","arguments":{}}')
[[ "$(echo "${ext}" | jget 'd["result"]["isError"]')" == "False" ]] && ok "call ext.e2e-self.list_templates" || bad "call ext.e2e-self.list_templates" "${ext}"
restore=$(api PUT /api/v1/ai/mcp-servers "{\"servers\":${keep}}")
echo "${restore}" | grep -q '"success":true' && ok "remove e2e-self" || bad "remove e2e-self" "${restore}"

# ── LLM chat (optional) ─────────────────────────────────────────────
chat() {
    api POST /api/v1/ai/chat '{"messages":[{"role":"user","content":"How many VMs are running? Use a tool."}]}' 240
}
if [[ "${VEYRON_E2E_LLM:-0}" == "1" ]]; then
    echo "  (LLM)"
    ch=$(chat)
    [[ -n "$(echo "${ch}" | jget 'd["data"]["outcome"]["content"]')" ]] && ok "chat (tools used: $(echo "${ch}" | jget '",".join(d["data"]["outcome"]["tools_used"])'))" || bad "chat" "${ch}"
fi

# ── Model serving (optional, slow) ─────────────────────────────────
if [[ "${VEYRON_E2E_MODEL:-0}" == "1" ]]; then
    echo "  (model serving)"
    M="ai-e2e-llm-${SUFFIX}"
    MODEL_ID="${VEYRON_E2E_MODEL_ID:-Qwen/Qwen2.5-1.5B-Instruct-GGUF/qwen2.5-1.5b-instruct-q4_k_m.gguf}"
    MODEL_MEM="${VEYRON_E2E_MODEL_MEMORY:-4Gi}"
    cr=$(api POST /api/v1/ai/models "{\"name\":\"${M}\",\"namespace\":\"${NS}\",\"runtime\":\"llamacpp\",\"model\":\"${MODEL_ID}\",\"memory\":\"${MODEL_MEM}\"}" 120)
    if echo "${cr}" | grep -q '"success":true'; then
        ok "deploy ${M} ($(echo "${cr}" | jget 'd["data"]["served_name"]'))"
        st=""
        for _ in $(seq 1 180); do
            st=$(api GET /api/v1/ai/models | jget "[m for m in d['data'] if m['name']=='${M}'][0]['status']")
            [[ "${st}" == "ready" ]] && break
            sleep 10
        done
        if [[ "${st}" == "ready" ]]; then
            ok "${M} is serving"
            us=$(api PUT /api/v1/ai/settings/llm "{\"namespace\":\"${NS}\",\"name\":\"${M}\"}")
            [[ "$(echo "${us}" | jget 'd["data"]["source"]')" == "in-cluster" ]] && ok "Use for Veyron AI" || bad "Use for Veyron AI" "${us}"
            ch=$(chat)
            [[ -n "$(echo "${ch}" | jget 'd["data"]["outcome"]["content"]')" ]] && ok "chat through the in-cluster model (tools used: $(echo "${ch}" | jget '",".join(d["data"]["outcome"]["tools_used"])'))" || bad "chat through the in-cluster model" "${ch}"
            inv=$(api POST /api/v1/ai/investigations/run "{\"namespace\":\"${NS}\",\"vm_name\":\"${M}\"}" 300)
            [[ "$(echo "${inv}" | jget 'd["data"]["mode"]')" == "llm" ]] && ok "investigation written by the model" || bad "investigation written by the model (mode=$(echo "${inv}" | jget 'd["data"]["mode"]'))" "${inv}"
            api PUT /api/v1/ai/settings/llm '{"clear":true}' >/dev/null
        else
            bad "${M} is serving (last status: ${st:-unknown})" "$(api GET /api/v1/ai/models)"
        fi
        dm=$(api DELETE "/api/v1/ai/models/${NS}/${M}")
        echo "${dm}" | grep -q '"success":true' && ok "delete ${M}" || bad "delete ${M}" "${dm}"
    else
        bad "deploy ${M}" "${cr}"
    fi
fi

echo ""
if [[ "${FAIL}" -eq 0 ]]; then
    echo "All AI checks passed."
    exit 0
fi
echo "${FAIL} AI check(s) failed." >&2
exit 1
