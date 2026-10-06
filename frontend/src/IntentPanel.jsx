import { useState } from 'react';
import { Sparkles, Loader2, Check, AlertTriangle, XCircle, Code2 } from 'lucide-react';
import { api } from './api.js';
import { ProposalCard } from './ProposalCard.jsx';

const EXAMPLES = [
  'Postgres server on Debian with 4 cores, 16 GB RAM and a 200 GB disk',
  'Three Ubuntu web servers, small',
  'Windows Server 2022 for Active Directory, no internet',
];

function CheckIcon({ level }) {
  if (level === 'ok') return <Check size={13} className="ai-ok" />;
  if (level === 'warn') return <AlertTriangle size={13} className="ai-warn" />;
  return <XCircle size={13} className="ai-bad" />;
}

function money(n) {
  return n == null ? '—' : `$${Number(n).toFixed(2)}`;
}

/** "Describe it": a sentence becomes VM specs with cost, preflight and YAML; creating goes through a proposal. */
export function IntentPanel({ user, namespace, onUseSpec }) {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState('');
  const [plan, setPlan] = useState(null);
  const [err, setErr] = useState('');
  const [yamlOpen, setYamlOpen] = useState(-1);

  const interpret = async () => {
    if (!text.trim()) return;
    setBusy('plan');
    setErr('');
    setPlan(null);
    try {
      setPlan(await api.aiIntentVm({ text, namespace }));
    } catch (e) {
      setErr(e.message);
    } finally {
      setBusy('');
    }
  };

  const propose = async () => {
    setBusy('submit');
    setErr('');
    try {
      const r = await api.aiIntentVm({ text, namespace, specs: plan.items.map((i) => i.spec), submit: true });
      setPlan({ ...plan, proposal: r.proposal });
    } catch (e) {
      setErr(e.message);
    } finally {
      setBusy('');
    }
  };

  const blocked = plan?.items?.some((i) => !i.ok);
  const canWrite = user?.role === 'admin' || user?.role === 'write';

  return (
    <div className="ai-intent">
      <label className="ai-intent-label">
        <Sparkles size={14} /> Describe it
      </label>
      <div className="ai-intent-row">
        <textarea
          rows={2}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              interpret();
            }
          }}
          placeholder={EXAMPLES[0]}
          aria-label="Describe the machine you want"
        />
        <button className="btn" disabled={!text.trim() || !!busy} onClick={interpret}>
          {busy === 'plan' ? <Loader2 size={14} className="spin" /> : 'Plan it'}
        </button>
      </div>
      {!plan && !busy && (
        <div className="ai-intent-ex">
          {EXAMPLES.map((x) => (
            <button key={x} type="button" className="chip" onClick={() => setText(x)}>
              {x}
            </button>
          ))}
        </div>
      )}
      {err && <p className="ai-bad">{err}</p>}
      {plan && (
        <div className="ai-intent-plan">
          <p className="ai-muted">
            {plan.explanation}
            <span className="ai-src"> · {plan.source === 'llm' ? 'model' : plan.source === 'rules' ? 'rules (no model configured)' : plan.source}</span>
          </p>
          {plan.notes?.map((n) => (
            <p key={n} className="ai-muted">
              {n}
            </p>
          ))}
          {plan.items.map((it, i) => (
            <div key={it.spec.name} className="ai-intent-item">
              <div className="ai-intent-item-h">
                <b className="mono">
                  {it.spec.namespace}/{it.spec.name}
                </b>
                <span>
                  {it.spec.template} · {it.spec.cpus ?? 'template'} vCPU · {it.spec.memory ?? 'template'}
                  {it.spec.disk_size ? ` · ${it.spec.disk_size} disk` : ''}
                  {it.spec.gpu ? ` · ${it.spec.gpu.count} GPU` : ''}
                  {it.spec.allow_internet === false ? ' · no internet' : ''}
                </span>
                <span className="ai-cost" title={it.cost.basis}>
                  {money(it.cost.monthly)}/mo
                </span>
              </div>
              <ul className="ai-checks">
                {it.preflight.map((c) => (
                  <li key={c.check + c.message}>
                    <CheckIcon level={c.level} /> {c.message}
                  </li>
                ))}
              </ul>
              <div className="ai-proposal-acts">
                {onUseSpec && (
                  <button className="btn sm" onClick={() => onUseSpec(it.spec)}>
                    Edit in form
                  </button>
                )}
                {it.yaml && (
                  <button className="btn sm" onClick={() => setYamlOpen(yamlOpen === i ? -1 : i)}>
                    <Code2 size={13} /> {yamlOpen === i ? 'Hide YAML' : 'YAML'}
                  </button>
                )}
              </div>
              {yamlOpen === i && <pre className="ai-yaml">{it.yaml}</pre>}
            </div>
          ))}
          {plan.items.length > 1 && <p className="ai-muted">Total about {money(plan.total_monthly)} a month.</p>}
          {plan.proposal ? (
            <ProposalCard proposal={plan.proposal} user={user} />
          ) : (
            <div className="ai-proposal-acts">
              <button className="btn primary" disabled={!!busy || !canWrite} onClick={propose} title={canWrite ? '' : 'Needs the write role'}>
                {busy === 'submit' ? <Loader2 size={14} className="spin" /> : null}
                {plan.items.length > 1 ? `Propose ${plan.items.length} machines` : 'Propose this machine'}
              </button>
              {blocked && <span className="ai-warn">Preflight found problems; the create may fail.</span>}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
