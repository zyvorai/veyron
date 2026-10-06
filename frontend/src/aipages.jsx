import { useCallback, useEffect, useMemo, useState } from 'react';
import { McpServersBox } from './McpServersBox.jsx';
import { Copy, Plus, Trash2, Play, Sparkles, Shield, TerminalSquare, Cpu, Search } from 'lucide-react';
import { api, relTime } from './api.js';
import { Shell, Specs, Band, Note, DataTable, useSources, run, titleCase } from './insights.jsx';
import { ProposalCard } from './ProposalCard.jsx';
import { Md } from './Assistant.jsx';

const isAdmin = (u) => u?.role === 'admin';
const canWrite = (u) => u?.role === 'admin' || u?.role === 'write';

function copyText(text, showToast) {
  navigator.clipboard?.writeText(text).then(
    () => showToast('Copied'),
    () => showToast('Copy failed', true),
  );
}

function Snippet({ text, showToast }) {
  return (
    <div className="ai-snippet">
      <pre>{text}</pre>
      <button type="button" className="tb" onClick={() => copyText(text, showToast)} aria-label="Copy">
        <Copy size={13} />
      </button>
    </div>
  );
}

/* ───────────────────────── Veyron AI overview ───────────────────────── */

export function AiHomePage({ user, showToast, onAsk }) {
  const s = useSources({
    status: api.aiStatus,
    llm: api.getLlmSetting,
    tools: api.aiTools,
  });
  const st = s.get('status') || {};
  const llm = s.get('llm') || {};
  const origin = window.location.origin;
  const mcpJson = JSON.stringify(
    { mcpServers: { veyron: { url: `${origin}/mcp`, headers: { 'X-API-Key': '<your Veyron API key>' } } } },
    null,
    2,
  );
  const stdioJson = JSON.stringify(
    {
      mcpServers: {
        veyron: {
          command: 'veyron',
          args: ['mcp', '--url', origin, '--insecure'],
          env: { VEYRON_API_KEY: '<your Veyron API key>' },
        },
      },
    },
    null,
    2,
  );

  const [llmForm, setLlmForm] = useState({ url: '', model: '', api_key: '' });
  const saveLlm = async (e) => {
    e.preventDefault();
    if (await run(showToast, () => api.putLlmSetting(llmForm), 'Language model saved')) {
      setLlmForm({ url: '', model: '', api_key: '' });
      s.reload();
    }
  };

  const toolRows = (s.get('tools') || []).map((t) => ({
    id: t.name,
    name: t.name,
    kind: titleCase(t.kind),
    role: t.min_role,
    desc: t.description,
  }));

  return (
    <Shell
      kicker="Veyron AI"
      title="An operator that asks first."
      lede="Veyron AI reads live cluster state with your permissions, investigates incidents on its own, and drafts every change as a proposal a human approves. Connect any MCP client to the same tools."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Mode', st.mode === 'agent' ? 'Agent' : st.mode === 'advisor' ? 'Advisors only' : '…'],
          ['Model', st.model || (st.mode ? 'None' : '…')],
          ['Tools', st.tools ?? '…'],
          ['Change tools', st.change_tools ?? '…'],
          ['You', st.you ? `${st.you.subject} · ${st.you.role}` : '…'],
        ]}
      />

      <Band
        kicker="Assistant"
        title="Ask anything about your fleet"
        lede="Press ⌘J anywhere. The assistant sees the page and VM you are looking at."
        actions={
          <button className="btn sm primary" onClick={() => onAsk?.()}>
            <Sparkles size={13} /> Open assistant
          </button>
        }
      >
        {st.mode === 'advisor' && (
          <Note warn>
            No language model is configured, so answers come from Veyron&apos;s built-in advisors. Point Veyron at an
            OpenAI-compatible endpoint below, or serve a model in your cluster from AI → Models.
          </Note>
        )}
      </Band>

      <Band
        kicker="Language model"
        title={
          llm.source === 'in-cluster'
            ? 'In-cluster model'
            : llm.source === 'custom'
              ? 'Custom endpoint'
              : llm.source === 'env'
                ? 'From the deployment'
                : 'Not configured'
        }
        lede={
          llm.base_url
            ? `${llm.model || 'default model'} at ${llm.base_url}`
            : llm.source === 'env'
              ? `${llm.model || 'default model'} (set by VEYRON_AI_* on the deployment)`
            : 'Any OpenAI-compatible endpoint works: OpenAI, OpenRouter, Azure, vLLM, llama.cpp, Ollama.'
        }
        actions={
          isAdmin(user) && (llm.source === 'in-cluster' || llm.source === 'custom') ? (
            <button
              className="btn sm secondary"
              onClick={async () => {
                if (await run(showToast, () => api.putLlmSetting({ clear: true }), 'Using the deployment setting')) s.reload();
              }}
            >
              Reset
            </button>
          ) : null
        }
      >
        {isAdmin(user) ? (
          <form className="insight-form" onSubmit={saveLlm}>
            <label>
              Base URL
              <input
                value={llmForm.url}
                onChange={(e) => setLlmForm({ ...llmForm, url: e.target.value })}
                placeholder="https://api.openai.com/v1"
                required
              />
            </label>
            <label>
              Model
              <input value={llmForm.model} onChange={(e) => setLlmForm({ ...llmForm, model: e.target.value })} placeholder="gpt-4.1-mini" />
            </label>
            <label>
              API key
              <input
                type="password"
                value={llmForm.api_key}
                onChange={(e) => setLlmForm({ ...llmForm, api_key: e.target.value })}
                placeholder="Stored in a Secret"
                autoComplete="off"
              />
            </label>
            <button type="submit" className="btn primary sm">
              Save
            </button>
          </form>
        ) : (
          <Note>An admin can change the model here.</Note>
        )}
      </Band>

      <Band
        kicker="MCP"
        title="Use Veyron from Claude, Cursor or your own agent"
        lede="Veyron is an MCP server. Clients get the tools your API key's role allows. Change tools return proposals, never direct changes."
        wide
      >
        <div className="ai-two">
          <div>
            <p className="insight-lede">Streamable HTTP (recommended)</p>
            <Snippet text={mcpJson} showToast={showToast} />
          </div>
          <div>
            <p className="insight-lede">stdio, through the veyron CLI</p>
            <Snippet text={stdioJson} showToast={showToast} />
          </div>
        </div>
      </Band>

      {isAdmin(user) && (
        <Band
          kicker="Connected servers"
          title="Tools from other MCP servers"
          lede="Veyron AI can use tools from other MCP servers (ticketing, docs, CMDB). Only tools marked read-only are offered, unless you allow writes; writes always become proposals an Admin approves."
          wide
        >
          <McpServersBox />
        </Band>
      )}

      <PolicyBand user={user} showToast={showToast} />

      <Band kicker="Tools" title="What the AI can use" wide>
        <DataTable
          cols={[
            ['name', 'Tool', 'm'],
            ['kind', 'Kind'],
            ['role', 'Role'],
            ['desc', 'Description'],
          ]}
          rows={toolRows}
          loading={!s.loaded('tools')}
          error={s.err('tools')}
          filter="Filter tools"
          empty="No tools."
          limit={20}
        />
      </Band>
    </Shell>
  );
}

