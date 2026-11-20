import { useEffect, useState } from 'react';
import { Check, X, Camera, ShieldAlert, Undo2, Loader2 } from 'lucide-react';
import { api, canApprove, proposalApproverRole, relTime } from './api.js';

const STATUS_LABEL = {
  pending: 'Waiting for approval',
  running: 'Running',
  succeeded: 'Done',
  failed: 'Failed',
  rolled_back: 'Rolled back',
  rejected: 'Rejected',
  expired: 'Expired',
};

export function ProposalStatus({ s }) {
  return (
    <span className="ai-pstatus" data-s={s}>
      {s === 'running' && <Loader2 size={11} className="spin" />}
      {STATUS_LABEL[s] || s}
    </span>
  );
}

/** One AI change proposal: what it does, what it touches, and the approve/reject controls. */
export function ProposalCard({ proposal, user, onChange, compact }) {
  const [p, setP] = useState(proposal);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  useEffect(() => setP(proposal), [proposal]);

  useEffect(() => {
    if (p?.status !== 'running') return undefined;
    const t = setInterval(async () => {
      try {
        const next = await api.getProposal(p.id);
        if (next) {
          setP(next);
          if (next.status !== 'running') onChange?.(next);
        }
      } catch {
        /* keep polling */
      }
    }, 2500);
    return () => clearInterval(t);
  }, [p?.id, p?.status, onChange]);

  if (!p) return null;
  const br = p.blast_radius || {};
  const allowed = canApprove(user, p);
  const needs = proposalApproverRole(p);
  const canSnap = (p.snapshot_targets || []).length > 0;

  const decide = async (kind, snap = false) => {
    setBusy(kind + (snap ? '-snap' : ''));
    setErr('');
    try {
      const next = kind === 'approve' ? await api.approveProposal(p.id, snap) : await api.rejectProposal(p.id);
      if (next) {
        setP(next);
        onChange?.(next);
      }
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy('');
    }
  };

  return (
    <div className="ai-proposal" data-s={p.status}>
      <div className="ai-proposal-head">
        <b>{p.title}</b>
        <ProposalStatus s={p.status} />
      </div>
      {p.rationale && !compact && <p className="ai-proposal-why">{p.rationale}</p>}

      {(p.diff || []).length > 0 && (
        <table className="ai-diff">
          <tbody>
            {p.diff.map((d, i) => (
              <tr key={i}>
                <td>{d.target}</td>
                <td>{d.field}</td>
                <td className="before">{d.before || '—'}</td>
                <td className="arrow">→</td>
                <td className="after">{d.after || '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="ai-blast">
        {(br.vms || []).length > 0 && <span>{br.vms.length === 1 ? br.vms[0] : `${br.vms.length} VMs`}</span>}
        {(br.nodes || []).length > 0 && <span>{br.nodes.join(', ')}</span>}
        {br.downtime && (
          <span className="warn">
            <ShieldAlert size={11} /> Downtime
          </span>
        )}
        {br.reversible && (
          <span>
            <Undo2 size={11} /> Reversible
          </span>
        )}
        {br.monthly_cost_delta != null && (
          <span className={br.monthly_cost_delta > 0 ? 'warn' : ''}>
            {br.monthly_cost_delta > 0 ? '+' : ''}${Math.round(br.monthly_cost_delta)}/mo
          </span>
        )}
        {(br.notes || []).map((n) => (
          <span key={n}>{n}</span>
        ))}
      </div>

      {!compact && (p.steps || []).length > 0 && (
        <ol className="ai-steps">
          {p.steps.map((s, i) => {
            const r = (p.results || [])[i];
            return (
              <li key={i} data-ok={r ? String(r.ok) : undefined}>
                {s.summary}
                <code>
                  {s.method} {s.path}
                </code>
                {r && !r.ok && <small>{r.message}</small>}
              </li>
            );
          })}
        </ol>
      )}

      {p.status === 'pending' ? (
        <div className="ai-proposal-acts">
          {allowed ? (
            <>
              <button className="btn sm primary" disabled={!!busy} onClick={() => decide('approve', false)}>
                <Check size={13} /> Approve
              </button>
              {canSnap && (
                <button className="btn sm secondary" disabled={!!busy} onClick={() => decide('approve', true)}>
                  <Camera size={13} /> Snapshot first, then approve
                </button>
              )}
              <button className="btn sm secondary" disabled={!!busy} onClick={() => decide('reject')}>
                <X size={13} /> Reject
              </button>
            </>
          ) : (
            <small className="ai-muted">Needs a user with the {needs} role to approve.</small>
          )}
        </div>
      ) : (
        <div className="ai-proposal-foot">
          {p.decided_by && <span>{p.status === 'rejected' ? 'Rejected' : 'Approved'} by {p.decided_by}</span>}
          <span>{relTime(p.updated_at)}</span>
        </div>
      )}
      {err && <div className="ops-err">{err}</div>}
      {!compact && (p.log || []).length > 0 && p.status !== 'pending' && (
        <details className="ai-log">
          <summary>Run log</summary>
          <pre>{p.log.join('\n')}</pre>
        </details>
      )}
      <div className="ai-proposal-meta">
        Proposed by {p.proposed_by} · {p.source} · <code>{p.id}</code>
      </div>
    </div>
  );
}
