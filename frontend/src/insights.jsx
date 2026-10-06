import { useCallback, useEffect, useMemo, useState } from 'react';
import { Search, Trash2, Play, KeyRound, Download, RefreshCw, Plus } from 'lucide-react';
import {
  api, money, relTime, tidyEventMessage, mapRole, mapBinding, mapUser, mapCiliumPolicy,
} from './api.js';
import { Table } from './Table.jsx';
import { PageHero } from './PageHero.jsx';

/** Loads every source independently so one slow or failing endpoint never blanks the page. */
function useSources(sources) {
  const [state, setState] = useState({});
  const [busy, setBusy] = useState(false);
  const reload = useCallback(async () => {
    setBusy(true);
    await Promise.all(
      Object.entries(sources).map(async ([k, fn]) => {
        try {
          const v = await fn();
          setState((s) => ({ ...s, [k]: { v } }));
        } catch (e) {
          setState((s) => ({ ...s, [k]: { err: e.message || String(e) } }));
        }
      }),
    );
    setBusy(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    reload();
  }, [reload]);
  const get = (k) => state[k]?.v;
  const err = (k) => state[k]?.err;
  const loaded = (k) => k in state;
  return { get, err, loaded, busy, reload };
}

function Shell({ kicker, title, lede, busy, onRefresh, children }) {
  return (
    <div className="chapter-page insight-page">
      <PageHero kicker={kicker} title={title} lede={lede} onRefresh={onRefresh} busy={busy} />
      {children}
    </div>
  );
}

function Specs({ items }) {
  return (
    <section className="apple-band-paper">
      <div className="specs">
        {items.map(([label, value, warn]) => (
          <div key={label} data-warn={warn || undefined}>
            <b>{value == null || value === '' ? '—' : String(value)}</b>
            <span>{label}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

/** `wide` bands span the full page; plain bands pair up side by side on wide screens. */
function Band({ kicker, title, lede, wide, actions, children }) {
  return (
    <section className={wide ? 'insight-band' : 'apple-band'}>
      <div className="apple-chapter insight-chapter">
        {kicker && <div className="kicker">{kicker}</div>}
        <div className="insight-head">
          <h2>{title}</h2>
          {actions && <div className="insight-acts">{actions}</div>}
        </div>
        {lede && <p className="insight-lede">{lede}</p>}
        {children}
      </div>
    </section>
  );
}

function Note({ children, warn }) {
  return <p className={`insight-note${warn ? ' warn' : ''}`}>{children}</p>;
}

function DataTable({ cols, rows, empty, loading, error, filter, limit = 50, chrono }) {
  const [q, setQ] = useState('');
  const [all, setAll] = useState(false);
  const shown = useMemo(() => {
    const list = rows || [];
    if (!q) return list;
    const needle = q.toLowerCase();
    return list.filter((r) =>
      Object.values(r).some((v) => (typeof v === 'string' || typeof v === 'number') && String(v).toLowerCase().includes(needle)),
    );
  }, [rows, q]);
  if (error) return <div className="ops-err">{error}</div>;
  if (loading) return <Note>Loading…</Note>;
  return (
    <>
      {filter && (rows || []).length > 8 && (
        <label className="tsearch insight-filter">
          <Search size={14} />
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder={filter} />
        </label>
      )}
      {shown.length === 0 ? (
        <Note>{q ? 'Nothing matches.' : empty}</Note>
      ) : (
        <div className="table-card">
          <Table cols={cols} rows={all ? shown : shown.slice(0, limit)} {...(chrono ? { initialSort: null } : {})} />
        </div>
      )}
      {!all && shown.length > limit && (
        <button type="button" className="btn sm secondary insight-more" onClick={() => setAll(true)}>
          Show all {shown.length}
        </button>
      )}
    </>
  );
}

const isAdmin = (user) => user?.role === 'admin';
const pct = (v) => (v == null || !Number.isFinite(Number(v)) ? '—' : `${Math.round(Number(v) * 10) / 10}%`);
const yesNo = (v) => (v == null ? '—' : v ? 'Yes' : 'No');
const titleCase = (s) => String(s || '').replace(/[_-]+/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());

function downloadJson(filename, data) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

async function run(showToast, fn, done) {
  try {
    await fn();
    if (done) showToast(done);
    return true;
  } catch (e) {
    showToast(e.message || String(e), true);
    return false;
  }
}

/* ───────────────────────── Operations ───────────────────────── */

export function CostsPage({ user, showToast }) {
  const s = useSources({
    summary: api.costSummary,
    forecast: api.costForecast,
    costs: api.costs,
    budgets: api.listBudgets,
    namespaces: api.listNamespaces,
  });
  const summary = s.get('summary') || {};
  const forecast = s.get('forecast') || {};
  const cur = summary.currency || forecast.currency || 'USD';
  const [form, setForm] = useState({ name: '', namespace: 'default', limit: '500', threshold: '80' });
  const canWrite = user?.role === 'admin' || user?.role === 'write';

  const byType = Object.entries(summary.by_resource_type || {}).sort((a, b) => b[1] - a[1]);
  const vmRows = (s.get('costs') || []).map((c, i) => ({
    id: `${c.namespace}/${c.vm_name}/${i}`,
    name: c.vm_name,
    ns: c.namespace,
    cpu: money(c.cpu_cost, cur),
    memory: money(c.memory_cost, cur),
    storage: money(c.storage_cost, cur),
    network: money(c.network_cost, cur),
    total: money(c.total_cost, cur),
  }));
  const budgetRows = (s.get('budgets') || []).map((b) => {
    const used = b.monthly_limit ? (b.current_spend / b.monthly_limit) * 100 : 0;
    return {
      id: b.name,
      name: b.name,
      ns: b.namespace,
      status: b.status || '—',
      spend: `${money(b.current_spend, cur)} of ${money(b.monthly_limit, cur)}`,
      used: Math.round(used),
      threshold: `${b.alert_threshold_percent ?? 80}%`,
      act: canWrite ? (
        <button
          type="button"
          className="btn sm secondary"
          onClick={async () => {
            if (!window.confirm(`Delete budget ${b.name}?`)) return;
            if (await run(showToast, () => api.deleteBudget(b.name), `Budget ${b.name} deleted`)) s.reload();
          }}
        >
          <Trash2 size={12} /> Delete
        </button>
      ) : null,
    };
  });

  const create = async (e) => {
    e.preventDefault();
    const name = form.name.trim();
    const limit = Number(form.limit);
    if (!/^[a-z0-9]([-a-z0-9]*[a-z0-9])?$/.test(name)) {
      showToast('Budget name: lowercase letters, digits and dashes', true);
      return;
    }
    if (!(limit > 0)) {
      showToast('Monthly limit must be more than zero', true);
      return;
    }
    const ok = await run(
      showToast,
      () =>
        api.createBudget({
          name,
          namespace: form.namespace,
          monthly_limit: limit,
          alert_threshold_percent: Number(form.threshold) || 80,
        }),
      `Budget ${name} created`,
    );
    if (ok) {
      setForm((f) => ({ ...f, name: '' }));
      s.reload();
    }
  };

  return (
    <Shell
      kicker="Operations"
      title="Costs"
      lede="What your machines cost each month, where it is heading, and the budgets that keep it in check."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['This month', s.loaded('summary') ? money(summary.total_cost, cur) : '…'],
          ['Projected', s.loaded('forecast') ? money(forecast.projected_monthly, cur) : '…'],
          ['Trend', forecast.trend ? titleCase(forecast.trend) : '—'],
          ['Machines priced', s.loaded('costs') ? vmRows.length : '…'],
          ['Budgets', s.loaded('budgets') ? budgetRows.length : '…'],
        ]}
      />
      <Band kicker="Breakdown" title="Where it goes">
        <div className="rows">
          {byType.length ? (
            byType.map(([k, v]) => (
              <button type="button" key={k} disabled>
                <b>{k.length <= 3 ? k.toUpperCase() : titleCase(k)}</b>
                <span className="why">{summary.total_cost ? pct((v / summary.total_cost) * 100) : '—'} of total</span>
                <span className="mono">{money(v, cur)}</span>
              </button>
            ))
          ) : (
            <button type="button" disabled>
              <b>{s.err('summary') || (s.loaded('summary') ? 'No priced resources' : 'Loading…')}</b>
            </button>
          )}
        </div>
        {(summary.disclaimer || summary.veyron_context?.limitations) && (
          <Note>{summary.disclaimer || summary.veyron_context.limitations}</Note>
        )}
      </Band>
      <Band kicker="Budgets" title="Spending limits">
        {canWrite && (
          <form className="insight-form" onSubmit={create}>
            <label>
              Name
              <input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="team-dev" required />
            </label>
            <label>
              Namespace
              <select value={form.namespace} onChange={(e) => setForm({ ...form, namespace: e.target.value })}>
                {['all', ...(s.get('namespaces') || ['default'])].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Monthly limit ({cur})
              <input type="number" min="1" value={form.limit} onChange={(e) => setForm({ ...form, limit: e.target.value })} />
            </label>
            <label>
              Alert at %
              <input type="number" min="1" max="100" value={form.threshold} onChange={(e) => setForm({ ...form, threshold: e.target.value })} />
            </label>
            <button type="submit" className="btn primary sm">
              <Plus size={13} /> Add budget
            </button>
          </form>
        )}
        <DataTable
          cols={[
            ['name', 'Budget'],
            ['status', 'Status', 'st'],
            ['ns', 'Namespace', 'm'],
            ['spend', 'Spent of limit', 'n'],
            ['used', 'Used', 'meter'],
            ['act', '', 'act'],
          ]}
          rows={budgetRows}
          loading={!s.loaded('budgets')}
          error={s.err('budgets')}
          empty="No budgets yet. Add one to get warned before a namespace overspends."
        />
      </Band>
      <Band wide kicker="Per machine" title="Monthly cost by VM">
        <DataTable
          cols={[
            ['name', 'VM'],
            ['ns', 'Namespace', 'm'],
            ['cpu', 'CPU', 'n'],
            ['memory', 'Memory', 'n'],
            ['storage', 'Storage', 'n'],
            ['network', 'Network', 'n'],
            ['total', 'Total', 'n'],
          ]}
          rows={vmRows}
          loading={!s.loaded('costs')}
          error={s.err('costs')}
          filter="Filter machines"
          empty="No machines to price."
        />
      </Band>
    </Shell>
  );
}

export function SloPage() {
  const s = useSources({ objectives: api.sloObjectives, burn: api.sloBurnRate });
  const objectives = s.get('objectives') || [];
  const burn = s.get('burn') || [];
  const breached = objectives.filter((o) => /breach/i.test(o.status)).length;
  const worst = burn.reduce((m, b) => Math.max(m, Number(b.burn_rate_1h) || 0), 0);
  return (
    <Shell
      kicker="Operations"
      title="SLOs"
      lede="Availability targets for your machines, and how fast each one is spending its error budget."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Objectives', s.loaded('objectives') ? objectives.length : '…'],
          ['Breached', s.loaded('objectives') ? breached : '…', breached > 0],
          ['Worst 1h burn', s.loaded('burn') ? `${Math.round(worst * 10) / 10}×` : '…', worst > 1],
        ]}
      />
      <Band wide kicker="Targets" title="Objectives">
        <DataTable
          cols={[
            ['name', 'Objective'],
            ['status', 'Status', 'st'],
            ['service', 'Service', 'm'],
            ['target', 'Target', 'n'],
            ['current', 'Current', 'n'],
            ['budget', 'Budget left', 'n'],
            ['window', 'Window', 'm'],
          ]}
          rows={objectives.map((o, i) => ({
            id: o.name || i,
            name: o.name,
            status: titleCase(o.status),
            service: o.service || '—',
            target: pct(o.target),
            current: pct(o.current),
            budget: pct(o.error_budget_remaining),
            window: o.window || '—',
          }))}
          loading={!s.loaded('objectives')}
          error={s.err('objectives')}
          empty="No objectives defined."
        />
      </Band>
      <Band wide kicker="Error budget" title="Burn rate" lede="A burn rate of 1× spends the whole budget in exactly one window; higher means faster.">
        <DataTable
          cols={[
            ['name', 'Objective'],
            ['status', 'Status', 'st'],
            ['availability', 'Availability', 'n'],
            ['consumed', 'Budget used', 'n'],
            ['h1', '1h burn', 'n'],
            ['h24', '24h burn', 'n'],
            ['exhaust', 'Runs out in', 'n'],
          ]}
          rows={burn.map((b, i) => ({
            id: b.slo_name || i,
            name: b.slo_name,
            status: titleCase(b.status),
            availability: pct(b.current_availability),
            consumed: pct(b.budget_consumed_percent),
            h1: `${b.burn_rate_1h ?? '—'}×`,
            h24: `${b.burn_rate_24h ?? '—'}×`,
            exhaust: b.projected_exhaustion_hours == null ? '—' : b.projected_exhaustion_hours <= 0 ? 'Exhausted' : `${Math.round(b.projected_exhaustion_hours)}h`,
          }))}
          loading={!s.loaded('burn')}
          error={s.err('burn')}
          empty="No burn-rate data."
        />
      </Band>
    </Shell>
  );
}

export function LogsPage() {
  const s = useSources({ overview: api.logs });
  const overview = s.get('overview') || {};
  const [search, setSearch] = useState('');
  const [level, setLevel] = useState('');
  const [entries, setEntries] = useState(null);
  const [qErr, setQErr] = useState('');
  const [busy, setBusy] = useState(false);

  const query = async (e) => {
    e?.preventDefault();
    setBusy(true);
    setQErr('');
    try {
      setEntries(await api.queryLogs({ search: search.trim(), level }));
    } catch (err) {
      setQErr(err.message || String(err));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    query();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <Shell kicker="Operations" title="Logs" lede="Search what your guests' launcher pods are logging." busy={s.busy || busy} onRefresh={() => { s.reload(); query(); }}>
      <Specs
        items={[
          ['Errors (1h)', overview.error_count ?? '…', overview.error_count > 0],
          ['Warnings (1h)', overview.warn_count ?? '…'],
          ['Info (1h)', overview.info_count ?? '…'],
          ['Lines (1h)', overview.total_1h ?? '…'],
        ]}
      />
      <Band wide kicker="Search" title="Log lines">
        <form className="insight-form" onSubmit={query}>
          <label className="grow">
            Contains
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="error, migration, qemu…" />
          </label>
          <label>
            Level
            <select value={level} onChange={(e) => setLevel(e.target.value)}>
              <option value="">Any</option>
              <option value="error">Error</option>
              <option value="warn">Warning</option>
              <option value="info">Info</option>
            </select>
          </label>
          <button type="submit" className="btn primary sm" disabled={busy}>
            <Search size={13} /> Search
          </button>
        </form>
        <DataTable
          cols={[
            ['when', 'When', 'n'],
            ['status', 'Level', 'st'],
            ['source', 'Source', 'm'],
            ['name', 'Message'],
          ]}
          rows={(entries || []).map((l, i) => ({
            id: i,
            when: relTime(l.timestamp),
            status: titleCase(l.level),
            source: l.source || '—',
            name: l.message,
          }))}
          loading={entries == null && !qErr}
          error={qErr}
          limit={100}
          chrono
          empty={search || level ? 'No lines match.' : 'No launcher log lines in the last hour.'}
        />
        {overview.veyron_context?.limitations && <Note>{overview.veyron_context.limitations}</Note>}
      </Band>
    </Shell>
  );
}

/* ───────────────────────── Security ───────────────────────── */

export function CompliancePage() {
  const s = useSources({ status: api.complianceStatus, reports: api.complianceReports });
  const frameworks = s.get('status') || [];
  const reports = s.get('reports') || [];
  const avg = frameworks.length ? Math.round(frameworks.reduce((a, f) => a + (f.score || 0), 0) / frameworks.length) : null;
  return (
    <Shell
      kicker="Security"
      title="Compliance"
      lede="How the cluster scores against each framework, with reports you can download."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Average score', avg == null ? '…' : `${avg}%`, avg != null && avg < 70],
          ['Frameworks', s.loaded('status') ? frameworks.length : '…'],
          ['Failing controls', s.loaded('status') ? frameworks.reduce((a, f) => a + (f.failing_controls || 0), 0) : '…', true],
        ]}
      />
      <Band wide kicker="Frameworks" title="Scores">
        <DataTable
          cols={[
            ['name', 'Framework'],
            ['status', 'Status', 'st'],
            ['score', 'Score', 'meter'],
            ['passing', 'Passing', 'n'],
            ['failing', 'Failing', 'n'],
            ['checked', 'Checked', 'n'],
          ]}
          rows={frameworks.map((f, i) => ({
            id: f.framework || i,
            name: f.framework,
            status: f.data_incomplete ? 'Incomplete' : f.compliant ? 'Compliant' : 'Non-compliant',
            score: f.score ?? 0,
            passing: `${f.passing_controls ?? 0} / ${f.total_controls ?? 0}`,
            failing: f.failing_controls ?? 0,
            checked: relTime(f.last_checked),
          }))}
          loading={!s.loaded('status')}
          error={s.err('status')}
          empty="No frameworks evaluated."
        />
      </Band>
      <Band wide kicker="Reports" title="Download">
        <DataTable
          cols={[
            ['name', 'Report'],
            ['generated', 'Generated', 'n'],
            ['findings', 'Findings', 'n'],
            ['act', '', 'act'],
          ]}
          rows={reports.map((r, i) => ({
            id: r.id || i,
            name: r.framework || r.id,
            generated: relTime(r.generated_at),
            findings: (r.findings || []).length,
            act: (
              <button type="button" className="btn sm secondary" onClick={() => downloadJson(`${r.id || 'compliance-report'}.json`, r)}>
                <Download size={12} /> JSON
              </button>
            ),
          }))}
          loading={!s.loaded('reports')}
          error={s.err('reports')}
          empty="No reports generated."
        />
      </Band>
    </Shell>
  );
}

export function HuntingPage({ user, showToast }) {
  const s = useSources({
    exporters: api.socExportStatus,
    hunts: api.socHunts,
    playbooks: api.socPlaybooks,
    surface: api.socAttackSurface,
    events: api.socEvents,
  });
  const exporters = s.get('exporters') || [];
  const huntable = exporters.filter((e) => e.configured && ['elastic', 'splunk'].includes(e.backend));
  const surface = s.get('surface') || {};
  const [hunt, setHunt] = useState({ backend: '', query: '', range: '24h' });
  const [result, setResult] = useState(null);
  const admin = isAdmin(user);

  const runHunt = async (e) => {
    e.preventDefault();
    const backend = hunt.backend || huntable[0]?.backend;
    if (!backend || !hunt.query.trim()) return;
    try {
      setResult(await api.runSocHunt({ backend, query: hunt.query.trim(), time_range: hunt.range }));
    } catch (err) {
      showToast(err.message || String(err), true);
    }
  };

  return (
    <Shell
      kicker="Security"
      title="Threat hunting"
      lede="Query your SIEM, fire response playbooks, and see which machines face the internet."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Internet-facing', surface.total_internet_facing ?? '…', surface.total_internet_facing > 0],
          ['Average risk', surface.average_risk_score ?? '…'],
          ['SIEM backends', s.loaded('exporters') ? `${exporters.filter((e) => e.configured).length} / ${exporters.length}` : '…'],
          ['Playbooks', s.loaded('playbooks') ? (s.get('playbooks') || []).length : '…'],
        ]}
      />
      <Band kicker="SIEM" title="Export">
        <DataTable
          cols={[
            ['name', 'Backend'],
            ['status', 'Status', 'st'],
            ['pushed', 'Events pushed', 'n'],
            ['error', 'Last error', 'm'],
          ]}
          rows={exporters.map((e) => ({
            id: e.backend,
            name: { qradar: 'QRadar' }[e.backend] || titleCase(e.backend),
            status: !e.configured ? 'Not configured' : e.last_push_ok ? 'Healthy' : 'Configured',
            pushed: e.events_pushed ?? 0,
            error: e.last_error || '—',
          }))}
          loading={!s.loaded('exporters')}
          error={s.err('exporters')}
          empty="No exporters."
        />
        <Note>Set VEYRON_ELASTIC_*, VEYRON_SPLUNK_*, VEYRON_SENTINEL_* or VEYRON_QRADAR_* on the deployment to push detections.</Note>
      </Band>
      <Band kicker="Hunts" title="Run a hunt">
        {!s.loaded('exporters') ? (
          <Note>Checking SIEM backends…</Note>
        ) : huntable.length ? (
          <form className="insight-form" onSubmit={runHunt}>
            <label>
              Backend
              <select value={hunt.backend || huntable[0].backend} onChange={(e) => setHunt({ ...hunt, backend: e.target.value })}>
                {huntable.map((e) => (
                  <option key={e.backend} value={e.backend}>
                    {titleCase(e.backend)}
                  </option>
                ))}
              </select>
            </label>
            <label className="grow">
              Query
              <input value={hunt.query} onChange={(e) => setHunt({ ...hunt, query: e.target.value })} placeholder="event.action:no-network-policy-ns" />
            </label>
            <label>
              Range
              <select value={hunt.range} onChange={(e) => setHunt({ ...hunt, range: e.target.value })}>
                {['1h', '24h', '7d', '30d'].map((r) => (
                  <option key={r}>{r}</option>
                ))}
              </select>
            </label>
            <button type="submit" className="btn primary sm">
              <Play size={13} /> Run
            </button>
          </form>
        ) : (
          <Note>Hunts query Elastic or Splunk. Neither is configured, so hunts are unavailable.</Note>
        )}
        {result && <pre className="insight-pre">{JSON.stringify(result, null, 2).slice(0, 4000)}</pre>}
        <DataTable
          cols={[
            ['name', 'Saved hunt'],
            ['backend', 'Backend', 'm'],
            ['query', 'Query', 'm'],
          ]}
          rows={(s.get('hunts') || []).map((h) => ({ id: h.id, name: h.name, backend: h.backend, query: h.query }))}
          loading={!s.loaded('hunts')}
          error={s.err('hunts')}
          empty="No saved hunts."
        />
      </Band>
      <Band kicker="Response" title="Playbooks">
        <DataTable
          cols={[
            ['name', 'Playbook'],
            ['status', 'Status', 'st'],
            ['event', 'Fires on', 'm'],
            ['act', '', 'act'],
          ]}
          rows={(s.get('playbooks') || []).map((p) => ({
            id: p.id,
            name: p.name,
            status: p.active ? 'Active' : 'Disabled',
            event: p.event_type,
            act: admin ? (
              <button
                type="button"
                className="btn sm secondary"
                onClick={async () => {
                  if (!window.confirm(`Trigger playbook ${p.name}? It calls its webhook now.`)) return;
                  await run(showToast, () => api.triggerSocPlaybook(p.id, p.event_type), `Playbook ${p.name} triggered`);
                }}
              >
                <Play size={12} /> Trigger
              </button>
            ) : null,
          }))}
          loading={!s.loaded('playbooks')}
          error={s.err('playbooks')}
          empty="No playbooks. Add a ConfigMap labelled veyron.io/type=soc-playbooks to wire a webhook."
        />
      </Band>
      <Band kicker="Exposure" title="Attack surface">
        <DataTable
          cols={[
            ['name', 'VM'],
            ['ns', 'Namespace', 'm'],
            ['type', 'Exposure', 'm'],
            ['detail', 'Detail', 'm'],
            ['risk', 'Risk', 'n'],
          ]}
          rows={(surface.assets || []).map((a, i) => ({
            id: `${a.vm_namespace}/${a.vm_name}/${i}`,
            name: a.vm_name,
            ns: a.vm_namespace,
            type: a.exposure_type,
            detail: a.detail,
            risk: a.risk_score,
          }))}
          loading={!s.loaded('surface')}
          error={s.err('surface')}
          empty="No machine is exposed outside the cluster."
        />
      </Band>
      <Band wide kicker="Stream" title="Recent security events">
        <DataTable
          cols={[
            ['when', 'When', 'n'],
            ['status', 'Severity', 'st'],
            ['name', 'Event'],
            ['target', 'Target', 'm'],
            ['outcome', 'Outcome', 'm'],
          ]}
          rows={(s.get('events') || []).map((e) => ({
            id: e.id,
            when: relTime(e.timestamp),
            status: e.severity,
            name: e.labels?.['detection.title'] || tidyEventMessage(e.message) || e.action,
            target: e.target || '—',
            outcome: e.outcome || '—',
          }))}
          loading={!s.loaded('events')}
          error={s.err('events')}
          filter="Filter events"
          chrono
          empty="No security events."
        />
      </Band>
    </Shell>
  );
}