function PolicyBand({ user, showToast }) {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState(null);
  const [proposal, setProposal] = useState(null);
  const submit = async (e) => {
    e.preventDefault();
    if (!text.trim()) return;
    setBusy(true);
    setProposal(null);
    try {
      setDraft(await api.aiPolicyDraft(text.trim()));
    } catch (err) {
      showToast(err.message || String(err), true);
    } finally {
      setBusy(false);
    }
  };
  const propose = async () => {
    try {
      const r = await api.aiPolicyPreview(draft.policy, true);
      setProposal(r.proposal || null);
      showToast('Proposal created');
    } catch (err) {
      showToast(err.message || String(err), true);
    }
  };
  const violations = draft?.preview?.violations || [];
  return (
    <Band
      kicker="Guardrails"
      title="Write a policy in a sentence"
      lede='For example: "Production VMs must have a snapshot schedule and no public RDP." Veyron turns it into a VeyronPolicy and shows which VMs would break it before anything is saved.'
      wide
    >
      <form className="ai-policy-form" onSubmit={submit}>
        <textarea value={text} onChange={(e) => setText(e.target.value)} rows={2} placeholder="Describe the rule…" />
        <button type="submit" className="btn primary sm" disabled={busy || !text.trim()}>
          <Shield size={13} /> {busy ? 'Drafting…' : 'Draft policy'}
        </button>
      </form>
      {draft && (
        <div className="ai-policy-out">
          {draft.explanation && <Note>{draft.explanation}</Note>}
          <pre className="ai-yaml">{JSON.stringify(draft.policy, null, 2)}</pre>
          <p className="insight-lede">
            Dry run: {violations.length} of {draft.preview?.checked ?? 0} VMs would violate this policy.
          </p>
          {violations.length > 0 && (
            <DataTable
              cols={[
                ['vm', 'VM'],
                ['rule', 'Rule', 'm'],
                ['message', 'Why'],
              ]}
              rows={violations.map((v, i) => ({ id: `${v.vm}-${v.rule}-${i}`, ...v }))}
              limit={10}
            />
          )}
          {(draft.preview?.errors || []).map((e) => (
            <Note warn key={e}>
              {e}
            </Note>
          ))}
          {canWrite(user) && !proposal && (
            <button className="btn sm primary" onClick={propose}>
              Propose this policy
            </button>
          )}
          {proposal && <ProposalCard proposal={proposal} user={user} />}
        </div>
      )}
    </Band>
  );
}

