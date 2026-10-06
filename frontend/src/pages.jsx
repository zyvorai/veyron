import { useState, useEffect, useRef, lazy, Suspense } from 'react';
import { useFocusTrap } from './a11y.js';
import {
  ChevronRight, Monitor, Terminal, Archive, Plus, X, Copy, RefreshCw,
  Network, Database, Shield, Settings, Loader2, CheckCircle, Activity,
} from 'lucide-react';
import { Status } from './status.jsx';
import { api } from './api.js';
import { PremiumLoginShell, LoginError } from './PremiumLoginShell.jsx';
import { LogsPanel } from './LogsPanel.jsx';

const VncConsole = lazy(() => import('./VncConsole.jsx').then((m) => ({ default: m.VncConsole })));
import { PageHero } from './PageHero.jsx';
import { FleetHero } from './FleetHero.jsx';
import { Reveal, useSeries, PulseFigure, OsBadge, osInfo, EmptyArt } from './story.jsx';

const HOT_PCT = 90;

function listJoin(items) {
  const xs = [...new Set(items.filter(Boolean))];
  return xs.length < 2 ? xs.join('') : `${xs.slice(0, -1).join(', ')} and ${xs[xs.length - 1]}`;
}

export function Mission({ data, go, onCreate, onConsole, healthOk, podSummary }) {
  const vms = data.vms?.rows || [];
  const hosts = data.hosts?.rows || [];
  const run = vms.filter((v) => v.status === 'Running').length;
  const attn = [
    ...vms.filter((v) => ['Degraded', 'Provisioning', 'Failed'].includes(v.status)).map((v) => ['vms', v]),
    ...hosts.filter((h) => ['Cordoned', 'NotReady'].includes(h.status)).map((h) => ['hosts', h]),
    ...(data.pvcs?.rows || []).filter((p) => p.status === 'Pending').map((p) => ['pvcs', p]),
  ];
  const hot = hosts.filter((h) => (h.cpu ?? 0) >= HOT_PCT || (h.mem ?? 0) >= HOT_PCT);
  for (const h of hot) {
    attn.push(['hosts', { ...h, status: 'Warning', why: `Under pressure: CPU ${h.cpu ?? 0}% · memory ${h.mem ?? 0}%` }]);
  }
  const failedPods = podSummary?.failed ?? 0;
  const attnRows = failedPods
    ? [...attn.slice(0, 7), ['pods', { id: 'failed-pods', name: `${failedPods.toLocaleString()} failed pod${failedPods === 1 ? '' : 's'}`, status: 'Failed', why: 'Evicted or crashed — safe to clean up' }]]
    : attn;
  const why = {
    Degraded: 'Needs attention',
    Provisioning: 'Still provisioning',
    Failed: 'Stopped unexpectedly — open to see events',
    Pending: 'Waiting for capacity',
    Cordoned: 'Cordoned for maintenance',
    NotReady: 'Host not ready',
  };
  const measured = hosts.filter((h) => h.cpu != null);
  const cpuAvg = measured.length ? Math.round(measured.reduce((a, h) => a + h.cpu, 0) / measured.length) : null;
  const memGiB = vms.filter((v) => v.status === 'Running').reduce((a, v) => a + (Number(v.ram) || 0), 0);
  const alertCount = (data.alerts?.rows || []).length + attn.length;
  const series = useSeries({ run, cpu: cpuAvg ?? 0, mem: memGiB, alerts: alertCount });

  const needs = attnRows.length;
  const headline = !healthOk
    ? 'Can’t reach the cluster reliably.'
    : hot.length
      ? `${hot.length === 1 ? 'A host is' : `${hot.length} hosts are`} under pressure.`
      : needs
        ? `${needs} thing${needs === 1 ? '' : 's'} need${needs === 1 ? 's' : ''} a look.`
        : !vms.length
          ? 'Your private cloud is ready.'
          : run === vms.length
            ? 'Everything’s running.'
            : 'Everything’s healthy.';
  const templates = (data.templates?.rows || []).filter((t) => !/^(ubuntu|debian|fedora|centos|almalinux|rocky|opensuse|windows)$/.test(t.name));
  const seenFamily = new Set();
  const featured = templates
    .filter((t) => {
      const fam = osInfo(t.name).family;
      if (seenFamily.has(fam)) return false;
      seenFamily.add(fam);
      return true;
    })
    .slice(0, 12);

  const shortcuts = [
    { I: Monitor, t: 'Create a machine', p: 'Pick an OS, size it, boot it.', tone: 'sky', run: () => { go('vms'); onCreate?.(); } },
    { I: Terminal, t: 'Open a console', p: 'A live screen in your browser.', tone: 'violet', run: () => go('console') },
    { I: Archive, t: 'Protect a machine', p: 'Snapshots, backups and DR.', tone: 'emerald', run: () => go('snapshots') },
    { I: Activity, t: 'Check capacity', p: 'Headroom before you scale.', tone: 'amber', run: () => go('monitoring') },
  ];

  return (
    <div className="mission story">
      <section className="story-hero">
        <div className="story-hero-copy">
          <p className="story-eyebrow">Private cloud command center</p>
          <h1>{headline}</h1>
          <p className="story-lede">
            {run} of {vms.length} machine{vms.length === 1 ? '' : 's'} running on {hosts.length} host
            {hosts.length === 1 ? '' : 's'}.{' '}
            {healthOk ? 'Live from your cluster.' : 'The API is degraded; figures may be stale.'}
          </p>
          <div className="story-cta">
            <button className="pill" onClick={() => { go('vms'); onCreate?.(); }}>
              <Plus size={16} />
              Create a machine
            </button>
            <button className="link on-dark" onClick={() => go('console')}>
              Open a console
              <ChevronRight size={16} />
            </button>
          </div>
        </div>
        <Reveal className="story-hero-art">
          <FleetHero
            hosts={hosts}
            vms={vms}
            onVm={(v) => (v.status === 'Running' && onConsole ? onConsole(v) : go('vms', v.id))}
            onHost={(h) => go('hosts', h.id)}
          />
        </Reveal>
      </section>

      <section className="pulse-band">
        <Reveal className="pulse-grid">
          <PulseFigure label="Machines running" value={run} unit={`/${vms.length}`} series={series.run} tone="sky" />
          <PulseFigure
            label="Host CPU"
            value={cpuAvg ?? '—'}
            unit={cpuAvg == null ? '' : '%'}
            series={series.cpu}
            tone={cpuAvg != null && cpuAvg >= HOT_PCT ? 'amber' : 'violet'}
            sub={cpuAvg == null ? 'No metrics-server data' : `${hosts.length} host${hosts.length === 1 ? '' : 's'}`}
          />
          <PulseFigure label="VM memory" value={memGiB} unit=" GiB" series={series.mem} tone="emerald" sub="Allocated to running machines" />
          <PulseFigure label="Alerts" value={alertCount} series={series.alerts} tone={alertCount ? 'amber' : 'emerald'} />
        </Reveal>
      </section>

      <section className="story-band">
        <Reveal className="apple-chapter">
          <div className="kicker">Fleet</div>
          <h2>{attnRows.length ? 'Needs attention.' : 'All clear.'}</h2>
          <p>Anything that isn’t in the state you asked for shows up here first.</p>
          <div className="rows">
            {attnRows.length === 0 ? (
              <div className="rows-clear">
                <CheckCircle size={20} />
                <span>No degraded, failed or pending resources.</span>
              </div>
            ) : (
              attnRows.slice(0, 8).map(([k, r]) => (
                <button key={`${k}-${r.id}`} onClick={() => go(k, r.why ? undefined : r.id)}>
                  <Status s={r.status} />
                  <b>{r.name}</b>
                  <span className="why">{r.why || why[r.status] || 'Not in the requested state'}</span>
                  <ChevronRight size={16} />
                </button>
              ))
            )}
          </div>
        </Reveal>
      </section>

      <section className="story-band paper">
        <Reveal className="apple-chapter">
          <div className="kicker">Shortcuts</div>
          <h2>Do something great.</h2>
          <p>The things people come here to do, one click away.</p>
          <div className="tile-shelf">
            {shortcuts.map((s, i) => (
              <Reveal key={s.t} as="button" className="tile" data-tone={s.tone} delay={i * 70} onClick={s.run}>
                <span className="tile-ico">
                  <s.I size={22} />
                </span>
                <b>{s.t}</b>
                <p>{s.p}</p>
                <span className="tile-go">
                  Go <ChevronRight size={14} />
                </span>
              </Reveal>
            ))}
          </div>
        </Reveal>
      </section>

      {featured.length > 0 && (
        <section className="story-band">
          <Reveal className="apple-chapter">
            <div className="kicker">Templates</div>
            <h2>Current releases, ready to boot.</h2>
            <p>{listJoin(featured.map((t) => { const o = osInfo(t.name); return `${o.label} ${o.version || ''}`.trim(); }))}, with cloud-init and the guest agent built in.</p>
            <div className="os-shelf">
              {featured.map((t) => {
                const o = osInfo(t.name);
                return (
                  <button key={t.id} className="os-card" onClick={() => { go('vms'); onCreate?.(t.name); }}>
                    <OsBadge name={t.name} size={44} />
                    <b>{o.label}</b>
                    <small>{o.version}</small>
                  </button>
                );
              })}
            </div>
            <button className="link" onClick={() => go('templates')}>
              All templates
              <ChevronRight size={16} />
            </button>
          </Reveal>
        </section>
      )}

      {hosts.length > 0 && (
        <section className="story-band paper">
          <Reveal className="apple-chapter">
            <div className="kicker">Capacity</div>
            <h2>Headroom, per host.</h2>
            <p>CPU and memory pressure across the nodes running your machines.</p>
            <div className="cap-list">
              {hosts.slice(0, 8).map((h) => (
                <button key={h.id} className="cap-row" onClick={() => go('hosts', h.id)}>
                  <span className="cap-name">
                    <b>{h.name}</b>
                    <Status s={h.status} />
                  </span>
                  <span className="cap-bar" title={`CPU ${h.cpu}%`}>
                    <i style={{ width: `${Math.min(100, Number(h.cpu) || 0)}%` }} data-hot={Number(h.cpu) > 85 || undefined} />
                  </span>
                  <span className="cap-bar mem" title={`Memory ${h.mem}%`}>
                    <i style={{ width: `${Math.min(100, Number(h.mem) || 0)}%` }} data-hot={Number(h.mem) > 85 || undefined} />
                  </span>
                  <span className="cap-num">
                    {h.cpu == null ? '—' : `${h.cpu}%`} · {h.mem == null ? '—' : `${h.mem}%`}
                  </span>
                </button>
              ))}
            </div>
          </Reveal>
        </section>
      )}

      <footer className="story-foot">
        <span>Veyron · Zyvor AI Labs</span>
        <span>
          Press <kbd>⌘K</kbd> to jump anywhere
        </span>
      </footer>
    </div>
  );
}

