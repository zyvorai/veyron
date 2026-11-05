import { useEffect, useState } from 'react';
import { Check, AlertTriangle, ShieldAlert } from 'lucide-react';
import { api } from './api.js';
import { canUndo, needsSecondApprover } from './planutil.js';

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
  const [typedCount, setTypedCount] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const destructive = plan.steps.filter((s) => s.destructive);
  const typedNames = typed.split(/[\s,]+/).filter(Boolean);
  const namesOk = destructive.every((s) => typedNames.includes(s.name));
  // Blast-radius guard: plans over the server's threshold need the target count typed back.
  const needCount = plan.confirm_count ?? null;
  const countOk = needCount == null || Number(typedCount.trim()) === needCount;
  const confirmed = namesOk && countOk;
  const namespaces = new Set(plan.steps.map((s) => s.namespace).filter(Boolean)).size;
  const blocked = plan.steps.filter((s) => s.blocked).length;
  const pending = plan.status === 'pending';
  const waitingApproval = pending && needsSecondApprover(plan);

  // Four-eyes: pick up the second person's approval without making the user reload.
  useEffect(() => {
    if (!waitingApproval) return undefined;
    const t = setInterval(async () => {
      try {
        const fresh = await api.getPlan(plan.id);
        if (fresh?.steps) setPlan(fresh);
      } catch {
        /* keep the card as it is; the next tick retries */
      }
    }, 8000);
    return () => clearInterval(t);
  }, [waitingApproval, plan.id]);

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
      {waitingApproval && (
        <div className="plan-approval wait" role="note">
          <ShieldAlert size={13} /> Needs approval from a second person (Write role) before it can run.
        </div>
      )}
      {pending && plan.needs_approval && plan.approved_by && (
        <div className="plan-approval ok" role="note">
          <Check size={13} /> Approved by {plan.approved_by}
        </div>
      )}
      {needCount != null && (
        <div className="plan-blast" role="note">
          <ShieldAlert size={13} className="bad" /> Blast radius: {needCount} targets
          {namespaces > 1 ? ` in ${namespaces} namespaces` : ''}
        </div>
      )}
      {pending && needCount != null && (
        <label className="plan-confirm">
          Type {needCount} to confirm
          <input
            value={typedCount}
            onChange={(e) => setTypedCount(e.target.value)}
            inputMode="numeric"
            aria-label="Type the number of targets to confirm"
          />
        </label>
      )}
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
            disabled={busy || !confirmed || waitingApproval}
            onClick={() =>
              act(
                () =>
                  api.executePlan(
                    plan.id,
                    destructive.map((s) => s.name).filter((n) => typedNames.includes(n)),
                    needCount == null ? null : Number(typedCount.trim()),
                  ),
                'Plan executed',
              )
            }
          >
            {busy ? 'Working…' : `Approve & run (${plan.steps.length})`}
          </button>
        </div>
      )}
      {canUndo(plan) && (
        <div className="plan-f">
          <small>Drafts a new plan that reverses the steps that succeeded; you still approve it.</small>
          <button
            className="tb"
            disabled={busy}
            onClick={() => act(async () => (await api.undoPlan(plan.id))?.plan, 'Undo plan drafted')}
          >
            Undo
          </button>
        </div>
      )}
    </div>
  );
}