/* ───────────────────────── Proposals ───────────────────────── */

const FILTERS = [
  ['pending', 'Waiting'],
  ['', 'All'],
  ['succeeded', 'Done'],
  ['rolled_back', 'Rolled back'],
  ['failed', 'Failed'],
  ['rejected', 'Rejected'],
  ['expired', 'Expired'],
];

export function ProposalsPage({ user }) {
  const [filter, setFilter] = useState('pending');
  const [items, setItems] = useState(null);
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const load = useCallback(async () => {
    setBusy(true);
    try {
      setItems(await api.listProposals(filter));
      setErr('');
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy(false);
    }
  }, [filter]);
  useEffect(() => {
    load();
    const t = setInterval(load, 15_000);
    return () => clearInterval(t);
  }, [load]);

  return (
    <Shell
      kicker="Veyron AI"
      title="Proposals"
      lede="Every change the AI wants to make waits here. Approving runs it with your permissions, checks that it worked, and rolls it back if it did not."
      busy={busy}
      onRefresh={load}
    >
      <section className="insight-band">
        <div className="apple-chapter insight-chapter">
          <div className="seg ai-filter" role="group" aria-label="Filter proposals">
            {FILTERS.map(([v, l]) => (
              <button key={l} className="tb" aria-pressed={filter === v} onClick={() => setFilter(v)}>
                {l}
              </button>
            ))}
          </div>
          {err && <div className="ops-err">{err}</div>}
          {items == null ? (
            <Note>Loading…</Note>
          ) : items.length === 0 ? (
            <Note>
              {filter === 'pending'
                ? 'Nothing is waiting for approval. Ask the assistant to fix something and its proposal will appear here.'
                : 'No proposals.'}
            </Note>
          ) : (
            <div className="ai-proposal-list">
              {items.map((p) => (
                <ProposalCard key={p.id} proposal={p} user={user} onChange={load} />
              ))}
            </div>
          )}
        </div>
      </section>
    </Shell>
  );
}

/* ───────────────────────── Investigations ───────────────────────── */