export function ConsoleHub({ vms, onOpen, onCreate }) {
  const live = (vms || []).filter((v) => v.status === 'Running');
  return (
    <div className="hub-page story">
      <section className="story-band">
        <Reveal className="apple-chapter">
          <div className="kicker">Display</div>
          <h2>Consoles.</h2>
          <p>Open a live screen on any running guest, right in the browser.</p>
        </Reveal>
      </section>
      {live.length === 0 ? (
        <div className="empty" style={{ minHeight: 320 }}>
          <div>
            <EmptyArt tone="violet" />
            <b>No running guests</b>
            <p className="lede">Start a machine, then open its console from here.</p>
            <div className="acts">
              <button className="pill" onClick={() => onCreate?.()}>
                Create a machine
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        </div>
      ) : (
        <div className="screen-grid">
          {live.map((v, i) => (
            <Reveal key={v.id} as="button" className="screen-card" delay={Math.min(i, 8) * 50} onClick={() => onOpen(v)}>
              <span className="screen">
                <span className="screen-glow" />
                <OsBadge name={v.os} size={46} />
                <span className="screen-play">
                  <Terminal size={14} /> Open console
                </span>
              </span>
              <span className="screen-meta">
                <b>{v.name}</b>
                <small>
                  {[v.os && v.os !== '—' ? v.os : null, v.ip && v.ip !== '—' ? v.ip : null].filter(Boolean).join(' · ') || v.status}
                </small>
              </span>
            </Reveal>
          ))}
        </div>
      )}
    </div>
  );
}