export function UsersPage({ user, showToast }) {
  const admin = isAdmin(user);
  const s = useSources({
    users: () => (admin ? api.listUsers() : Promise.resolve(null)),
    roles: api.rbacRoles,
    bindings: api.rbacBindings,
  });
  const [form, setForm] = useState({ username: '', display: '', role: 'readonly', password: '' });

  const create = async (e) => {
    e.preventDefault();
    const username = form.username.trim();
    if (!username || form.password.length < 8) {
      showToast('Username required; password needs at least 8 characters', true);
      return;
    }
    const ok = await run(
      showToast,
      () => api.createUser({ username, password: form.password, role: form.role, display_name: form.display.trim() }),
      `User ${username} created`,
    );
    if (ok) {
      setForm({ username: '', display: '', role: 'readonly', password: '' });
      s.reload();
    }
  };

  const userRows = (s.get('users') || []).map((u) => {
    const row = mapUser(u);
    const self = row.name === user?.username;
    return {
      ...row,
      act: (
        <>
          <button
            type="button"
            className="btn sm secondary"
            onClick={async () => {
              const pw = window.prompt(`New password for ${row.name} (8+ characters)`);
              if (!pw) return;
              if (pw.length < 8) {
                showToast('Password needs at least 8 characters', true);
                return;
              }
              await run(showToast, () => api.setUserPassword(row.name, pw), `Password for ${row.name} updated`);
            }}
          >
            <KeyRound size={12} /> Password
          </button>
          {!self && (
            <button
              type="button"
              className="btn sm secondary"
              onClick={async () => {
                if (!window.confirm(`Delete user ${row.name}? They are signed out at once.`)) return;
                if (await run(showToast, () => api.deleteUser(row.name), `User ${row.name} deleted`)) s.reload();
              }}
            >
              <Trash2 size={12} /> Delete
            </button>
          )}
        </>
      ),
    };
  });

  return (
    <Shell
      kicker="Security"
      title="Users & roles"
      lede="Who can sign in to this console, and the Kubernetes roles behind the cluster."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Console users', admin ? (s.loaded('users') ? userRows.length : '…') : 'Admin only'],
          ['Kubernetes roles', s.loaded('roles') ? (s.get('roles') || []).length : '…'],
          ['Role bindings', s.loaded('bindings') ? (s.get('bindings') || []).length : '…'],
        ]}
      />
      <Band wide kicker="Console" title="Users">
        {admin ? (
          <>
            <form className="insight-form" onSubmit={create} autoComplete="off">
              <label>
                Username
                <input value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} required />
              </label>
              <label>
                Display name
                <input value={form.display} onChange={(e) => setForm({ ...form, display: e.target.value })} />
              </label>
              <label>
                Role
                <select value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })}>
                  <option value="readonly">Read only</option>
                  <option value="write">Write</option>
                  <option value="admin">Admin</option>
                </select>
              </label>
              <label>
                Password
                <input type="password" autoComplete="new-password" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} required />
              </label>
              <button type="submit" className="btn primary sm">
                <Plus size={13} /> Add user
              </button>
            </form>
            <DataTable
              cols={[
                ['name', 'Username'],
                ['display', 'Name', 'm'],
                ['role', 'Role', 'm'],
                ['age', 'Created', 'n'],
                ['act', '', 'act'],
              ]}
              rows={userRows}
              loading={!s.loaded('users')}
              error={s.err('users')}
              empty="No console users."
            />
          </>
        ) : (
          <Note>Only admins can see and manage console users.</Note>
        )}
      </Band>
      <Band kicker="Kubernetes" title="Roles">
        <DataTable
          cols={[
            ['name', 'Role'],
            ['ns', 'Scope', 'm'],
            ['rules', 'Rules', 'n'],
            ['verbs', 'Verbs', 'm'],
          ]}
          rows={(s.get('roles') || []).map(mapRole)}
          loading={!s.loaded('roles')}
          error={s.err('roles')}
          filter="Filter roles"
          limit={25}
          empty="No roles."
        />
      </Band>
      <Band kicker="Kubernetes" title="Bindings">
        <DataTable
          cols={[
            ['name', 'Binding'],
            ['role', 'Role', 'm'],
            ['subjects', 'Subjects', 'm'],
          ]}
          rows={(s.get('bindings') || []).map(mapBinding)}
          loading={!s.loaded('bindings')}
          error={s.err('bindings')}
          filter="Filter bindings"
          limit={25}
          empty="No bindings."
        />
      </Band>
    </Shell>
  );
}