function InvestigationCard({ inv, user }) {
  const [open, setOpen] = useState(false);
  const [proposals, setProposals] = useState([]);
  useEffect(() => {
    if (!open || !(inv.proposal_ids || []).length) return;
    Promise.all(inv.proposal_ids.map((id) => api.getProposal(id).catch(() => null))).then((ps) =>
      setProposals(ps.filter(Boolean)),
    );
  }, [open, inv.proposal_ids]);
  return (
    <div className="ai-inv" data-sev={inv.severity}>
      <button type="button" className="ai-inv-head" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className="ai-sev">{inv.severity}</span>
        <b>{inv.title}</b>
        <small>
          {inv.trigger} · {relTime(inv.created_at)}
          {inv.mode === 'llm' ? ' · AI analysis' : ' · evidence only'}
        </small>
      </button>
      {inv.root_cause && (
        <p className="ai-inv-cause">
          <b>Likely cause{inv.confidence != null ? ` (${Math.round(inv.confidence * 100)}% sure)` : ''}:</b> {inv.root_cause}
        </p>
      )}
      {open && (
        <div className="ai-inv-body">
          {inv.summary && <Md text={inv.summary} />}
          {(inv.timeline || []).length > 0 && (
            <>
              <p className="md-h">Timeline</p>
              <ul className="ai-timeline">
                {inv.timeline.map((t, i) => (
                  <li key={i}>
                    <code>{t.time ? relTime(t.time) : ''}</code> {t.event}
                  </li>
                ))}
              </ul>
            </>
          )}
          {(inv.recommendations || []).length > 0 && (
            <>
              <p className="md-h">What to do</p>
              <ul>
                {inv.recommendations.map((r) => (
                  <li key={r}>{r}</li>
                ))}
              </ul>
            </>
          )}
          {proposals.map((p) => (
            <ProposalCard key={p.id} proposal={p} user={user} compact />
          ))}
          {inv.evidence && (
            <details className="ai-log">
              <summary>Evidence</summary>
              <pre>{JSON.stringify(inv.evidence, null, 2)}</pre>
            </details>
          )}
        </div>
      )}
    </div>
  );
}

export function InvestigationsPage({ user, showToast }) {
  const s = useSources({ items: api.listInvestigations, vms: () => api.listVms('all') });
  const [target, setTarget] = useState('');
  const [busy, setBusy] = useState(false);
  const vms = (s.get('vms') || []).map((v) => `${v.namespace}/${v.name}`);
  const items = s.get('items') || [];
  const runOne = async (e) => {
    e.preventDefault();
    const [ns, name] = target.includes('/') ? target.split('/') : ['default', target];
    if (!name) return;
    setBusy(true);
    const ok = await run(showToast, () => api.runInvestigation({ namespace: ns, vm_name: name }), 'Investigation finished');
    setBusy(false);
    if (ok) s.reload();
  };
  return (
    <Shell
      kicker="Veyron AI"
      title="Investigations"
      lede="When a VM fails or events pile up, Veyron AI gathers events, logs, guest health and recent changes, then writes up the likely cause and a fix you can approve."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Investigations', s.loaded('items') ? items.length : '…'],
          ['Last 24 h', items.filter((i) => Date.now() - Date.parse(i.created_at) < 86_400_000).length],
          ['Critical', items.filter((i) => i.severity === 'critical').length, true],
          ['With AI analysis', items.filter((i) => i.mode === 'llm').length],
        ]}
      />
      <Band kicker="On demand" title="Investigate a VM now" wide>
        {canWrite(user) ? (
          <form className="insight-form" onSubmit={runOne}>
            <label>
              VM
              <input list="ai-inv-vms" value={target} onChange={(e) => setTarget(e.target.value)} placeholder="default/web" required />
              <datalist id="ai-inv-vms">
                {vms.map((v) => (
                  <option key={v} value={v} />
                ))}
              </datalist>
            </label>
            <button type="submit" className="btn primary sm" disabled={busy}>
              <Search size={13} /> {busy ? 'Investigating…' : 'Investigate'}
            </button>
          </form>
        ) : (
          <Note>Running an investigation needs the write role.</Note>
        )}
      </Band>
      <Band kicker="Reports" title="Recent investigations" wide>
        {s.err('items') ? (
          <div className="ops-err">{s.err('items')}</div>
        ) : !s.loaded('items') ? (
          <Note>Loading…</Note>
        ) : items.length === 0 ? (
          <Note>No incidents investigated yet. Veyron checks for failed VMs and event storms every minute.</Note>
        ) : (
          <div className="ai-inv-list">
            {items.map((inv) => (
              <InvestigationCard key={inv.id} inv={inv} user={user} />
            ))}
          </div>
        )}
      </Band>
    </Shell>
  );
}