function headroomLine(h) {
  const part = (r, label) =>
    r && typeof r.headroom === 'number' ? `${label} ${r.headroom.toLocaleString()} of ${r.total.toLocaleString()} ${r.unit} free` : null;
  return (
    [
      part(h.cpu, 'CPU'),
      part(h.memory, 'memory'),
      typeof h.nodes === 'number' && `${h.nodes} node${h.nodes === 1 ? '' : 's'}`,
      typeof h.additional_avg_vms === 'number' && `room for about ${h.additional_avg_vms} more average VMs`,
    ]
      .filter(Boolean)
      .join(' · ') || 'Not available'
  );
}

export function SettingsPage({ theme, setTheme }) {
  const [policy, setPolicy] = useState({ enabled: false, cooldown_secs: 600 });
  const [policyBusy, setPolicyBusy] = useState(false);
  const [integrations, setIntegrations] = useState(null);
  const [about, setAbout] = useState({ versions: null, caps: null });
  const [headroom, setHeadroom] = useState(null);
  const [err, setErr] = useState('');

  useEffect(() => {
    let cancelled = false;
    const set = (fn) => (v) => {
      if (!cancelled) fn(v);
    };
    api
      .getSelfHealingPolicy()
      .catch(() => ({ enabled: false }))
      .then(set((p) => setPolicy({ enabled: !!p.enabled, namespaces: p.namespaces || 'all', cooldown_secs: p.cooldown_secs || 600 })));
    api
      .integrationsStatus()
      .catch(() => ({ integrations: [] }))
      .then(set((integ) => setIntegrations(Array.isArray(integ?.integrations) ? integ.integrations : Array.isArray(integ) ? integ : [])));
    api.platformVersions().catch(() => null).then(set((versions) => setAbout((a) => ({ ...a, versions }))));
    api.platformCapabilities().catch(() => null).then(set((caps) => setAbout((a) => ({ ...a, caps }))));
    api.capacityHeadroom().catch(() => null).then(set(setHeadroom));
    return () => {
      cancelled = true;
    };
  }, []);

  const savePolicy = async (next) => {
    setPolicyBusy(true);
    setErr('');
    try {
      await api.setSelfHealingPolicy(next);
      setPolicy(next);
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setPolicyBusy(false);
    }
  };

  const findInteg = (ids) =>
    (integrations || []).find((i) => ids.includes(String(i.id || '').toLowerCase()) || ids.includes(String(i.name || '').toLowerCase()));

  const netra = findInteg(['netra']);
  const paqtra = findInteg(['paqtra']);
  const atlas = findInteg(['atlas']);
  const oidc = findInteg(['oidc', 'sso']);

  const integRow = (n, d, I, item) => {
    const detail = item?.configured ? item.endpoint || 'Configured' : d;
    const state = !integrations
      ? <span className="dim">Checking…</span>
      : !item?.configured
        ? <span className="dim">Not configured</span>
        : item.probe === 'failed'
          ? <Status s="Failed" />
          : <Status s="Healthy" />;
    return (
      <div className="srow2" key={n}>
        <I size={16} style={{ color: 'var(--accent)' }} />
        <div>
          <b>{n}</b>
          <small>{detail}</small>
        </div>
        <span className="r">{state}</span>
      </div>
    );
  };

  const kvVersion = about.versions;
  const verLabel = (v, fallback) => {
    if (v == null) return fallback;
    if (typeof v === 'string') return v;
    if (typeof v === 'object') {
      return (
        v.observedVersion ||
        v.operatorVersion ||
        v.targetVersion ||
        v.version ||
        v.phase ||
        null
      );
    }
    return String(v);
  };
  const versionLine = kvVersion
    ? [
        verLabel(kvVersion.kairon || kvVersion.kairon_version, null) && `Kairon ${verLabel(kvVersion.kairon || kvVersion.kairon_version, null)}`,
        verLabel(kvVersion.veyron || kvVersion.version, null),
        verLabel(kvVersion.kubevirt, null) && `KubeVirt ${verLabel(kvVersion.kubevirt, null)}`,
        verLabel(kvVersion.cdi, null) && `CDI ${verLabel(kvVersion.cdi, null)}`,
      ]
        .filter(Boolean)
        .join(' · ') || 'Platform versions available'
    : 'Loading…';

  return (
    <div className="settings">
      {err && <div className="toast err" style={{ position: 'relative', top: 0, marginBottom: 12 }}>{err}</div>}
      <PageHero kicker="This console" title="Settings" lede="Theme, self-healing, and how this console connects to the cluster." />
      <div className="sgroup">
        <h3>Appearance</h3>
        <div className="sbox">
          <div className="srow2">
            <div>
              <b>Theme</b>
              <small>Saved on this device.</small>
            </div>
            <div className="seg r">
              {['light', 'dark'].map((x) => (
                <button key={x} className="tb" aria-pressed={theme === x} onClick={() => setTheme(x)}>
                  {x}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
      <div className="sgroup">
        <h3>Cluster</h3>
        <div className="sbox">
          <div className="srow2">
            <div>
              <b>Auto-heal degraded machines</b>
              <small>Restart Failed or Error machines (self-healing policy).</small>
            </div>
            <button
              className="sw r"
              role="switch"
              aria-checked={policy.enabled}
              disabled={policyBusy}
              onClick={() => savePolicy({ ...policy, enabled: !policy.enabled })}
            >
              <i />
            </button>
          </div>
          <div className="srow2">
            <div>
              <b>Run self-healing now</b>
              <small>Dry-run reports unhealthy machines; heal restarts them.</small>
            </div>
            <div className="r" style={{ display: 'flex', gap: 6 }}>
              <button
                className="btn sm secondary"
                disabled={policyBusy}
                onClick={async () => {
                  setPolicyBusy(true);
                  setErr('');
                  try {
                    const r = await api.runSelfHealing(false);
                    setErr('');
                    alert(`Dry-run: unhealthy=${r?.unhealthy ?? '?'} (heal=${r?.healed ?? 0})`);
                  } catch (e) {
                    setErr(e.message || String(e));
                  } finally {
                    setPolicyBusy(false);
                  }
                }}
              >
                Dry-run
              </button>
              <button
                className="btn sm warn"
                disabled={policyBusy}
                onClick={async () => {
                  if (!window.confirm('Heal unhealthy VMs now?')) return;
                  setPolicyBusy(true);
                  setErr('');
                  try {
                    const r = await api.runSelfHealing(true);
                    alert(`Healed ${r?.healed ?? 0} of ${r?.unhealthy ?? '?'}`);
                  } catch (e) {
                    setErr(e.message || String(e));
                  } finally {
                    setPolicyBusy(false);
                  }
                }}
              >
                Heal
              </button>
            </div>
          </div>
        </div>
      </div>
      <div className="sgroup">
        <h3>Integrations</h3>
        <div className="sbox">
          {integRow('Netra', 'Auto-detected in netra-system, or set VEYRON_NETRA_URL', Network, netra)}
          {integRow('Paqtra', 'Auto-detected in the paqtra namespace, or set VEYRON_PAQTRA_URL', Network, paqtra)}
          {integRow('Atlas storage', 'Configure via VEYRON_ATLAS_URL', Database, atlas)}
          {integRow('Single sign-on', 'Configure via VEYRON_OIDC_*', Shield, oidc)}
        </div>
      </div>
      <div className="sgroup">
        <h3>About</h3>
        <div className="sbox">
          <div className="srow2">
            <Settings size={16} style={{ color: 'var(--accent)' }} />
            <div>
              <b>Veyron Console</b>
              <small>{versionLine}</small>
            </div>
          </div>
          {headroom && (
            <div className="srow2">
              <div>
                <b>Capacity headroom</b>
                <small>{headroomLine(headroom)}</small>
              </div>
            </div>
          )}
          {about.caps && (
            <div className="srow2">
              <div>
                <b>Capabilities</b>
                <small>
                  {[
                    about.caps.live_migration && 'live migration',
                    about.caps.machine_backups && 'machine backups',
                    about.caps.machine_fork && 'VM fork',
                    about.caps.gpu_passthrough && 'GPU (DRA)',
                    about.caps.persistent_tpm_efi && 'persistent TPM/EFI',
                  ]
                    .filter(Boolean)
                    .join(' · ') || 'See /api/v1/platform/capabilities'}
                </small>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function ConsoleSheet({ vm, mode = 'console', onClose }) {
  const [logs, setLogs] = useState('');
  const [loading, setLoading] = useState(mode === 'logs');
  const [error, setError] = useState('');

  useEffect(() => {
    if (mode !== 'logs') return;
    let cancelled = false;
    setLoading(true);
    api
      .podLogs(vm.name, vm.ns || 'default')
      .then((t) => {
        if (!cancelled) setLogs(t);
      })
      .catch((e) => {
        if (!cancelled) setError(e.message || String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [mode, vm.name, vm.ns]);
  const sheetRef = useRef(null);
  useFocusTrap(sheetRef);

  return (
    <div className="scrim" onClick={onClose}>
      <div
        ref={sheetRef}
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-label={`${vm.name} ${mode === 'logs' ? 'logs' : 'console'}`}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        style={{ width: mode === 'console' ? 'min(960px, 96%)' : undefined }}
      >
        <div className="sheet-h">
          <Terminal size={15} />
          {vm.name} · {mode === 'logs' ? 'logs' : 'console'}
          <button className="tb x" onClick={onClose} aria-label="Close">
            <X size={15} />
          </button>
        </div>
        {mode === 'logs' ? (
          <LogsPanel text={logs} loading={loading} error={error} />
        ) : (
          <Suspense fallback={<div className="console"><div className="console-logs">Loading console…</div></div>}>
            <VncConsole vm={vm} />
          </Suspense>
        )}
        <div className="sheet-f">
          <button className="btn" onClick={() => navigator.clipboard?.writeText(vm.name)}>
            <Copy size={13} />
            Copy name
          </button>
          <button className="btn primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

const SIZES = [
  { id: 'S', label: 'Small', cpus: 2, memory: '4Gi', note: '2 vCPU · 4 GiB' },
  { id: 'M', label: 'Medium', cpus: 4, memory: '8Gi', note: '4 vCPU · 8 GiB' },
  { id: 'L', label: 'Large', cpus: 8, memory: '16Gi', note: '8 vCPU · 16 GiB' },
];
const FAMILY_ALIASES = /^(ubuntu|debian|fedora|centos|almalinux|rocky|opensuse|windows)$/;

export function NewSheet({ res, templates, storageClasses, initialTemplate, onClose, onCreate }) {
  const isVm = res.kind === 'VirtualMachine';
  const osTemplates = (templates || []).filter((t) => !FAMILY_ALIASES.test(t.name));
  const pickable = osTemplates.length ? osTemplates : templates || [];
  const [name, setName] = useState('');
  const [template, setTemplate] = useState(initialTemplate || pickable[0]?.name || '');
  const [sizeId, setSizeId] = useState('M');
  const [cls, setCls] = useState(storageClasses?.[0]?.name || '');
  const [size, setSize] = useState(res.kind === 'Image' ? '20Gi' : '10Gi');
  const [url, setUrl] = useState('');
  const [cidr, setCidr] = useState('10.244.100.0/24');
  const preset = SIZES.find((s) => s.id === sizeId);
  const o = osInfo(template);
  const [suffix] = useState(() => Math.random().toString(36).slice(2, 6));
  const suggested = `${o.family}-${suffix}`;
  const finalName = name || (isVm ? suggested : '');

  const canCreate = !!finalName && (res.kind !== 'Image' || !!url.trim()) && (!isVm || !!template);
  const sheetRef = useRef(null);
  useFocusTrap(sheetRef);
  const submit = () =>
    onCreate({
      name: finalName,
      template,
      cls,
      size,
      url,
      cidr,
      ...(isVm ? { cpus: preset.cpus, memory: preset.memory } : {}),
    });

  return (
    <div className="scrim" onClick={onClose}>
      <div
        ref={sheetRef}
        className={`sheet${isVm ? ' sheet--wide' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-label={isVm ? 'Create a machine' : `New ${res.kind}`}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="sheet-h">
          <Plus size={15} />
          {isVm ? 'Create a machine' : `New ${res.kind}`}
          <button className="tb x" onClick={onClose} aria-label="Close">
            <X size={15} />
          </button>
        </div>

        {isVm ? (
          <div className="create">
            <div className="create-main">
              <h3 className="create-step">
                <span>1</span> Choose an operating system
              </h3>
              <div className="os-pick">
                {pickable.length === 0 && <p className="ops-hint">No templates available.</p>}
                {pickable.map((t) => {
                  const info = osInfo(t.name);
                  return (
                    <button
                      key={t.id || t.name}
                      type="button"
                      className="os-opt"
                      aria-pressed={template === t.name}
                      onClick={() => setTemplate(t.name)}
                    >
                      <OsBadge name={t.name} size={34} />
                      <span>
                        <b>{info.label}</b>
                        <small>{info.version}</small>
                      </span>
                    </button>
                  );
                })}
              </div>

              <h3 className="create-step">
                <span>2</span> Pick a size
              </h3>
              <div className="size-pick">
                {SIZES.map((s) => (
                  <button key={s.id} type="button" className="size-opt" aria-pressed={sizeId === s.id} onClick={() => setSizeId(s.id)}>
                    <b>{s.id}</b>
                    <span>{s.label}</span>
                    <small>{s.note}</small>
                  </button>
                ))}
              </div>

              <h3 className="create-step">
                <span>3</span> Name it
              </h3>
              <input
                className="create-name"
                value={name}
                autoFocus
                onChange={(e) => setName(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '-'))}
                onKeyDown={(e) => e.key === 'Enter' && canCreate && submit()}
                placeholder={suggested}
              />
            </div>
            <aside className="create-summary">
              <OsBadge name={template} size={64} />
              <h4>{finalName}</h4>
              <p>
                {o.label} {o.version}
              </p>
              <dl>
                <dt>Size</dt>
                <dd>{preset.note}</dd>
                <dt>Template</dt>
                <dd className="mono">{template || '—'}</dd>
                <dt>Starts</dt>
                <dd>Immediately</dd>
              </dl>
              <button className="btn primary create-go" disabled={!canCreate} onClick={submit}>
                Create machine
              </button>
            </aside>
          </div>
        ) : (
          <>
            <div style={{ padding: '8px 0' }}>
              <div className="field">
                <label>Name</label>
                <input value={name} onChange={(e) => setName(e.target.value)} placeholder={`e.g. ${res.rows[0]?.name || 'name'}`} />
              </div>
              {(res.kind === 'PersistentVolumeClaim' || res.kind === 'Image') && (
                <>
                  <div className="field">
                    <label>Class</label>
                    <select value={cls} onChange={(e) => setCls(e.target.value)}>
                      <option value="">(default)</option>
                      {(storageClasses || []).map((p) => (
                        <option key={p.id || p.name} value={p.name}>
                          {p.cls || p.name}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="field">
                    <label>Size</label>
                    <input value={size} onChange={(e) => setSize(e.target.value)} />
                  </div>
                </>
              )}
              {res.kind === 'Image' && (
                <div className="field">
                  <label>URL</label>
                  <input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://…/image.qcow2" />
                </div>
              )}
              {res.kind === 'Network' && (
                <div className="field">
                  <label>CIDR</label>
                  <input value={cidr} onChange={(e) => setCidr(e.target.value)} placeholder="10.244.100.0/24" />
                </div>
              )}
            </div>
            <div className="sheet-f">
              <button className="btn" onClick={onClose}>
                Cancel
              </button>
              <button className="btn primary" disabled={!canCreate} onClick={submit}>
                Create
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export function Login({ onSubmit, error }) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [localError, setLocalError] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const shown = localError || error;

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!username.trim() || !password) {
      setLocalError('Wrong username or password.');
      return;
    }
    setLocalError('');
    setSubmitting(true);
    try {
      await onSubmit(username.trim(), password);
    } catch {
      /* parent sets error */
    } finally {
      setSubmitting(false);
    }
  };

  const clearError = () => {
    if (localError) setLocalError('');
  };

  return (
    <PremiumLoginShell
      productName="Veyron"
      eyebrow="Veyron · Zyvor"
      heroTitle="Real VMs. One console."
      heroLede="Fleet, storage, snapshots and day-2 operations for your virtual machines."
    >
      <form className="card login-card" onSubmit={handleSubmit} noValidate>
        <h1>Sign in.</h1>
        <label className="tokenbox">
          Username
          <input
            value={username}
            onChange={(e) => {
              setUsername(e.target.value);
              clearError();
            }}
            autoFocus
            autoComplete="username"
            disabled={submitting}
            aria-invalid={Boolean(shown)}
          />
        </label>
        <label className="tokenbox">
          Password
          <input
            type="password"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              clearError();
            }}
            autoComplete="current-password"
            disabled={submitting}
            aria-invalid={Boolean(shown)}
          />
        </label>
        <LoginError message={shown} />
        <button type="submit" className="primary" disabled={submitting}>
          {submitting ? 'Signing in…' : 'Sign in'}
        </button>
      </form>
    </PremiumLoginShell>
  );
}