/* ───────────────────────── Platform ───────────────────────── */

export function GitopsPage({ user, showToast }) {
  const s = useSources({ gitops: api.gitopsStatus, catalog: api.catalogStatus });
  const g = s.get('gitops') || {};
  const c = s.get('catalog') || {};
  const [preview, setPreview] = useState(null);
  const admin = isAdmin(user);
  const missing = [...(c.missing_templates || []), ...(c.missing_profiles || [])];

  return (
    <Shell
      kicker="Platform"
      title="GitOps & catalog"
      lede="Keep the cluster in step with your repository, and the template catalog in step with this release."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['GitOps', g.sync_status ? titleCase(g.sync_status) : '…'],
          ['Drift', s.loaded('gitops') ? yesNo(g.drift_detected) : '…', !!g.drift_detected],
          ['Catalog', s.loaded('catalog') ? (c.in_sync ? 'In sync' : 'Out of sync') : '…', s.loaded('catalog') && !c.in_sync],
          ['Templates', s.loaded('catalog') ? `${c.cluster_templates ?? 0} / ${c.embedded_templates ?? 0}` : '…'],
          ['Profiles', s.loaded('catalog') ? `${c.cluster_profiles ?? 0} / ${c.embedded_profiles ?? 0}` : '…'],
        ]}
      />
      <Band
        kicker="Repository"
        title="GitOps sync"
        actions={
          admin && (
            <button
              type="button"
              className="btn sm secondary"
              onClick={() => run(showToast, async () => setPreview(await api.gitopsSync({ force: false, dry_run: true })))}
            >
              <RefreshCw size={12} /> Preview sync
            </button>
          )
        }
      >
        <div className="rows">
          {[
            ['Repository', g.repo_url],
            ['Branch', g.branch],
            ['Last commit', g.last_commit],
            ['Last synced', g.last_synced ? relTime(g.last_synced) : null],
          ].map(([k, v]) => (
            <button type="button" key={k} disabled>
              <b>{k}</b>
              <span className="why mono">{v || 'Not configured'}</span>
            </button>
          ))}
        </div>
        {g.note && <Note>{g.note}</Note>}
        {preview && (
          <pre className="insight-pre">{typeof preview === 'string' ? preview : JSON.stringify(preview, null, 2).slice(0, 3000)}</pre>
        )}
      </Band>
      <Band
        kicker="Catalog"
        title="Templates & profiles"
        actions={
          admin && (
            <button
              type="button"
              className="btn sm secondary"
              onClick={async () => {
                if (!window.confirm('Write every embedded template and profile to the cluster catalog now?')) return;
                if (await run(showToast, () => api.catalogSync(), 'Catalog synced')) s.reload();
              }}
            >
              <RefreshCw size={12} /> Sync catalog
            </button>
          )
        }
      >
        <Note warn={s.loaded('catalog') && !c.in_sync}>{c.message || (s.err('catalog') ?? 'Loading…')}</Note>
        {missing.length > 0 && (
          <div className="rows">
            {missing.map((m) => (
              <button type="button" key={m} disabled>
                <b>{m}</b>
                <span className="why">Missing from the cluster</span>
              </button>
            ))}
          </div>
        )}
      </Band>
    </Shell>
  );
}

