import { useEffect, useState } from 'react';
import { Activity, Map, Cloud, Radar, Waypoints, RefreshCw, ChevronRight } from 'lucide-react';
import { api } from './api.js';
import { Status } from './status.jsx';
import { PageHero } from './PageHero.jsx';

function ChapterShell({ kicker, title, lede, children, onRefresh, busy }) {
  return (
    <div className="chapter-page">
      <PageHero kicker={kicker} title={title} lede={lede} onRefresh={onRefresh} busy={busy} />
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
    [
      'CPU free',
      headroom?.cpu?.headroom != null ? `${headroom.cpu.headroom} ${headroom.cpu.unit || ''}`.trim() : '—',
    ],
    [
      'Memory free',
      headroom?.memory?.headroom != null
        ? `${headroom.memory.headroom} ${headroom.memory.unit || ''}`.trim()
        : '—',
    ],
    ['Nodes', headroom?.nodes ?? caps?.nodes?.count ?? '—'],
    ['Live migration', caps?.live_migration ? 'Available' : caps ? 'Not available' : '—'],
  ];

  return (
    <ChapterShell
      kicker="Platform"
      title="Monitoring"
      lede="Capacity headroom and installed platform versions."
      onRefresh={load}
      busy={busy}
    >
      {err && <div className="ops-err page-err">{err}</div>}
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
              Object.entries(versions).slice(0, 12).map(([k, v]) => {
                let display;
                if (v && typeof v === 'object') {
                  const ver = v.operatorVersion || v.targetVersion || v.observedVersion;
                  display = ver ? `${ver}${v.phase ? ` · ${v.phase}` : ''}` : JSON.stringify(v);
                } else {
                  display = String(v);
                }
                return (
                  <button type="button" key={k} disabled>
                    <b>{k}</b>
                    <span className="why mono">{display}</span>
                  </button>
                );
              })
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
      {err && <div className="ops-err page-err">{err}</div>}
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Nodes · {Array.isArray(nodes) ? nodes.length : 0}</div>
          <div className="rows">
            {(Array.isArray(nodes) ? nodes : []).slice(0, 40).map((n, i) => {
              const label = n.name || n.id || n.node || `node-${i}`;
              const kind = n.kind || n.type || n.role || '—';
              return (
                <button type="button" key={label} disabled>
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
              <button type="button" key={i} disabled>
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
      {err && <div className="ops-err page-err">{err}</div>}
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

const fmtCount = (n) => (typeof n === 'number' ? n.toLocaleString() : '—');

function connectionHeadline(status) {
  if (!status) return 'Checking…';
  if (status.api_authorized) return 'Connected';
  if (status.reachable) return 'Unauthorized';
  return status.configured ? 'Unreachable' : 'Not installed';
}

export function NetraPage() {
  const [status, setStatus] = useState(null);
  const [flows, setFlows] = useState(null);
  const [vms, setVms] = useState([]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const load = async () => {
    setBusy(true);
    setErr('');
    try {
      const st = await api.netraStatus();
      setStatus(st);
      if (st?.api_authorized) {
        const [summary, vmList] = await Promise.allSettled([api.netraFlowSummary(), api.netraVms()]);
        setFlows(summary.status === 'fulfilled' ? summary.value : null);
        setVms(vmList.status === 'fulfilled' ? vmList.value : []);
        const failed = [summary, vmList].find((r) => r.status === 'rejected');
        if (failed) setErr(failed.reason?.message || String(failed.reason));
      } else {
        setFlows(null);
        setVms([]);
      }
    } catch (e) {
      setErr(e.message || String(e));
      setStatus({ configured: false, reachable: false });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const ok = Boolean(status?.api_authorized);
  const headline = connectionHeadline(status);
  const verdicts = Object.entries(flows?.verdicts || {}).sort((x, y) => y[1] - x[1]);
  const protocols = Object.entries(flows?.protocols || {}).sort((x, y) => y[1] - x[1]);
  const drops = flows?.dropReasons || [];
  const destinations = flows?.topDestinations || [];

  return (
    <ChapterShell
      kicker="Network"
      title="Netra"
      lede="eBPF network observability for VM and pod traffic: flows, drops and per-VM lockdown."
      onRefresh={load}
      busy={busy}
    >
      {err && <div className="ops-err page-err">{err}</div>}
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Connection</div>
          <h2>{headline}</h2>
          <p>
            {!status
              ? 'Looking for Netra in this cluster.'
              : status.message ||
                (status.base_url
                  ? `${status.base_url}${status.source === 'discovered' ? ' (found in netra-system)' : ''}`
                  : 'Install Netra in netra-system or set VEYRON_NETRA_URL on the API.')}
          </p>
          <div className="rows" style={{ marginTop: 20 }}>
            <button type="button" disabled>
              <Status s={ok ? 'Healthy' : 'Unknown'} />
              <b>Health</b>
              <span className="why">{ok ? 'Controller reachable and API key accepted' : 'No authorized connection'}</span>
            </button>
            {status?.version && (
              <button type="button" disabled>
                <Radar size={14} />
                <b>Version</b>
                <span className="why">
                  {status.version}
                  {status.datapath ? ` · ${status.datapath}` : ''}
                  {status.mode ? ` · ${status.mode} mode` : ''}
                </span>
              </button>
            )}
            {typeof status?.agents === 'number' && (
              <button type="button" disabled>
                <Status s={status.stale_agents ? 'Warning' : 'Healthy'} />
                <b>Agents</b>
                <span className="why">
                  {status.agents} reporting{status.stale_agents ? `, ${status.stale_agents} stale` : ''}
                </span>
              </button>
            )}
            {typeof status?.flows_per_second === 'number' && (
              <button type="button" disabled>
                <Activity size={14} />
                <b>Flow rate</b>
                <span className="why">{status.flows_per_second.toFixed(1)} flows/s</span>
              </button>
            )}
            {status?.external_url && (
              <a className="row-link" href={status.external_url} target="_blank" rel="noreferrer">
                <ChevronRight size={14} />
                <b>Open Netra</b>
                <span className="why">{status.external_url}</span>
              </a>
            )}
          </div>
        </div>
      </section>
      {ok && (
        <section className="apple-band">
          <div className="apple-chapter">
            <div className="kicker">Traffic · {flows ? `last ${fmtCount(flows.total)} flows` : 'loading'}</div>
            <div className="rows">
              {verdicts.map(([v, n]) => (
                <button type="button" key={`v-${v}`} disabled>
                  <Status s={v === 'DROPPED' || v === 'ERROR' ? 'Failed' : 'Healthy'} />
                  <b>{v}</b>
                  <span className="why">{fmtCount(n)}</span>
                </button>
              ))}
              {protocols.map(([p, n]) => (
                <button type="button" key={`p-${p}`} disabled>
                  <b>{p}</b>
                  <span className="why">{fmtCount(n)}</span>
                </button>
              ))}
              {!verdicts.length && (
                <button type="button" disabled>
                  <b>{flows ? 'No flows sampled' : busy ? 'Sampling flows…' : 'Flow summary unavailable'}</b>
                  <span className="why">
                    {flows ? 'Netra returned an empty window.' : busy ? 'Asking Netra for recent flows.' : 'See the error above.'}
                  </span>
                </button>
              )}
            </div>
            {drops.length > 0 && (
              <>
                <div className="kicker" style={{ marginTop: 28 }}>Drop reasons</div>
                <div className="rows">
                  {drops.map((d) => (
                    <button type="button" key={d.name} disabled>
                      <Status s="Failed" />
                      <b>{d.name}</b>
                      <span className="why">{fmtCount(d.count)}</span>
                    </button>
                  ))}
                </div>
              </>
            )}
            {destinations.length > 0 && (
              <>
                <div className="kicker" style={{ marginTop: 28 }}>Top destinations</div>
                <div className="rows">
                  {destinations.map((d) => (
                    <button type="button" key={d.name} disabled>
                      <b className="mono">{d.name}</b>
                      <span className="why">{fmtCount(d.count)} flows</span>
                    </button>
                  ))}
                </div>
              </>
            )}
            <div className="kicker" style={{ marginTop: 28 }}>VM network view · {vms.length}</div>
            <div className="rows">
              {vms.map((vm) => (
                <button type="button" key={`${vm.namespace}/${vm.name}`} disabled>
                  <Status s={vm.lockedDown ? 'Warning' : vm.running ? 'Running' : 'Stopped'} />
                  <b>
                    {vm.namespace}/{vm.name}
                  </b>
                  <span className="why">
                    {[vm.podIP, vm.node, vm.lockedDown ? 'locked down' : null].filter(Boolean).join(' · ') || vm.phase || '—'}
                  </span>
                </button>
              ))}
              {!vms.length && (
                <button type="button" disabled>
                  <b>No VMs seen by Netra</b>
                  <span className="why">Netra lists running virt-launcher pods.</span>
                </button>
              )}
            </div>
          </div>
        </section>
      )}
    </ChapterShell>
  );
}

const pct = (n) => (typeof n === 'number' ? `${Math.round(n)}%` : '—');
const endpoint = (e) => (e?.pod ? `${e.namespace}/${e.pod}` : e?.ip || '—');

export function PaqtraPage() {
  const [status, setStatus] = useState(null);
  const [flows, setFlows] = useState([]);
  const [drops, setDrops] = useState([]);
  const [posture, setPosture] = useState(null);
  const [onlyDropped, setOnlyDropped] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');

  const load = async (dropped) => {
    setBusy(true);
    setErr('');
    try {
      const st = await api.paqtraStatus();
      setStatus(st);
      if (st?.api_authorized) {
        const results = await Promise.allSettled([
          api.paqtraFlows(dropped ? 'DROPPED' : ''),
          api.paqtraDrops(),
          api.paqtraPosture(),
        ]);
        const [f, d, p] = results;
        setFlows(f.status === 'fulfilled' ? f.value : []);
        setDrops(d.status === 'fulfilled' ? d.value : []);
        setPosture(p.status === 'fulfilled' ? p.value : null);
        const failed = results.find((r) => r.status === 'rejected');
        if (failed) setErr(failed.reason?.message || String(failed.reason));
      } else {
        setFlows([]);
        setDrops([]);
        setPosture(null);
      }
    } catch (e) {
      setErr(e.message || String(e));
      setStatus({ configured: false, reachable: false });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load(false);
  }, []);

  const toggleDropped = () => {
    const next = !onlyDropped;
    setOnlyDropped(next);
    load(next);
  };

  const ok = Boolean(status?.api_authorized);
  const headline = connectionHeadline(status);
  const score = posture?.posture;
  const unprotected = posture?.breakdown?.namespaces_without_policies || [];

  return (
    <ChapterShell
      kicker="Network"
      title="Paqtra"
      lede="Cilium-native flow history from Hubble: verdicts, explained drops and network policy coverage."
      onRefresh={() => load(onlyDropped)}
      busy={busy}
    >
      {err && <div className="ops-err page-err">{err}</div>}
      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Connection</div>
          <h2>{headline}</h2>
          <p>
            {!status
              ? 'Looking for Paqtra in this cluster.'
              : status.message ||
                (status.base_url
                  ? `${status.base_url}${status.source === 'discovered' ? ' (found in the paqtra namespace)' : ''}`
                  : 'Install Paqtra or set VEYRON_PAQTRA_URL on the API.')}
          </p>
          <div className="rows" style={{ marginTop: 20 }}>
            <button type="button" disabled>
              <Status s={ok ? (status.health === 'healthy' ? 'Healthy' : 'Warning') : 'Unknown'} />
              <b>Health</b>
              <span className="why">
                {ok
                  ? `${status.health || 'unknown'}${typeof status.health_score === 'number' ? ` · score ${status.health_score}` : ''}`
                  : 'No authorized connection'}
              </span>
            </button>
            {status?.version && (
              <button type="button" disabled>
                <Waypoints size={14} />
                <b>Version</b>
                <span className="why">
                  {status.version}
                  {status.credentials_source === 'secret' ? ' · signed in with the install secret' : ''}
                </span>
              </button>
            )}
            {typeof status?.hubble_connected === 'boolean' && (
              <button type="button" disabled>
                <Status s={status.hubble_connected ? 'Healthy' : 'Failed'} />
                <b>Hubble ingest</b>
                <span className="why">
                  {status.hubble_connected ? 'Streaming' : 'Disconnected'}
                  {typeof status.flows_per_second === 'number' ? ` · ${status.flows_per_second.toFixed(1)} flows/s` : ''}
                  {typeof status.indexed_flows === 'number' ? ` · ${fmtCount(status.indexed_flows)} stored` : ''}
                  {status.retention_days ? ` · ${status.retention_days}d retention` : ''}
                </span>
              </button>
            )}
            {typeof status?.cilium_agents_total === 'number' && (
              <button type="button" disabled>
                <Status s={status.cilium_agents_ready === status.cilium_agents_total ? 'Healthy' : 'Warning'} />
                <b>Cilium agents</b>
                <span className="why">
                  {status.cilium_agents_ready} of {status.cilium_agents_total} ready
                </span>
              </button>
            )}
            {status?.external_url && (
              <a className="row-link" href={status.external_url} target="_blank" rel="noreferrer">
                <ChevronRight size={14} />
                <b>Open Paqtra</b>
                <span className="why">{status.external_url}</span>
              </a>
            )}
          </div>
        </div>
      </section>
      {ok && (
        <section className="apple-band">
          <div className="apple-chapter">
            {score && (
              <>
                <div className="kicker">Policy posture · score {pct(score.score)}</div>
                <div className="rows">
                  <button type="button" disabled>
                    <Status s={score.policy_coverage >= 80 ? 'Healthy' : 'Warning'} />
                    <b>Policy coverage</b>
                    <span className="why">{pct(score.policy_coverage)} of namespaces</span>
                  </button>
                  <button type="button" disabled>
                    <Status s={score.namespace_isolation >= 80 ? 'Healthy' : 'Warning'} />
                    <b>Namespace isolation</b>
                    <span className="why">{pct(score.namespace_isolation)}</span>
                  </button>
                  <button type="button" disabled>
                    <Status s={score.encryption_coverage >= 80 ? 'Healthy' : 'Warning'} />
                    <b>Encryption</b>
                    <span className="why">{pct(score.encryption_coverage)} of traffic</span>
                  </button>
                  {unprotected.length > 0 && (
                    <button type="button" disabled>
                      <Status s="Warning" />
                      <b>No network policy</b>
                      <span className="why">
                        {unprotected.length} namespaces: {unprotected.slice(0, 6).join(', ')}
                        {unprotected.length > 6 ? '…' : ''}
                      </span>
                    </button>
                  )}
                </div>
              </>
            )}
            <div className="kicker" style={{ marginTop: score ? 28 : 0 }}>Explained drops · {drops.length}</div>
            <div className="rows">
              {drops.map((d) => (
                <button type="button" key={d.id} disabled>
                  <Status s="Failed" />
                  <b>
                    {d.source} → {d.destination}
                  </b>
                  <span className="why">
                    {[d.drop_reason, d.root_cause, d.remediation, d.count > 1 ? `${fmtCount(d.count)}×` : null]
                      .filter(Boolean)
                      .join(' · ')}
                  </span>
                </button>
              ))}
              {!drops.length && (
                <button type="button" disabled>
                  <Status s="Healthy" />
                  <b>No drops</b>
                  <span className="why">Cilium has not dropped any recent flows.</span>
                </button>
              )}
            </div>
            <div className="kicker" style={{ marginTop: 28 }}>
              Recent flows · {flows.length}
              <button type="button" className="btn sm secondary" style={{ marginLeft: 12 }} onClick={toggleDropped} disabled={busy}>
                {onlyDropped ? 'Show all' : 'Only dropped'}
              </button>
            </div>
            <div className="rows">
              {flows.map((f) => (
                <button type="button" key={f.id} disabled>
                  <Status s={f.verdict === 'DROPPED' || f.verdict === 'ERROR' ? 'Failed' : 'Healthy'} />
                  <b className="mono">
                    {endpoint(f.source)} → {endpoint(f.destination)}
                  </b>
                  <span className="why">
                    {[f.verdict, f.protocol && f.port ? `${f.protocol}/${f.port}` : f.protocol, f.timestamp && new Date(f.timestamp).toLocaleTimeString()]
                      .filter(Boolean)
                      .join(' · ')}
                  </span>
                </button>
              ))}
              {!flows.length && (
                <button type="button" disabled>
                  <b>No flows</b>
                  <span className="why">{onlyDropped ? 'No dropped flows in the store.' : 'Paqtra returned no flows.'}</span>
                </button>
              )}
            </div>
          </div>
        </section>
      )}
    </ChapterShell>
  );
}
