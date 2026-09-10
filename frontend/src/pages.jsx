import { useState, useEffect } from 'react';
import {
  ChevronRight, Monitor, Terminal, Archive, Plus, X, Copy, RefreshCw,
  Network, Database, Shield, Settings, User, Lock, Eye, EyeOff, ArrowRight, ChevronLeft, Loader2,
  CheckCircle,
} from 'lucide-react';
import { Status } from './status.jsx';
import { api, getSavedUsername, setSavedUsername } from './api.js';
import {
  PremiumLoginShell,
  LoginField,
  LoginSubmit,
  LoginRemember,
  LoginError,
} from './PremiumLoginShell.jsx';
import { VncConsole, LogsPanel } from './VncConsole.jsx';
import { PageHero } from './PageHero.jsx';

export function Mission({ data, go, hostCount, onCreate }) {
  const vms = data.vms?.rows || [];
  const run = vms.filter((v) => v.status === 'Running').length;
  const attn = [
    ...vms.filter((v) => ['Degraded', 'Provisioning', 'Failed'].includes(v.status)).map((v) => ['vms', v]),
    ...(data.pods?.rows || []).filter((p) => p.status === 'Failed').map((p) => ['pods', p]),
    ...(data.pvcs?.rows || []).filter((p) => p.status === 'Pending').map((p) => ['pvcs', p]),
  ];
  const word = attn.some(([, r]) => ['Degraded', 'Failed'].includes(r.status)) ? 'Attention' : 'Operational';
  const tone = word === 'Attention' ? 'amber' : 'emerald';
  const why = {
    Degraded: 'Needs attention',
    Provisioning: 'Still provisioning',
    Failed: 'Failed',
    Pending: 'Waiting for capacity',
  };
  const hosts = hostCount || (data.hosts?.rows || []).length || 0;
  const memGiB = vms.filter((v) => v.status === 'Running').reduce((a, v) => a + (Number(v.ram) || 0), 0);

  return (
    <div className="mission" data-tone={tone}>
      <section className="apple-band-void">
        <div className="mhero">
          <h1 className="brand-word">Veyron</h1>
          <p className={`punch${word === 'Attention' ? ' attn' : ''}`}>{word}.</p>
          <p>
            {run} of {vms.length} machine{vms.length === 1 ? '' : 's'} running on {hosts} host
            {hosts === 1 ? '' : 's'}.{' '}
            {attn.length
              ? `${attn.length} thing${attn.length === 1 ? '' : 's'} need a look.`
              : 'Nothing needs a look.'}
          </p>
          <div className="acts">
            <button
              className="pill"
              onClick={() => {
                go('vms');
                onCreate?.();
              }}
            >
              Create a machine
              <ChevronRight size={16} />
            </button>
            <button className="link" onClick={() => go('vms')}>
              Virtual machines
              <ChevronRight size={15} />
            </button>
            <button className="link" onClick={() => go('hosts')}>
              Hosts
              <ChevronRight size={15} />
            </button>
          </div>
        </div>
      </section>

      <div className="tone-swatch" aria-hidden="true">
        {['sky', 'violet', 'amber', 'rose', 'emerald'].map((t) => (
          <i key={t} data-tone={t} />
        ))}
      </div>

      <section className="apple-band-paper">
        <div className="specs">
          <div>
            <b>
              {run}
              <small>/{vms.length}</small>
            </b>
            <span>VMs running</span>
          </div>
          <div>
            <b>
              {hosts}
              <small>/{hosts || 0}</small>
            </b>
            <span>Hosts ready</span>
          </div>
          <div>
            <b>
              {memGiB}
              <small> GiB</small>
            </b>
            <span>Memory in use</span>
          </div>
          <div>
            <b>{attn.length || 'None'}</b>
            <span>Alerts</span>
          </div>
        </div>
      </section>

      <section className="apple-band">
        <div className="apple-chapter">
          <div className="kicker">Fleet</div>
          <h2>Needs attention</h2>
          <p>Resources that aren&apos;t in the state you asked for.</p>
          <div className="rows">
            {attn.length === 0 ? (
              <button type="button" disabled style={{ opacity: 0.65, cursor: 'default' }}>
                <Status s="Healthy" />
                <b>All clear</b>
                <span className="why">No degraded, failed, or pending items.</span>
              </button>
            ) : (
              attn.map(([k, r]) => (
                <button key={`${k}-${r.id}`} onClick={() => go(k, r.id)}>
                  <Status s={r.status} />
                  <b>{r.name}</b>
                  <span className="why">{why[r.status] || r.status}</span>
                  <ChevronRight size={16} />
                </button>
              ))
            )}
          </div>
        </div>
      </section>

      <section className="apple-band-paper">
        <div className="apple-chapter">
          <div className="kicker">Shortcuts</div>
          <h2>Do something</h2>
          <p>The three things people come here to do.</p>
          <div className="apple-shelf">
            <button
              className="apple-tile"
              onClick={() => {
                go('vms');
                onCreate?.();
              }}
            >
              <Monitor size={22} />
              <b>Create a machine</b>
              <p>From a template in under a minute.</p>
            </button>
            <button className="apple-tile" onClick={() => go('monitoring')}>
              <Archive size={22} />
              <b>Check capacity</b>
              <p>Headroom and platform versions.</p>
            </button>
            <button className="apple-tile" onClick={() => go('alerts')}>
              <Terminal size={22} />
              <b>Review alerts</b>
              <p>Warning events across the cluster.</p>
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}

export function ConsoleHub({ vms, onOpen, onCreate }) {
  const live = (vms || []).filter((v) => v.status === 'Running');
  return (
    <div className="hub-page">
      <PageHero kicker="Display" title="Consoles" lede="Open a live display to any running guest." />
      {live.length === 0 ? (
        <div className="empty" style={{ minHeight: 280 }}>
          <div>
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
        <div className="grid">
          {live.map((v) => (
            <button key={v.id} className="gcard" onClick={() => onOpen(v)}>
              <b>{v.name}</b>
              <Status s={v.status} />
              <div className="m">
                {v.os || '—'} · {v.ip || '—'}
              </div>
              <div style={{ marginTop: 10 }}>
                <span className="btn primary">
                  <Terminal size={12} />
                  Open
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function SettingsPage({ theme, setTheme }) {
  const [policy, setPolicy] = useState({ enabled: false, cooldown_secs: 600 });
  const [policyBusy, setPolicyBusy] = useState(false);
  const [integrations, setIntegrations] = useState([]);
  const [about, setAbout] = useState({ versions: null, caps: null });
  const [headroom, setHeadroom] = useState(null);
  const [err, setErr] = useState('');

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [p, integ, versions, caps, hr] = await Promise.all([
          api.getSelfHealingPolicy().catch(() => ({ enabled: false })),
          api.integrationsStatus().catch(() => ({ integrations: [] })),
          api.platformVersions().catch(() => null),
          api.platformCapabilities().catch(() => null),
          api.capacityHeadroom().catch(() => null),
        ]);
        if (cancelled) return;
        setPolicy({
          enabled: !!p.enabled,
          namespaces: p.namespaces || 'all',
          cooldown_secs: p.cooldown_secs || 600,
        });
        const list = Array.isArray(integ?.integrations) ? integ.integrations : Array.isArray(integ) ? integ : [];
        setIntegrations(list);
        setAbout({ versions, caps });
        setHeadroom(hr);
      } catch (e) {
        if (!cancelled) setErr(e.message || String(e));
      }
    })();
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
    integrations.find((i) => ids.includes(String(i.id || '').toLowerCase()) || ids.includes(String(i.name || '').toLowerCase()));

  const packetwolf = findInteg(['packetwolf']);
  const atlas = findInteg(['atlas']);
  const oidc = findInteg(['oidc', 'sso']);

  const integRow = (n, d, I, item) => {
    const ok = item?.configured || item?.probe === 'ok';
    const detail = item?.configured
      ? item.endpoint || item.probe || 'Configured'
      : d;
    return (
      <div className="srow2" key={n}>
        <I size={16} style={{ color: 'var(--accent)' }} />
        <div>
          <b>{n}</b>
          <small>{detail}</small>
        </div>
        <span className="r">{ok ? <Status s="Healthy" /> : <Status s="Unknown" />}</span>
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
        verLabel(kvVersion.kubevirt || kvVersion.kubevirt_version, null),
        verLabel(kvVersion.cdi || kvVersion.cdi_version, null),
        verLabel(kvVersion.veyron || kvVersion.version, null),
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
              <small>Restart Failed/Unknown VMIs (self-healing policy).</small>
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
              <small>Dry-run reports unhealthy VMIs; heal restarts them.</small>
            </div>
            <div className="r" style={{ display: 'flex', gap: 6 }}>
              <button
                className="btn"
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
                className="btn"
                disabled={policyBusy}
                onClick={async () => {
                  if (!window.confirm('Heal unhealthy VMIs now?')) return;
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
          {integRow('PacketWolf', 'Configure via VEYRON_PACKETWOLF_URL', Network, packetwolf)}
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
                <small>
                  {JSON.stringify(headroom).slice(0, 120)}
                  {JSON.stringify(headroom).length > 120 ? '…' : ''}
                </small>
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
                    about.caps.windows_golden_images && 'Windows golden images',
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

  return (
    <div className="scrim" onClick={onClose}>
      <div className="sheet" onClick={(e) => e.stopPropagation()} style={{ width: mode === 'console' ? 'min(960px, 96%)' : undefined }}>
        <div className="sheet-h">
          <Terminal size={15} />
          {vm.name} · {mode === 'logs' ? 'logs' : 'console'}
          <button className="tb x" onClick={onClose}>
            <X size={15} />
          </button>
        </div>
        {mode === 'logs' ? (
          <LogsPanel text={logs} loading={loading} error={error} />
        ) : (
          <VncConsole vm={vm} />
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

export function NewSheet({ res, templates, storageClasses, onClose, onCreate }) {
  const [name, setName] = useState('');
  const [template, setTemplate] = useState(templates?.[0]?.name || '');
  const [cls, setCls] = useState(storageClasses?.[0]?.name || '');
  const [size, setSize] = useState(res.kind === 'Image' ? '20Gi' : '10Gi');
  const [url, setUrl] = useState('');
  const [cidr, setCidr] = useState('10.244.100.0/24');

  const canCreate =
    !!name &&
    (res.kind !== 'Image' || !!url.trim());

  return (
    <div className="scrim" onClick={onClose}>
      <div className="sheet" onClick={(e) => e.stopPropagation()}>
        <div className="sheet-h">
          <Plus size={15} />
          New {res.kind}
          <button className="tb x" onClick={onClose}>
            <X size={15} />
          </button>
        </div>
        <div style={{ padding: '8px 0' }}>
          <div className="field">
            <label>Name</label>
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder={`e.g. ${res.rows[0]?.name || 'name'}`} />
          </div>
          {res.kind === 'VirtualMachine' && (
            <div className="field">
              <label>Template</label>
              <select value={template} onChange={(e) => setTemplate(e.target.value)}>
                {(templates || []).length === 0 && <option value="">(no templates)</option>}
                {(templates || []).map((t) => (
                  <option key={t.id || t.name} value={t.name}>
                    {t.name}
                    {t.cpu != null ? ` · ${t.cpu} vCPU` : ''}
                    {t.ram != null ? ` · ${t.ram} GiB` : ''}
                  </option>
                ))}
              </select>
            </div>
          )}
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
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://…/image.qcow2"
              />
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
          <button
            className="btn primary"
            disabled={!canCreate}
            onClick={() => onCreate({ name, template, cls, size, url, cidr })}
          >
            Create
          </button>
        </div>
      </div>
    </div>
  );
}

export function Login({ onSubmit, error }) {
  const saved = getSavedUsername();
  const [step, setStep] = useState(saved ? 'password' : 'identify');
  const [username, setUsername] = useState(saved || '');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [rememberMe, setRememberMe] = useState(!!saved);
  const [submitting, setSubmitting] = useState(false);

  const scrollToForm = () => {
    document.getElementById('login-sign-in')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  const handleContinue = (e) => {
    e.preventDefault();
    if (!username.trim()) return;
    setStep('password');
  };

  const handleBack = () => {
    setStep('identify');
    setPassword('');
    setShowPassword(false);
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!username.trim() || !password) return;
    setSubmitting(true);
    try {
      await onSubmit(username.trim(), password);
      if (rememberMe) setSavedUsername(username.trim());
      else setSavedUsername('');
    } catch {
      /* parent sets error */
    } finally {
      setSubmitting(false);
    }
  };

  const storeCta = (
    <>
      <a
        href="#login-sign-in"
        className="login-cta-primary"
        onClick={(e) => {
          e.preventDefault();
          scrollToForm();
        }}
      >
        Sign in
      </a>
      <a
        href="#login-sign-in"
        className="login-cta-secondary"
        onClick={(e) => {
          e.preventDefault();
          scrollToForm();
        }}
      >
        Continue
      </a>
    </>
  );

  const panelSubtitle =
    step === 'password' ? (
      <>
        Enter the password for <span className="login-apple-host">{username.trim()}</span>
      </>
    ) : (
      'Sign in'
    );

  return (
    <PremiumLoginShell
      productName="Veyron"
      productWordmark="Veyron"
      heroTitle="Private cloud control."
      heroSubheadline="Fleet, storage, and day-2 ops for KubeVirt — from one console."
      pills={[
        { label: 'KubeVirt', tone: 'sky' },
        { label: 'CDI', tone: 'violet' },
        { label: 'Snapshots', tone: 'amber' },
        { label: 'Console', tone: 'emerald' },
      ]}
      heroCta={storeCta}
      chapterNote="Username and password · sign in to continue"
      panelSubtitle={panelSubtitle}
      panelHint={
        step === 'identify' ? (
          <>
            Use your Veyron account credentials. Default lab user is{' '}
            <span className="mono">admin</span>.
          </>
        ) : null
      }
      showSignInChapter
    >
      {step === 'identify' ? (
        <form key="identify" onSubmit={handleContinue} autoComplete="on" aria-label="Account" className="login-apple-step">
          {error ? <LoginError message={error} /> : null}

          <div className="login-apple-fields">
            <LoginField label="Username" id="username">
              <User className="login-field-icon" size={14} />
              <input
                id="username"
                name="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="login-input"
                placeholder="Username"
                autoComplete="username"
                autoFocus
                required
              />
            </LoginField>
          </div>

          <LoginSubmit loading={false} disabled={!username.trim()}>
            <span>Continue</span>
            <ArrowRight size={16} />
          </LoginSubmit>
        </form>
      ) : (
        <form key="password" onSubmit={handleSubmit} autoComplete="on" aria-label="Password" className="login-apple-step">
          <button type="button" onClick={handleBack} className="login-apple-identity" aria-label="Change account">
            <ChevronLeft size={16} aria-hidden />
            <span>{username.trim()}</span>
          </button>
          <input type="text" name="username" value={username} autoComplete="username" readOnly hidden />

          {error ? <LoginError message={error} /> : null}

          <div className="login-apple-fields">
            <LoginField label="Password" id="password">
              <Lock className="login-field-icon" size={14} />
              <input
                id="password"
                name="password"
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="login-input pr-11"
                placeholder="Password"
                autoComplete="current-password"
                autoFocus
                required
                disabled={submitting}
              />
              <button
                type="button"
                className="login-eye"
                onClick={() => setShowPassword((s) => !s)}
                aria-label={showPassword ? 'Hide password' : 'Show password'}
              >
                {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
            </LoginField>
          </div>

          <LoginRemember
            checked={rememberMe}
            onChange={(checked) => {
              setRememberMe(checked);
              if (!checked) setSavedUsername('');
            }}
            label="Remember me on this device"
          />

          <LoginSubmit loading={submitting} disabled={!password}>
            {submitting ? (
              <>
                <Loader2 size={16} className="spin" />
                <span>Signing in…</span>
              </>
            ) : (
              <>
                <span>Sign In</span>
                <ArrowRight size={16} />
              </>
            )}
          </LoginSubmit>

          <div className="login-secure-note">
            <CheckCircle size={14} aria-hidden />
            <span>Secured with JWT session authentication</span>
          </div>
        </form>
      )}
    </PremiumLoginShell>
  );
}