export function ClustersPage() {
  const s = useSources({ clusters: api.listClusters });
  const d = s.get('clusters') || {};
  const list = d.clusters || [];
  return (
    <Shell kicker="Platform" title="Clusters" lede="Kubeconfig contexts this API can switch between." busy={s.busy} onRefresh={s.reload}>
      <Specs
        items={[
          ['Active context', d.current_context || (s.loaded('clusters') ? 'In-cluster' : '…')],
          ['Contexts', s.loaded('clusters') ? list.length : '…'],
        ]}
      />
      <Band wide kicker="Contexts" title="Known clusters">
        <DataTable
          cols={[
            ['name', 'Cluster'],
            ['status', 'Health', 'st'],
            ['context', 'Context', 'm'],
            ['env', 'Environment', 'm'],
            ['vms', 'VMs', 'n'],
            ['nodes', 'Nodes', 'n'],
            ['active', 'Active', 'm'],
          ]}
          rows={list.map((c) => ({
            id: c.context || c.name,
            name: c.name,
            status: c.health || 'Unknown',
            context: c.context,
            env: c.environment || '—',
            vms: c.vm_count ?? 0,
            nodes: c.node_count ?? 0,
            active: c.is_active ? 'Yes' : '—',
          }))}
          loading={!s.loaded('clusters')}
          error={s.err('clusters')}
          empty="This API runs in-cluster with its service account, so there is only the local cluster."
        />
        {d.veyron_context?.limitations && <Note>{d.veyron_context.limitations}</Note>}
      </Band>
    </Shell>
  );
}

