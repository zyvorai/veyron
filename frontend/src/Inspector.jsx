import { useEffect, useState } from 'react';
import {
  Play, Square, Pause, ArrowLeftRight, Terminal, Camera, Trash2, RefreshCw,
  Activity, Copy, HardDrive, Ban, CheckCircle, Network, Monitor, Zap, Unlink, Sparkles, Disc,
} from 'lucide-react';
import { Status, Spark } from './status.jsx';
import { api } from './api.js';
import { VmGuest, VmDisks } from './VmExtras.jsx';

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
  unpause: ['Unpause', Play],
  publish: ['Publish', HardDrive],
  capture: ['Capture image', Camera],
  eject: ['Eject media', Disc],
  resolve: ['Resolve', CheckCircle],
  ack: ['Acknowledge', CheckCircle],
  reclaim: ['Reclaim…', Unlink],
};

const RUN_STRATEGIES = ['Always', 'Manual', 'Halted', 'RerunOnFailure'];

function resolveActs(res, row) {
  let acts = (res.acts || []).slice();
  if (!row.cdroms?.length) acts = acts.filter((a) => a !== 'eject');
  const prefer = ['console', 'logs', 'start', 'stop', 'pause', 'unpause', 'cordon', 'uncordon'];
  acts.sort((a, b) => {
    const ia = prefer.indexOf(a);
    const ib = prefer.indexOf(b);
    if (ia === -1 && ib === -1) return 0;
    if (ia === -1) return 1;
    if (ib === -1) return -1;
    return ia - ib;
  });
  acts = acts.map((a) => {
    if (a === 'start' && row.status === 'Running') return 'stop';
    if (a === 'stop' && row.status !== 'Running') return 'start';
    if (a === 'pause' && /paus/i.test(String(row.status || ''))) return 'unpause';
    if (a === 'unpause' && !/paus/i.test(String(row.status || ''))) return 'pause';
    if (a === 'cordon' && row.status === 'Cordoned') return 'uncordon';
    if (a === 'uncordon' && row.status !== 'Cordoned') return 'cordon';
    return a;
  });
  return acts.filter((a, i, s) => s.indexOf(a) === i);
}

