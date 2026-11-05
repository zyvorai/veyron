import { useCallback, useEffect, useState } from 'react';
import { Check } from 'lucide-react';
import { api } from './api.js';
import { canMutate } from './roles.js';
import { planByline, waitingForApproval } from './planutil.js';

/** Plans a colleague drafted that need a second person's approval (four-eyes). */
export function PlanInbox({ user, open }) {
  const [plans, setPlans] = useState([]);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const enabled = open && canMutate(user?.role);

  const load = useCallback(async () => {
    try {
      setPlans(await api.listPlans('pending'));
    } catch {
      setPlans([]);
    }
  }, []);

  useEffect(() => {
    if (!enabled) return undefined;
    load();
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  }, [enabled, load]);

  const waiting = waitingForApproval(plans, user?.username);
  if (!enabled || waiting.length === 0) return null;

  const approve = async (id) => {
    setBusy(id);
    setErr('');
    try {
      await api.approvePlan(id);
      await load();
    } catch (e) {
      setErr(e.message || 'Could not approve');
    } finally {
      setBusy('');
    }
  };

  return (
    <section className="plan-inbox" aria-label="Plans awaiting your approval">
      <h4>Awaiting your approval ({waiting.length})</h4>
      <ul>
        {waiting.map((p) => (
          <li key={p.id}>
            <div className="plan-inbox-main">
              <b>{p.prompt || p.id}</b>
              <small>{planByline(p)}</small>
              {(p.preview || []).map((line) => (
                <small key={line}>{line}</small>
              ))}
              {p.steps > (p.preview || []).length && <small>and {p.steps - (p.preview || []).length} more</small>}
            </div>
            <button className="tb primary" disabled={busy === p.id} onClick={() => approve(p.id)}>
              <Check size={13} /> {busy === p.id ? 'Approving…' : 'Approve'}
            </button>
          </li>
        ))}
      </ul>
      {err && <div className="plan-err">{err}</div>}
      <small className="plan-inbox-note">Approving lets the person who drafted it run the plan. It does not run it.</small>
    </section>
  );
}