/* ───────────────────────── Sandboxes ───────────────────────── */

const BACKEND_LABEL = { kubevirt: 'KubeVirt', kairon: 'Kairon' };

export function SandboxesPage({ user, showToast }) {
  const s = useSources({ data: api.listSandboxes });
  const data = s.get('data') || {};
  const items = data.items || [];
  const [ttl, setTtl] = useState('30');
  const [internet, setInternet] = useState(false);
  const [creating, setCreating] = useState(false);
  const [sel, setSel] = useState(null);
  const [cmd, setCmd] = useState('uname -a && python3 --version');
  const [out, setOut] = useState(null);
  const [running, setRunning] = useState(false);
  const selected = items.find((x) => x.id === sel);

  const create = async () => {
    setCreating(true);
    try {
      const sb = await api.createSandbox({ ttl_minutes: Number(ttl) || 30, internet });
      showToast(`Sandbox ${sb.id} ready`);
      setSel(sb.id);
      s.reload();
    } catch (e) {
      showToast(e.message || String(e), true);
    } finally {
      setCreating(false);
    }
  };
  const exec = async (e) => {
    e.preventDefault();
    if (!sel || !cmd.trim()) return;
    setRunning(true);
    try {
      setOut(await api.sandboxExec(sel, cmd, 60));
    } catch (err) {
      setOut({ exit_code: -1, stdout: '', stderr: err.message || String(err) });
    } finally {
      setRunning(false);
    }
  };
  const rows = items.map((x) => ({
    id: x.id,
    name: x.id,
    status: x.status,
    owner: x.owner,
    backend: BACKEND_LABEL[x.backend] || x.backend,
    vm: x.vm ? `${x.namespace}/${x.vm}` : '—',
    expires: x.expires_at ? relTime(x.expires_at).replace(' ago', '') : '—',
    act: (
      <span className="vt-acts">
        <button type="button" className="btn sm secondary" onClick={() => setSel(x.id)}>
          <TerminalSquare size={12} /> Open
        </button>
        <button
          type="button"
          className="btn sm secondary"
          onClick={async () => {
            if (await run(showToast, () => api.deleteSandbox(x.id), `Sandbox ${x.id} destroyed`)) {
              if (sel === x.id) setSel(null);
              s.reload();
            }
          }}
        >
          <Trash2 size={12} />
        </button>
      </span>
    ),
  }));
  const pool = data.pool || {};
  return (
    <Shell
      kicker="Veyron AI"
      title="Agent sandboxes"
      lede="Disposable, isolated VMs where AI agents run code. No internet by default, and each one deletes itself when its time is up."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Backend', data.backend ? BACKEND_LABEL[data.backend] || titleCase(data.backend) : '…'],
          ['Live', s.loaded('data') ? items.length : '…'],
          ['Warm pool', pool.target != null ? `${pool.ready ?? 0} of ${pool.target} ready` : '—'],
          ['Typical start', data.typical_start || '—'],
        ]}
      />
      <Band kicker="Create" title="New sandbox" wide>
        {canWrite(user) ? (
          <div className="insight-form">
            <label>
              Lifetime (minutes)
              <input type="number" min="5" max="1440" value={ttl} onChange={(e) => setTtl(e.target.value)} />
            </label>
            <label className="check">
              <input type="checkbox" checked={internet} onChange={(e) => setInternet(e.target.checked)} />
              Allow internet
            </label>
            <button type="button" className="btn primary sm" onClick={create} disabled={creating}>
              <Plus size={13} /> {creating ? 'Starting…' : 'Create sandbox'}
            </button>
          </div>
        ) : (
          <Note>Creating sandboxes needs the write role.</Note>
        )}
        {data.note && <Note>{data.note}</Note>}
      </Band>
      <Band kicker="Live" title="Sandboxes" wide>
        <DataTable
          cols={[
            ['name', 'Sandbox', 'm'],
            ['status', 'Status', 'st'],
            ['owner', 'Owner'],
            ['backend', 'Backend'],
            ['vm', 'VM', 'm'],
            ['expires', 'Expires in'],
            ['act', '', 'act'],
          ]}
          rows={rows}
          loading={!s.loaded('data')}
          error={s.err('data')}
          empty="No sandboxes running."
        />
      </Band>
      {selected && (
        <Band kicker="Terminal" title={selected.id} wide>
          <form className="ai-exec" onSubmit={exec}>
            <span>$</span>
            <input value={cmd} onChange={(e) => setCmd(e.target.value)} spellCheck={false} />
            <button type="submit" className="btn primary sm" disabled={running}>
              <Play size={13} /> {running ? 'Running…' : 'Run'}
            </button>
          </form>
          {out && (
            <div className="ai-exec-out">
              <small>
                exit {out.exit_code}
                {out.duration_ms != null ? ` · ${out.duration_ms} ms` : ''}
              </small>
              {out.stdout && <pre>{out.stdout}</pre>}
              {out.stderr && <pre className="err">{out.stderr}</pre>}
            </div>
          )}
        </Band>
      )}
    </Shell>
  );
}

