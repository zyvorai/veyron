import { useEffect, useState } from 'react';
import { Activity, Map, Cloud, Radar, RefreshCw, ChevronRight } from 'lucide-react';
import { api } from './api.js';
import { Status } from './status.jsx';

function ChapterShell({ kicker, title, lede, children, onRefresh, busy }) {
  return (
    <div className="chapter-page">
      <div className="apple-chapter">
        <div className="kicker">{kicker}</div>
        <h2>{title}</h2>
        <p>{lede}</p>
        {onRefresh && (
          <div className="acts" style={{ justifyContent: 'flex-start', marginTop: 16 }}>
            <button className="btn" disabled={busy} onClick={onRefresh}>
              <RefreshCw size={13} className={busy ? 'spin' : ''} />
              Refresh
            </button>
          </div>
        )}
      </div>
      {children}
    </div>
  );
}

export function MonitoringPage() {
  const [headroom, setHeadroom] = useState(null);
  const [versions, setVersions] = useState(null);
  const [caps, setCaps] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const load = async () => {
    setBusy(true);
    setErr('');
    try {
      const [h, v, c] = await Promise.all([
        api.capacityHeadroom().catch((e) => ({ error: e.message })),
        api.platformVersions().catch(() => null),
        api.platformCapabilities().catch(() => null),
      ]);
      setHeadroom(h);
      setVersions(v);
      setCaps(c);
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const cards = [
    ['CPU free', headroom?.cpu_available ?? headroom?.cpu?.available ?? headroom?.cpu_free ?? '—'],
    ['Memory free', headroom?.memory_available ?? headroom?.memory?.available ?? headroom?.memory_free ?? '—'],
    ['Nodes', headroom?.node_count ?? caps?.nodes ?? '—'],
    ['Live migration', caps?.live_migration ? 'yes' : caps ? 'no' : '—'],
  ];

  return (
    <ChapterShell
      kicker="Platform"
      title="Monitoring"
      lede="Capacity headroom and installed platform versions."
      onRefresh={load}
      busy={busy}
    >
      {err && <div className="ops-err" style={{ margin: '0 32px 16px' }}>{err}</div>}
      <section className="apple-band-paper">
        <div className="specs">
          {cards.map(([l, v]) => (
            <div key={l}>
              <b>{String(v)}</b>
              <span>{l}</span>
            </div>
          ))}
        </div>
      </section>
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Versions</div>
          <h2>Platform stack</h2>
          <div className="rows">
            {versions ? (
              Object.entries(versions).slice(0, 12).map(([k, v]) => (
                <button type="button" key={k} disabled style={{ cursor: 'default' }}>
                  <b>{k}</b>
                  <span className="why mono">
                    {typeof v === 'object' ? JSON.stringify(v).slice(0, 80) : String(v)}
                  </span>
                </button>
              ))
            ) : (
              <button type="button" disabled>
                <b>Loading…</b>
              </button>
            )}
          </div>
        </div>
      </section>
    </ChapterShell>
  );
}

export function TopologyPage() {
  const [map, setMap] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const load = async () => {
    setBusy(true);
    setErr('');
    try {
      setMap(await api.topologyMap());
    } catch (e) {
      setErr(e.message || String(e));
      setMap(null);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const nodes = map?.nodes || map?.Vertices || map?.items || [];
  const edges = map?.edges || map?.links || map?.connections || [];

  return (
    <ChapterShell
      kicker="Fleet"
      title="Topology"
      lede="Cluster map as nodes and edges from the topology API."
      onRefresh={load}
      busy={busy}
    >
      {err && <div className="ops-err" style={{ margin: '0 32px 16px' }}>{err}</div>}
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Nodes · {Array.isArray(nodes) ? nodes.length : 0}</div>
          <div className="rows">
            {(Array.isArray(nodes) ? nodes : []).slice(0, 40).map((n, i) => {
              const label = n.name || n.id || n.node || `node-${i}`;
              const kind = n.kind || n.type || n.role || '—';
              return (
                <button type="button" key={label} disabled style={{ cursor: 'default' }}>
                  <Status s={n.status || 'Ready'} />
                  <b>{label}</b>
                  <span className="why">{kind}</span>
                </button>
              );
            })}
            {(!nodes || nodes.length === 0) && (
              <button type="button" disabled>
                <b>No topology nodes</b>
                <span className="why">{map?.error || 'Empty map or API unavailable.'}</span>
              </button>
            )}
          </div>
          <div className="kicker" style={{ marginTop: 28 }}>
            Edges · {Array.isArray(edges) ? edges.length : 0}
          </div>
          <div className="rows">
            {(Array.isArray(edges) ? edges : []).slice(0, 30).map((e, i) => (
              <button type="button" key={i} disabled style={{ cursor: 'default' }}>
                <b>
                  {e.source || e.from || '?'} → {e.target || e.to || '?'}
                </b>
                <span className="why">{e.kind || e.type || 'link'}</span>
              </button>
            ))}
          </div>
        </div>
      </section>
    </ChapterShell>
  );
}

export function DrPage({ showToast }) {
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');

  const load = async () => {
    setBusy('load');
    setErr('');
    try {
      setStatus(await api.veleroStatus());
    } catch (e) {
      setErr(e.message || String(e));
      setStatus({ configured: false, error: e.message });
    } finally {
      setBusy('');
    }
  };

  useEffect(() => {
    load();
  }, []);

  const run = async (label, fn) => {
    setBusy(label);
    setErr('');
    try {
      await fn();
      showToast?.(`${label} · done`);
      await load();
    } catch (e) {
      setErr(e.message || String(e));
      showToast?.(e.message || String(e), true);
    } finally {
      setBusy('');
    }
  };

  const configured = status?.configured !== false && !status?.error;

  return (
    <ChapterShell
      kicker="Disaster recovery"
      title="DR & Velero"
      lede="Cluster backup and failback controls. Dry-run friendly where the API allows."
      onRefresh={load}
      busy={busy === 'load'}
    >
      {err && <div className="ops-err" style={{ margin: '0 32px 16px' }}>{err}</div>}
      <section className="apple-band-paper">
        <div className="apple-chapter">
          <div className="kicker">Status</div>
          <h2>{configured ? 'Velero reachable' : 'Not configured'}</h2>
          <p className="mono" style={{ marginTop: 8, color: 'var(--ink-2)' }}>
            {status?.namespace || status?.message || status?.error || 'VEYRON / Velero integration'}
          </p>
          <div className="apple-shelf" style={{ marginTop: 24 }}>
            <button
              className="apple-tile"
              disabled={!!busy}
              onClick={() =>
                run('Velero backup', () =>
                  api.createVeleroBackup({
                    name: `veyron-${Date.now().toString(36).slice(-6)}`,
                    included_namespaces: ['default'],
                    dry_run: true,
                  }),
                )
              }
            >
              <Cloud size={22} />
              <b>Create Velero backup</b>
              <p>Dry-run when the API supports it.</p>
            </button>
            <button
              className="apple-tile"
              disabled={!!busy}
              onClick={() => {
                const name = window.prompt('Backup name to restore');
                if (!name) return;
                run('Velero restore', () =>
                  api.createVeleroRestore({ backup_name: name, dry_run: true }),
                );
              }}
            >
              <RefreshCw size={22} />
              <b>Restore from backup</b>
              <p>Prompt for backup name.</p>
            </button>
            <button
              className="apple-tile"
              disabled={!!busy}
              onClick={() => {
                const ns = window.prompt('Namespace for failback', 'default');
                const vm = window.prompt('VM name');
                if (!ns || !vm) return;
                if (!window.confirm(`Run DR failback for ${ns}/${vm}?`)) return;
                run('DR failback', () => api.drFailback({ namespace: ns, name: vm }));
              }}
            >
              <ChevronRight size={22} />
              <b>DR failback</b>
              <p>Confirm before mutating.</p>
            </button>
          </div>
        </div>
      </section>
    </ChapterShell>
  );
}

export function PacketWolfPage() {
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const load = async () => {
    setBusy(true);
    setErr('');
    try {
      setStatus(await api.packetwolfStatus());
    } catch (e) {
      setErr(e.message || String(e));
      setStatus({ configured: false, reachable: false, error: e.message });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const ok = status?.reachable || status?.configured || status?.ok;

  return (
    <ChapterShell
      kicker="Network brain"
      title="PacketWolf"
      lede="Cilium / PacketWolf integration health for the console."
      onRefresh={load}
      busy={busy}
    >
      {err && <div className="ops-err" style={{ margin: '0 32px 16px' }}>{err}</div>}
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Connection</div>
          <h2>{ok ? 'Reachable' : 'Not configured'}</h2>
          <p>
            {status?.endpoint ||
              status?.url ||
              status?.message ||
              status?.error ||
              'Set VEYRON_PACKETWOLF_URL (and login credentials) on the API.'}
          </p>
          <div className="rows" style={{ marginTop: 20 }}>
            <button type="button" disabled style={{ cursor: 'default' }}>
              <Status s={ok ? 'Healthy' : 'Unknown'} />
              <b>Health</b>
              <span className="why">{ok ? 'Probe succeeded' : 'Integration absent or unreachable'}</span>
            </button>
            {status?.version && (
              <button type="button" disabled style={{ cursor: 'default' }}>
                <Radar size={14} />
                <b>Version</b>
                <span className="why">{String(status.version)}</span>
              </button>
            )}
          </div>
        </div>
      </section>
    </ChapterShell>
  );
}
