import { useState } from 'react';
import { Check, AlertTriangle, ShieldAlert } from 'lucide-react';
import { api } from './api.js';

const STATUS_LABEL = {
  pending: 'Awaiting approval',
  executing: 'Executing…',
  executed: 'Executed',
  failed: 'Failed',
  cancelled: 'Cancelled',
};

/** A plan the agent drafted. Nothing runs until the user approves it here. */
export function CopilotPlanCard({ initial, onDone }) {
  const [plan, setPlan] = useState(initial);
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const destructive = plan.steps.filter((s) => s.destructive);
  const typedNames = typed.split(/[\s,]+/).filter(Boolean);
  const confirmed = destructive.every((s) => typedNames.includes(s.name));
  const blocked = plan.steps.filter((s) => s.blocked).length;
  const pending = plan.status === 'pending';

  const act = async (fn, label) => {
    setBusy(true);
    setErr('');
    try {
      const next = await fn();
      if (next?.steps) setPlan(next);
      onDone?.(label, next);
    } catch (e) {
      setErr(e.message || 'Request failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={`plan ${plan.status}`}>
      <div className="plan-h">
        <b>Proposed plan</b>
        <span className="plan-st">{STATUS_LABEL[plan.status] || plan.status}</span>
        <span className={`plan-role ${plan.min_role}`}>{plan.min_role} role</span>
      </div>
      <ul className="plan-steps">
        {plan.steps.map((s) => (
          <li key={`${s.kind}:${s.namespace}:${s.name}`} className={s.blocked ? 'blocked' : ''}>
            {s.result ? (
              s.result.ok ? <Check size={13} className="ok" /> : <AlertTriangle size={13} className="bad" />
            ) : s.destructive ? (
              <ShieldAlert size={13} className="bad" />
            ) : (
              <i className="dot" />
            )}
            <span>{s.preview}</span>
            {s.destructive && <em className="tag">destructive</em>}
            {s.blocked && <small className="why">Blocked: {s.blocked}</small>}
            {s.result && !s.result.ok && <small className="why">{s.result.error}</small>}
          </li>
        ))}
      </ul>
      {pending && destructive.length > 0 && (
        <label className="plan-confirm">
          Type {destructive.map((s) => s.name).join(', ')} to confirm
          <input value={typed} onChange={(e) => setTyped(e.target.value)} aria-label="Type names to confirm" />
        </label>
      )}
      {err && <div className="plan-err">{err}</div>}
      {pending && (
        <div className="plan-f">
          {blocked > 0 && <small>{blocked} step(s) blocked — they will fail and stop the plan.</small>}
          <button className="tb" disabled={busy} onClick={() => act(() => api.cancelPlan(plan.id), 'Plan cancelled')}>
            Cancel
          </button>
          <button
            className="tb primary"
            disabled={busy || !confirmed}
            onClick={() =>
              act(() => api.executePlan(plan.id, destructive.map((s) => s.name).filter((n) => typedNames.includes(n))), 'Plan executed')
            }
          >
            {busy ? 'Working…' : `Approve & run (${plan.steps.length})`}
          </button>
        </div>
      )}
    </div>
  );
}