/* ───────────────────────── Storage & network ───────────────────────── */

export function StorageHealthPage() {
  const s = useSources({ usage: api.storageUsage, pools: api.storagePools });
  const usage = s.get('usage') || [];
  const pools = s.get('pools') || [];
  const allocatedOnly = usage.length > 0 && usage.every((u) => u.usage_basis === 'pvc_allocated');
  const full = usage.filter((u) => Number(u.usage_percent) >= 90).length;
  return (
    <Shell
      kicker="Storage"
      title="Storage health"
      lede="How full every volume and storage pool is."
      busy={s.busy}
      onRefresh={s.reload}
    >
      <Specs
        items={[
          ['Pools', s.loaded('pools') ? pools.length : '…'],
          ['Volumes', s.loaded('usage') ? usage.length : '…'],
          [allocatedOnly ? 'Fully allocated' : '90% or fuller', s.loaded('usage') ? full : '…', !allocatedOnly && full > 0],
        ]}
      />
      <Band wide kicker="Pools" title="Storage classes">
        <DataTable
          cols={[
            ['name', 'Pool'],
            ['prov', 'Provisioner', 'm'],
            ['volumes', 'Volumes', 'n'],
            ['used', 'Used', 'n'],
            ['total', 'Total', 'n'],
            ['free', 'Free', 'n'],
          ]}
          rows={pools.map((p) => ({
            id: p.name,
            name: p.name,
            prov: p.provisioner || '—',
            volumes: p.volume_count ?? 0,
            used: p.used_capacity || '—',
            total: p.total_capacity || '—',
            free: !p.available_capacity || p.available_capacity === 'N/A' ? 'Not reported' : p.available_capacity,
          }))}
          loading={!s.loaded('pools')}
          error={s.err('pools')}
          empty="No storage classes."
        />
      </Band>
      <Band wide kicker="Volumes" title="Usage by claim">
        {allocatedOnly && (
          <Note>
            Prometheus isn’t connected, so usage shows space allocated to each claim, not bytes written. Set VEYRON_PROMETHEUS_URL for
            real filesystem usage.
          </Note>
        )}
        <DataTable
          cols={[
            ['name', 'Claim'],
            ['ns', 'Namespace', 'm'],
            ['sc', 'Class', 'm'],
            ['vm', 'VM', 'm'],
            ['used', 'Used', 'n'],
            ['capacity', 'Capacity', 'n'],
            ['pct', 'Usage', 'meter'],
          ]}
          rows={usage.map((u) => ({
            id: `${u.namespace}/${u.pvc_name}`,
            name: u.pvc_name,
            ns: u.namespace,
            sc: u.storage_class || '—',
            vm: u.bound_to_vm || '—',
            used: u.used || '—',
            capacity: u.capacity || '—',
            pct: u.usage_percent == null ? '—' : Math.round(u.usage_percent),
          }))}
          loading={!s.loaded('usage')}
          error={s.err('usage')}
          filter="Filter claims"
          empty="No volumes."
        />
      </Band>
    </Shell>
  );
}