/* ───────────────────────── Models ───────────────────────── */

const RUNTIMES = [
  ['llamacpp', 'llama.cpp (CPU)', 'https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf'],
  ['vllm', 'vLLM (GPU)', 'Qwen/Qwen2.5-7B-Instruct'],
];

const MODEL_STATUS = { ready: 'Ready', installing: 'Installing', provisioning: 'Provisioning', stopped: 'Stopped' };

export function ModelsPage({ user, showToast }) {
  const s = useSources({ models: api.listModels, llm: api.getLlmSetting });
  const llm = s.get('llm') || {};
  const models = s.get('models') || [];
  const [form, setForm] = useState({ name: '', namespace: 'default', runtime: 'llamacpp', model: RUNTIMES[0][2], gpu_count: 1, cpus: 4, memory: '8Gi' });
  const runtime = useMemo(() => RUNTIMES.find((r) => r[0] === form.runtime), [form.runtime]);

  const create = async (e) => {
    e.preventDefault();
    if (!/^[a-z0-9]([-a-z0-9]*[a-z0-9])?$/.test(form.name)) {
      showToast('Name: lowercase letters, digits and dashes', true);
      return;
    }
    const { gpu_count, ...rest } = form;
    const body = { ...rest, cpus: Number(form.cpus) || 4 };
    if (form.runtime === 'vllm') body.gpu = { count: Number(gpu_count) || 1 };
    if (await run(showToast, () => api.createModel(body), `Deploying ${form.name}. First start downloads the model.`)) {
      setForm((f) => ({ ...f, name: '' }));
      s.reload();
    }
  };

  const rows = models.map((m) => {
    const active = m.in_use || (llm.ref && llm.ref.name === m.name && llm.ref.namespace === m.namespace);
    return {
      id: `${m.namespace}/${m.name}`,
      name: m.name,
      status: MODEL_STATUS[m.status] || 'Pending',
      runtime: m.runtime === 'vllm' ? 'vLLM' : 'llama.cpp',
      model: m.model,
      served: m.models?.[0] || m.served_name || '—',
      endpoint: m.endpoint || '—',
      act: (
        <span className="vt-acts">
          {isAdmin(user) && (
            <button
              type="button"
              className="btn sm secondary"
              disabled={active || m.status !== 'ready'}
              title={m.status !== 'ready' ? 'Available once the model is serving' : undefined}
              onClick={async () => {
                if (await run(showToast, () => api.putLlmSetting({ namespace: m.namespace, name: m.name }), `Veyron AI now uses ${m.name}`))
                  s.reload();
              }}
            >
              <Sparkles size={12} /> {active ? 'In use' : 'Use for Veyron AI'}
            </button>
          )}
          {canWrite(user) && (
            <button
              type="button"
              className="btn sm secondary"
              onClick={async () => {
                if (!window.confirm(`Delete model ${m.name} and its VM?`)) return;
                if (await run(showToast, () => api.deleteModel(m.namespace, m.name), `Deleted ${m.name}`)) s.reload();
              }}
            >
              <Trash2 size={12} />
            </button>
          )}
        </span>
      ),
    };
  });

  return (
    <Shell
      kicker="Veyron AI"
      title="Models"
      lede="Serve an open model inside your cluster with one click, then point Veyron AI at it so nothing leaves your network."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Models', s.loaded('models') ? models.length : '…'],
          ['Ready', models.filter((m) => m.status === 'ready').length],
          ['Veyron AI uses', llm.ref ? llm.ref.name : llm.source === 'env' ? llm.model || 'External endpoint' : 'Nothing yet'],
        ]}
      />
      {llm.ref && isAdmin(user) && (
        <p className="ai-muted">
          Veyron AI is using the in-cluster model {llm.ref.namespace}/{llm.ref.name}.{' '}
          <button
            type="button"
            className="link"
            onClick={async () => {
              if (await run(showToast, () => api.putLlmSetting({ clear: true }), llm.env_configured ? 'Back to the configured endpoint' : 'In-cluster model unset')) s.reload();
            }}
          >
            {llm.env_configured ? 'Switch back to the configured endpoint' : 'Stop using it'}
          </button>
        </p>
      )}
      {canWrite(user) && (
        <Band kicker="Deploy" title="Serve a model" lede="Creates a VM that serves an OpenAI-compatible API on port 8000, behind a ClusterIP Service." wide>
          <form className="insight-form" onSubmit={create}>
            <label>
              Name
              <input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="qwen-small" required />
            </label>
            <label>
              Namespace
              <input value={form.namespace} onChange={(e) => setForm({ ...form, namespace: e.target.value })} placeholder="default" />
            </label>
            <label>
              Runtime
              <select
                value={form.runtime}
                onChange={(e) => {
                  const r = RUNTIMES.find((x) => x[0] === e.target.value);
                  setForm({ ...form, runtime: r[0], model: r[2], memory: r[0] === 'vllm' ? '32Gi' : '8Gi', cpus: r[0] === 'vllm' ? 8 : 4 });
                }}
              >
                {RUNTIMES.map(([v, l]) => (
                  <option key={v} value={v}>
                    {l}
                  </option>
                ))}
              </select>
            </label>
            <label className="wide">
              {form.runtime === 'vllm' ? 'Hugging Face model' : 'GGUF (URL or owner/repo/file.gguf)'}
              <input value={form.model} onChange={(e) => setForm({ ...form, model: e.target.value })} placeholder={runtime?.[2]} required />
            </label>
            {form.runtime === 'vllm' && (
              <label>
                GPUs
                <input type="number" min="1" max="8" value={form.gpu_count} onChange={(e) => setForm({ ...form, gpu_count: e.target.value })} />
              </label>
            )}
            <label>
              vCPU
              <input type="number" min="1" max="64" value={form.cpus} onChange={(e) => setForm({ ...form, cpus: e.target.value })} />
            </label>
            <label>
              Memory
              <input value={form.memory} onChange={(e) => setForm({ ...form, memory: e.target.value })} />
            </label>
            <button type="submit" className="btn primary sm">
              <Cpu size={13} /> Deploy
            </button>
          </form>
        </Band>
      )}
      <Band kicker="Serving" title="Models in this cluster" wide>
        <DataTable
          cols={[
            ['name', 'Model'],
            ['status', 'Status', 'st'],
            ['runtime', 'Runtime'],
            ['model', 'Weights', 'm'],
            ['served', 'Served as', 'm'],
            ['endpoint', 'Endpoint', 'm'],
            ['act', '', 'act'],
          ]}
          rows={rows}
          loading={!s.loaded('models')}
          error={s.err('models')}
          empty="No models yet."
        />
      </Band>
    </Shell>
  );
}