function VmOps({ row, onDone }) {
  const ns = row.ns || 'default';
  const name = row.name;
  const [expose, setExpose] = useState(null);
  const [rdp, setRdp] = useState(null);
  const [guest, setGuest] = useState(null);
  const [inet, setInet] = useState(null);
  const [drift, setDrift] = useState(null);
  const [strategy, setStrategy] = useState('Always');
  const [sockets, setSockets] = useState(String(parseInt(String(row.cpu), 10) || 2));
  const [memory, setMemory] = useState(`${Number(row.ram) || 2}Gi`);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');

  const refresh = async () => {
    const [ex, rd, gs, net, dr] = await Promise.all([
      api.getExpose(ns, name).catch(() => null),
      api.getRdpExpose(ns, name).catch(() => null),
      api.getGuestStatus(ns, name).catch(() => null),
      api.getInternet(ns, name).catch(() => null),
      row.managed ? api.getDrift(ns, name).catch(() => null) : null,
    ]);
    setExpose(ex);
    setRdp(rd);
    setGuest(gs);
    setInet(net);
    setDrift(dr);
  };

  useEffect(() => {
    let cancelled = false;
    setErr('');
    (async () => {
      try {
        await refresh();
      } catch (e) {
        if (!cancelled) setErr(e.message || String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [row.id]);

  const run = async (label, fn) => {
    setBusy(label);
    setErr('');
    try {
      await fn();
      await refresh();
      onDone?.(label);
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy('');
    }
  };

  const sshOn = !!(expose?.enabled || expose?.service);
  const rdpOn = !!(rdp?.enabled || rdp?.service);
  const inetOn = !!(inet?.enabled || inet?.allowed || inet?.policy);
  const drifted = !!(drift?.drift_detected);

  return (
    <div className="ops">
      {err && <div className="ops-err">{err}</div>}
      <div className="ops-block">
        <h4>
          <Network size={13} /> Access
        </h4>
        <div className="ops-row">
          <span>
            SSH expose
            <small>{sshOn ? expose?.service_type || expose?.type || 'on' : 'off'}</small>
          </span>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() =>
              run(
                sshOn ? 'SSH off' : 'SSH on',
                () =>
                  sshOn
                    ? api.deleteExpose(ns, name)
                    : api.putExpose(ns, name, {
                        enabled: true,
                        service_type: 'ClusterIP',
                        ports: [{ port: 22, target_port: 22 }],
                      }),
              )
            }
          >
            {busy === 'SSH on' || busy === 'SSH off' ? '…' : sshOn ? 'Disable' : 'Enable'}
          </button>
        </div>
        <div className="ops-row">
          <span>
            RDP expose
            <small>{rdpOn ? rdp?.service_type || 'ClusterIP' : 'off'}</small>
          </span>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() =>
              run(
                rdpOn ? 'RDP off' : 'RDP on',
                () =>
                  rdpOn
                    ? api.deleteRdpExpose(ns, name)
                    : api.putRdpExpose(ns, name, {
                        enabled: true,
                        service_type: 'ClusterIP',
                      }),
              )
            }
          >
            {busy.startsWith('RDP') ? '…' : rdpOn ? 'Disable' : 'Enable'}
          </button>
        </div>
        <div className="ops-row">
          <span>
            Internet egress
            <small>{inetOn ? 'allowed' : 'restricted / unknown'}</small>
          </span>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() =>
              run(
                inetOn ? 'Internet off' : 'Internet on',
                () => (inetOn ? api.deleteInternet(ns, name) : api.putInternet(ns, name)),
              )
            }
          >
            {busy.startsWith('Internet') ? '…' : inetOn ? 'Revoke' : 'Allow'}
          </button>
        </div>
        <div className="ops-acts" style={{ marginTop: 8 }}>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Enable RDP guest', () => api.enableRdpGuest(ns, name))}
          >
            Enable RDP in guest
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Disable RDP guest', () => api.disableRdpGuest(ns, name))}
          >
            Disable RDP in guest
          </button>
        </div>
      </div>

      <div className="ops-block">
        <h4>
          <Monitor size={13} /> Guest
        </h4>
        <small className="ops-hint">
          {guest
            ? guest.connected || guest.guest_agent_connected
              ? 'Guest agent connected'
              : 'Guest agent not connected — these actions need it'
            : 'Guest status unavailable'}
        </small>
        <div className="ops-acts">
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Soft reboot', () => api.guestSoftReboot(ns, name))}
          >
            Soft reboot
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Freeze', () => api.guestFreeze(ns, name))}
          >
            Freeze
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Unfreeze', () => api.guestUnfreeze(ns, name))}
          >
            Unfreeze
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => {
              if (!window.confirm(`Patch OS packages inside ${name}?`)) return;
              run('Guest patch', () => api.guestPatch(ns, name, { snapshot_first: true }));
            }}
          >
            Guest patch
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Reclaim', () => api.disksReclaim(ns, name))}
          >
            fstrim reclaim
          </button>
          <button
            className="btn sm secondary"
            disabled={!!busy}
            onClick={() => run('Ceph snap', () => api.atlasVmCephSnapshot(ns, name))}
          >
            Atlas Ceph snap
          </button>
        </div>
      </div>

      <div className="ops-block">
        <h4>Drift</h4>
        <small className="ops-hint">
          {drift
            ? drifted
              ? drift.drift_message || 'Drift detected'
              : 'In sync with VeyronVM desired state'
            : 'No VeyronVM drift status (unmanaged VM)'}
        </small>
        <button
          className="btn sm secondary"
          disabled={!!busy || !drift}
          onClick={() => run('Remediate', () => api.remediateDrift(ns, name))}
        >
          Remediate drift
        </button>
      </div>

      <div className="ops-block">
        <h4>
          <Zap size={13} /> Hotplug
        </h4>
        <div className="ops-fields">
          <label>
            Sockets
            <input value={sockets} onChange={(e) => setSockets(e.target.value)} />
          </label>
          <label>
            Memory
            <input value={memory} onChange={(e) => setMemory(e.target.value)} placeholder="4Gi" />
          </label>
        </div>
        <button
          className="btn sm primary"
          disabled={!!busy}
          onClick={() => {
            const s = parseInt(sockets, 10);
            if (!window.confirm(`Hotplug ${name} to ${s || '?'} sockets / ${memory}?`)) return;
            run('Hotplug', () =>
              api.hotplugVm(ns, name, {
                sockets: Number.isFinite(s) ? s : undefined,
                memory: memory || undefined,
              }),
            );
          }}
        >
          {busy === 'Hotplug' ? 'Applying…' : 'Apply hotplug'}
        </button>
      </div>

      <div className="ops-block">
        <h4>Run strategy</h4>
        <div className="ops-fields">
          <label>
            Strategy
            <select value={strategy} onChange={(e) => setStrategy(e.target.value)}>
              {RUN_STRATEGIES.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>
        </div>
        <button
          className="btn sm secondary"
          disabled={!!busy}
          onClick={() => run('Run strategy', () => api.setRunStrategy(ns, name, strategy))}
        >
          Set strategy
        </button>
      </div>

      <VmDisks row={row} onDone={onDone} />
    </div>
  );
}

export function Inspector({ res, row, hide, onAct, onOpsDone, onAskAi }) {
  const [tab, setTab] = useState('Info');
  const [events, setEvents] = useState([]);
  const [metrics, setMetrics] = useState(null);
  const [security, setSecurity] = useState(null);
  const [loadingExtra, setLoadingExtra] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);

  useEffect(() => {
    setTab('Info');
    setEvents([]);
    setMetrics(null);
    setSecurity(null);
    setMoreOpen(false);
    if (!row || res?.kind !== 'VirtualMachine') return;
    let cancelled = false;
    setLoadingExtra(true);
    (async () => {
      try {
        const [ev, met, sec] = await Promise.all([
          api.vmEvents(row.ns || 'default', row.name).catch(() => []),
          api.vmMetrics(row.name, row.ns || 'default').catch(() => null),
          api.vmSecurity(row.ns || 'default', row.name).catch(() => null),
        ]);
        if (!cancelled) {
          setEvents(Array.isArray(ev) ? ev.slice(0, 12) : []);
          setMetrics(met);
          setSecurity(sec);
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
  if (res.kind === 'VirtualMachine' && security?.score != null) {
    infoRows.push(['Security score', `${security.score} / 100`]);
    for (const c of security.checks || []) infoRows.push([c.name, `${c.pass ? '✓' : '✗'} ${c.detail || ''}`.trim()]);
  }
  const eventRows =
    events.length > 0
      ? events.map((e) => [
          e.reason || e.type_ || e.type || 'Event',
          e.message || e.timestamp || '—',
        ])
      : [['Events', loadingExtra ? 'Loading…' : 'No recent events']];

  const isVm = res.kind === 'VirtualMachine';
  const tabNames = isVm ? ['Info', 'Events', 'Guest', 'Ops'] : ['Info', 'Events'];
  const acts = resolveActs(res, row);
  const primary = acts.slice(0, 4);
  const more = acts.slice(4);

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
        {onAskAi && (
          <button type="button" className="btn sm secondary insp-ai" onClick={() => onAskAi(row)}>
            <Sparkles size={12} /> Ask AI
          </button>
        )}
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
        {tabNames.map((t) => (
          <button key={t} role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
            {t}
          </button>
        ))}
      </div>
      {tab === 'Ops' && isVm ? (
        <VmOps row={row} onDone={(l) => onOpsDone?.(l)} />
      ) : tab === 'Guest' && isVm ? (
        <VmGuest row={row} />
      ) : (
        <div className="form">
          {(tab === 'Events' ? eventRows : infoRows).map(([l, v], i) => (
            <div className="frow" key={`${l}-${i}`}>
              <label>{l}</label>
              <span className="mono">{String(v ?? '—')}</span>
            </div>
          ))}
        </div>
      )}
      <div className="insp-acts">
        {primary.map((a) => {
          const [l0, I] = ACT[a] || [a, Activity];
          const l = a === 'clone' && res.kind === 'Template' ? 'Create VM' : l0;
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
        {more.length > 0 && (
          <div className="insp-more">
            <button className="btn" onClick={() => setMoreOpen((o) => !o)}>
              More
            </button>
            {moreOpen && (
              <div className="insp-more-menu">
                {more.map((a) => {
                  const [l0, I] = ACT[a] || [a, Activity];
                  const l = a === 'clone' && res.kind === 'Template' ? 'Create VM' : l0;
                  return (
                    <button
                      key={a}
                      className={a === 'delete' ? 'danger' : ''}
                      onClick={() => {
                        setMoreOpen(false);
                        onAct(a);
                      }}
                    >
                      <I size={13} />
                      {l}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </div>
    </aside>
  );
}