export function CiliumPage() {
  const s = useSources({ status: api.ciliumStatus, policies: api.ciliumPolicies, flows: api.ciliumFlows });
  const st = s.get('status') || {};
  const flows = s.get('flows') || {};
  const flowList = flows.flows || [];
  return (
    <Shell kicker="Network" title="Cilium" lede="The eBPF dataplane: agent health, network policies and recent flows." busy={s.busy} onRefresh={s.reload}>
      <Specs
        items={[
          ['Agents healthy', s.loaded('status') ? `${st.healthy_agents ?? 0} / ${st.agent_count ?? 0}` : '…', st.healthy_agents < st.agent_count],
          ['Hubble', s.loaded('status') ? (st.hubble_enabled ? 'On' : 'Off') : '…'],
          ['Encryption', s.loaded('status') ? (st.encryption_enabled ? 'On' : 'Off') : '…'],
          ['Cluster mesh', s.loaded('status') ? (st.cluster_mesh_enabled ? 'On' : 'Off') : '…'],
          ['Denied flows', flows.denied ?? '…', flows.denied > 0],
        ]}
      />
      <Band wide kicker="Policies" title="Cilium network policies">
        <DataTable
          cols={[
            ['name', 'Policy'],
            ['status', 'Enforcement', 'st'],
            ['kind', 'Kind', 'm'],
            ['ns', 'Namespace', 'm'],
            ['selector', 'Applies to', 'm'],
            ['ingress', 'Ingress', 'n'],
            ['egress', 'Egress', 'n'],
          ]}
          rows={(s.get('policies') || []).map(mapCiliumPolicy)}
          loading={!s.loaded('policies')}
          error={s.err('policies')}
          filter="Filter policies"
          empty="No Cilium policies."
        />
      </Band>
      <Band wide kicker="Flows" title="Recent flows">
        <DataTable
          cols={[
            ['name', 'Source'],
            ['dest', 'Destination', 'm'],
            ['ns', 'Namespace', 'm'],
            ['proto', 'Protocol', 'm'],
            ['port', 'Port', 'n'],
            ['status', 'Verdict', 'st'],
          ]}
          rows={flowList.map((f, i) => ({
            id: i,
            name: f.source,
            dest: f.destination,
            ns: f.namespace || '—',
            proto: f.protocol || '—',
            port: f.port ?? '—',
            status: titleCase(f.verdict),
          }))}
          loading={!s.loaded('flows')}
          error={s.err('flows')}
          filter="Filter flows"
          empty="No flows recorded."
        />
        {flows.flow_source === 'kubernetes_network_policies' && (
          <Note>These flows are derived from NetworkPolicy rules, not live Hubble captures. Paqtra shows live flow history.</Note>
        )}
      </Band>
    </Shell>
  );
}
