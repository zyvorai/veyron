import { useEffect, useState } from 'react';
import {
  Play, Square, Pause, ArrowLeftRight, Terminal, Camera, Trash2, RefreshCw,
  Activity, Copy, HardDrive, Ban, CheckCircle,
} from 'lucide-react';
import { Status, Spark } from './status.jsx';
import { api } from './api.js';

export const ACT = {
  start: ['Start', Play],
  stop: ['Stop', Square],
  restart: ['Restart', RefreshCw],
  migrate: ['Migrate', ArrowLeftRight],
  console: ['Console', Terminal],
  snapshot: ['Snapshot', Camera],
  delete: ['Delete', Trash2],
  cordon: ['Cordon', Ban],
  uncordon: ['Uncordon', CheckCircle],
  reboot: ['Reboot', RefreshCw],
  logs: ['Logs', Activity],
  clone: ['Clone', Copy],
  resize: ['Resize', HardDrive],
  restore: ['Restore', RefreshCw],
  run: ['Run now', Play],
  pause: ['Pause', Pause],
};

export function Inspector({ res, row, hide, onAct }) {
  const [tab, setTab] = useState('Info');
  const [events, setEvents] = useState([]);
  const [metrics, setMetrics] = useState(null);
  const [loadingExtra, setLoadingExtra] = useState(false);

  useEffect(() => {
    setTab('Info');
    setEvents([]);
    setMetrics(null);
    if (!row || res?.kind !== 'VirtualMachine') return;
    let cancelled = false;
    setLoadingExtra(true);
    (async () => {
      try {
        const [ev, met] = await Promise.all([
          api.vmEvents(row.ns || 'default', row.name).catch(() => []),
          api.vmMetrics(row.name, row.ns || 'default').catch(() => null),
        ]);
        if (!cancelled) {
          setEvents(Array.isArray(ev) ? ev.slice(0, 12) : []);
          setMetrics(met);
        }
      } finally {
        if (!cancelled) setLoadingExtra(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [row?.id, res?.kind]);

  if (!row) {
    return (
      <aside className={`insp ${hide ? 'hide' : ''}`}>
        <div className="empty">
          <div>
            <b>No selection</b>
            Select an item to inspect it.
          </div>
        </div>
      </aside>
    );
  }

  const infoRows = (res.insp || []).map(([l, k]) => [l, row[k]]);
  const eventRows =
    events.length > 0
      ? events.map((e) => [
          e.reason || e.type_ || e.type || 'Event',
          e.message || e.timestamp || '—',
        ])
      : [['Events', loadingExtra ? 'Loading…' : 'No recent events']];

  const tabs = {
    Info: infoRows,
    Events: eventRows,
  };

  let acts = (res.acts || []).slice();
  // Prefer console/logs early so Running VMs still show Console (slice would
  // otherwise keep only start/stop/restart/pause).
  const prefer = ['console', 'logs', 'start', 'stop', 'cordon', 'uncordon'];
  acts.sort((a, b) => {
    const ia = prefer.indexOf(a);
    const ib = prefer.indexOf(b);
    if (ia === -1 && ib === -1) return 0;
    if (ia === -1) return 1;
    if (ib === -1) return -1;
    return ia - ib;
  });
  acts = acts.slice(0, 4).map((a) => {
    if (a === 'start' && row.status === 'Running') return 'stop';
    if (a === 'stop' && row.status !== 'Running') return 'start';
    if (a === 'cordon' && row.status === 'Cordoned') return 'uncordon';
    if (a === 'uncordon' && row.status !== 'Cordoned') return 'cordon';
    return a;
  });
  acts = acts.filter((a, i, s) => s.indexOf(a) === i);

  const cpuPct = metrics?.cpu_usage_percent;
  const memPct = metrics?.memory_usage_percent;
  const hasMetrics = cpuPct != null || memPct != null;

  return (
    <aside className={`insp ${hide ? 'hide' : ''}`}>
      <div className="insp-h">
        <h2>{row.name}</h2>
        <div className="kind">
          {res.kind}
          {row.status && (
            <>
              {' · '}
              <Status s={row.status} />
            </>
          )}
        </div>
      </div>
      {res.spark && (
        <div className="spark">
          <div>
            <small>
              CPU <span>{cpuPct != null ? `${Math.round(cpuPct)}%` : '—'}</span>
            </small>
            <b>{row.cpu ?? '—'} vCPU</b>
            {hasMetrics ? (
              <Spark data={[0, cpuPct || 0, cpuPct || 0]} color="var(--accent)" />
            ) : (
              <small style={{ color: 'var(--ink-3)' }}>{loadingExtra ? 'Loading metrics…' : 'No live metrics'}</small>
            )}
          </div>
          <div>
            <small>
              Memory <span>{memPct != null ? `${Math.round(memPct)}%` : '—'}</span>
            </small>
            <b>{row.ram ?? '—'} GiB</b>
            {hasMetrics ? (
              <Spark data={[0, memPct || 0, memPct || 0]} color="var(--green)" />
            ) : (
              <small style={{ color: 'var(--ink-3)' }}>{loadingExtra ? '…' : '—'}</small>
            )}
          </div>
        </div>
      )}
      <div className="itabs" role="tablist">
        {Object.keys(tabs).map((t) => (
          <button key={t} role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
            {t}
          </button>
        ))}
      </div>
      <div className="form">
        {tabs[tab].map(([l, v], i) => (
          <div className="frow" key={`${l}-${i}`}>
            <label>{l}</label>
            <span className="mono">{String(v ?? '—')}</span>
          </div>
        ))}
      </div>
      <div className="insp-acts">
        {acts.map((a) => {
          const [l, I] = ACT[a] || [a, Activity];
          return (
            <button
              key={a}
              className={`btn ${a === 'console' || a === 'logs' ? 'primary' : a === 'delete' ? 'danger' : ''}`}
              onClick={() => onAct(a)}
            >
              <I size={13} />
              {l}
            </button>
          );
        })}
      </div>
    </aside>
  );
}
