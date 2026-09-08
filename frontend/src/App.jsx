import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import {
  ChevronLeft, ChevronDown, Search, Plus, List, LayoutGrid, Play, Square, Pause,
  ArrowLeftRight, Terminal, Camera, Trash2, Monitor, Server, Sparkles,
  X, Info, Copy, RefreshCw, Sun, Moon, PanelLeft, PanelRight, Bell, ChevronRight,
  Compass, LayoutGrid as GridIcon, Disc3, Boxes, HardDrive, Network, Camera as SnapIcon,
  Archive, Waypoints, Radar, Activity, Bell as AlertIcon, Shield, Settings,
} from 'lucide-react';
import { api, getApiKey, setApiKey } from './api.js';
import Sparkline from './Sparkline.jsx';
import { ResourceTable, ResourceInspector } from './Resource.jsx';

const machineWord = (n) => `${n} machine${n === 1 ? '' : 's'}`;

const SC = { Running: 'var(--green)', Stopped: 'var(--gray)', Degraded: 'var(--orange)', Paused: 'var(--yellow)', Provisioning: 'var(--accent)', Failed: 'var(--red)', Migrating: 'var(--accent)' };
const Status = ({ s }) => <span className="st" style={{ '--c': SC[s] || 'var(--gray)' }}><i />{s || 'Unknown'}</span>;

// Rail groups mirror the real veyron rail (dashboard-apple-nav.css / dashboard.html).
// Only Mission Control + Virtual machines are real React pages in this first slice;
// every other item is a plain link back to the existing vanilla dashboard.
const NAV = [
  { g: 'Overview', items: [
    { id: 'mission', l: 'Mission Control', I: Sparkles, internal: true },
    { id: 'machine-finder', l: 'Machine Finder', I: Compass, href: '/dashboard#dashboard' },
    { id: 'gallery-wall', l: 'Gallery Wall', I: GridIcon, href: '/dashboard#gallery-wall' },
  ] },
  { g: 'Compute', items: [
    { id: 'vms', l: 'Virtual machines', I: Monitor, internal: true },
    { id: 'hosts', l: 'Hosts', I: Server, internal: true },
    { id: 'images', l: 'Images & ISOs', I: Disc3, href: '/dashboard#images' },
    { id: 'pods', l: 'Pods', I: Boxes, internal: true },
  ] },
  { g: 'Storage & network', items: [
    { id: 'storage', l: 'Storage', I: HardDrive, internal: true },
    { id: 'snapshots', l: 'Snapshots', I: SnapIcon, internal: true },
    { id: 'backups', l: 'Backups', I: Archive, internal: true },
    { id: 'topology', l: 'Topology', I: Waypoints, href: '/dashboard#topology' },
  ] },
  { g: 'Operate', items: [
    { id: 'monitoring', l: 'Monitoring', I: Radar, href: '/dashboard#monitoring' },
    { id: 'alerts', l: 'Alerts', I: AlertIcon, href: '/dashboard#alerts' },
    { id: 'security', l: 'Security', I: Shield, href: '/dashboard#security' },
  ] },
  { g: 'System', items: [
    { id: 'settings', l: 'Settings', I: Settings, href: '/dashboard#settings' },
  ] },
];

const COLS = [
  ['name', 'Name'], ['status', 'Status'], ['cpu', 'vCPU', 'n'], ['memory', 'Memory', 'n'],
  ['ip', 'IP'], ['node', 'Host'], ['namespace', 'Namespace'],
];

function Cell({ col, row }) {
  const [k, , t] = col;
  const v = row[k];
  if (k === 'status') return <Status s={v} />;
  if (k === 'name') return <span className="name">{v}</span>;
  return <span className={t === 'n' ? '' : 'dim'}>{v ?? '—'}</span>;
}

function VmTable({ rows, selected, focus, onRow, onMenu, sort, setSort }) {
  const sorted = useMemo(() => {
    const arr = [...rows];
    arr.sort((a, b) => {
      const av = a[sort.k], bv = b[sort.k];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      return (av > bv ? 1 : av < bv ? -1 : 0) * sort.d;
    });
    return arr;
  }, [rows, sort]);
  return (
    <table className="vt">
      <thead>
        <tr>
          {COLS.map((c) => (
            <th key={c[0]} className={c[2] === 'n' ? 'num' : ''} onClick={() => setSort((s) => ({ k: c[0], d: s.k === c[0] ? -s.d : 1 }))}>
              {c[1]}
              {sort.k === c[0] && <span className="s"><ChevronDown size={11} style={{ transform: sort.d < 0 ? 'rotate(180deg)' : '' }} /></span>}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => {
          const key = `${r.namespace || 'default'}/${r.name}`;
          return (
            <tr key={key} aria-selected={selected.has(key)} className={focus === key ? 'focus' : ''}
              onClick={(e) => onRow(r, e)} onContextMenu={(e) => onMenu(r, e)}>
              {COLS.map((c) => <td key={c[0]} className={c[2] === 'n' ? 'num' : ''}><Cell col={c} row={r} /></td>)}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

const ACT = {
  start: ['Start', Play], stop: ['Stop', Square], restart: ['Restart', RefreshCw],
  pause: ['Pause', Pause], migrate: ['Migrate', ArrowLeftRight], console: ['Console', Terminal],
  snapshot: ['Snapshot', Camera], delete: ['Delete', Trash2],
};

// Column/field definitions for every read-only resource page (Hosts, Pods, Storage,
// Snapshots, Backups) — the table/sort/inspector plumbing itself lives once in
// Resource.jsx; each resource below is just its shape.
const memGiB = (ki) => {
  const n = parseFloat(String(ki || '').replace('Ki', ''));
  return Number.isFinite(n) ? `${Math.round(n / 1024 / 1024)}Gi` : '—';
};

const HOST_SC = { Ready: 'var(--green)', NotReady: 'var(--red)', SchedulingDisabled: 'var(--orange)' };
const HOST_COLS = [
  ['name', 'Name'],
  ['status', 'Status'],
  { 0: 'roles', 1: 'Role', render: (r) => (r.roles || []).join(', ') || 'worker' },
  ['cpu_capacity', 'vCPU', 'n'],
  { 0: 'memory_capacity', 1: 'Memory', render: (r) => memGiB(r.memory_capacity) },
  ['os_image', 'OS'],
  ['age', 'Age'],
];
const HOST_FIELDS = [
  ['Role', (h) => (h.roles || []).join(', ') || 'worker'],
  ['CPU', (h) => `${h.cpu_allocatable ?? h.cpu_capacity ?? '—'} allocatable / ${h.cpu_capacity ?? '—'} total`],
  ['Memory', (h) => memGiB(h.memory_capacity)],
  ['Kubelet', 'kubelet_version'], ['OS image', 'os_image'], ['Kernel', 'kernel_version'], ['Age', 'age'],
];

const POD_SC = { Running: 'var(--green)', Pending: 'var(--yellow)', Succeeded: 'var(--gray)', Failed: 'var(--red)', Unknown: 'var(--gray)' };
const POD_COLS = [
  ['name', 'Name'], ['phase', 'Status'], ['namespace', 'Namespace'], ['node_name', 'Node'],
  ['ip', 'IP'], ['restarts', 'Restarts', 'n'], ['age', 'Age'],
];
const POD_FIELDS = [
  ['Namespace', 'namespace'], ['Node', 'node_name'], ['IP', 'ip'],
  ['Containers', (p) => (p.containers || []).join(', ') || '—'], ['Restarts', (p) => p.restarts ?? 0], ['Age', 'age'],
];

const PVC_SC = { Bound: 'var(--green)', Pending: 'var(--yellow)', Lost: 'var(--red)' };
const PVC_COLS = [
  ['name', 'Name'], ['status', 'Status'], ['namespace', 'Namespace'], ['capacity', 'Size'], ['storage_class', 'Class'],
];
const PVC_FIELDS = [['Namespace', 'namespace'], ['Size', 'capacity'], ['Storage class', 'storage_class']];

const SNAP_SC = { Ready: 'var(--green)', InProgress: 'var(--yellow)', Failed: 'var(--red)' };
const SNAP_COLS = [
  ['name', 'Name'], ['status', 'Status'], ['vm_name', 'Machine'], ['namespace', 'Namespace'],
  ['ready', 'Ready', null, (r) => (r.ready ? 'Yes' : 'No')], ['age', 'Age'],
];
const SNAP_FIELDS = [['Machine', 'vm_name'], ['Namespace', 'namespace'], ['Ready', (s) => (s.ready ? 'Yes' : 'No')], ['Age', 'age']];

const BACKUP_COLS = [
  ['name', 'Name'], ['status', 'Status'], ['namespace', 'Namespace'], ['created', 'Created'],
];
const BACKUP_SC = { Completed: 'var(--green)', InProgress: 'var(--yellow)', Failed: 'var(--red)' };
const BACKUP_FIELDS = [['Namespace', 'namespace'], ['Created', 'created']];

const nsKey = (r) => `${r.namespace || 'default'}/${r.name}`;

// Config for every generic read-only list page: how to pull its rows out of the
// loaded data, its table columns, its inspector fields, and its row identity.
const LIST_PAGES = {
  hosts: { title: 'Hosts', noun: 'host', kind: 'Node', rows: ({ nodes }) => nodes, cols: HOST_COLS, statusKey: 'status', statusMap: HOST_SC, rowKey: (r) => r.name, fields: HOST_FIELDS },
  pods: { title: 'Pods', noun: 'pod', kind: 'Pod', rows: ({ pods }) => pods, cols: POD_COLS, statusKey: 'phase', statusMap: POD_SC, rowKey: nsKey, fields: POD_FIELDS },
  storage: { title: 'Storage', noun: 'volume', kind: 'PersistentVolumeClaim', rows: ({ pvcs }) => pvcs, cols: PVC_COLS, statusKey: 'status', statusMap: PVC_SC, rowKey: nsKey, fields: PVC_FIELDS },
  snapshots: { title: 'Snapshots', noun: 'snapshot', kind: 'Snapshot', rows: ({ snapshots }) => snapshots, cols: SNAP_COLS, statusKey: 'status', statusMap: SNAP_SC, rowKey: nsKey, fields: SNAP_FIELDS },
  backups: { title: 'Backups', noun: 'backup', kind: 'Backup', rows: ({ backups }) => backups, cols: BACKUP_COLS, statusKey: 'status', statusMap: BACKUP_SC, rowKey: nsKey, fields: BACKUP_FIELDS },
};

function Inspector({ vm, hide, history, onAct, busy }) {
  const [tab, setTab] = useState('General');
  if (!vm) return <aside className={`insp ${hide ? 'hide' : ''}`}><div className="empty"><div><b>No selection</b>Select a machine to inspect it.</div></div></aside>;
  const running = vm.status === 'Running';
  const key = `${vm.namespace || 'default'}/${vm.name}`;
  const h = history[key] || { cpu: [], mem: [] };
  const tabs = {
    General: [['Name', vm.name], ['Status', <Status key="s" s={vm.status} />], ['Namespace', vm.namespace || 'default'], ['Host', vm.node || '—'], ['IP', vm.ip || '—']],
    Hardware: [['vCPU', vm.cpu ?? '—'], ['Memory', vm.memory ?? '—']],
    Network: [['IP address', vm.ip || '—'], ['Host', vm.node || '—']],
  };
  return (
    <aside className={`insp ${hide ? 'hide' : ''}`}>
      <div className="insp-h"><h2>{vm.name}</h2><div className="kind">VirtualMachine · <Status s={vm.status} /></div></div>
      <div className="spark">
        <div><small>CPU <span>{h.cpu.at(-1) ?? 0}</span></small><b>{vm.cpu ?? '—'} vCPU</b><Sparkline data={h.cpu.length ? h.cpu : [0]} color="var(--accent)" /></div>
        <div><small>Memory <span>{vm.memory ?? '—'}</span></small><b>{vm.memory ?? '—'}</b><Sparkline data={h.mem.length ? h.mem : [0]} color="var(--green)" /></div>
      </div>
      <div className="itabs" role="tablist">{Object.keys(tabs).map((t) => <button key={t} role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>{t}</button>)}</div>
      <div className="form">{tabs[tab].map(([l, v]) => <div className="frow" key={l}><label>{l}</label><span className="mono">{v}</span></div>)}</div>
      <div className="insp-acts">
        {running
          ? <button className="btn" disabled={busy} onClick={() => onAct('stop')}><Square size={12} />Stop</button>
          : <button className="btn" disabled={busy} onClick={() => onAct('start')}><Play size={12} />Start</button>}
        <button className="btn" disabled={busy} onClick={() => onAct('migrate')}><ArrowLeftRight size={13} />Migrate</button>
        <button className="btn primary" disabled={busy} onClick={() => onAct('console')}><Terminal size={13} />Console</button>
      </div>
    </aside>
  );
}

function ConsoleSheet({ vm, onClose }) {
  const key = vm ? `${vm.namespace || 'default'}/${vm.name}` : '';
  return (
    <div className="scrim" onClick={onClose}>
      <div className="sheet" onClick={(e) => e.stopPropagation()}>
        <div className="sheet-h"><Terminal size={15} />{vm?.name} · console<button className="tb x" onClick={onClose}><X size={15} /></button></div>
        <div className="console">
          <div>Full VNC/serial streaming isn't ported to the console yet (Phase 1 scope) —</div>
          <div>open it in the existing dashboard for now:</div>
          <br />
          <div><span className="p">$</span> open /dashboard?vm={encodeURIComponent(key)}#vms</div>
        </div>
        <div className="sheet-f">
          <a className="btn primary" href={`/dashboard?vm=${encodeURIComponent(key)}#vms`}>Open in dashboard</a>
          <button className="btn" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

function Mission({ vms, nodes, go }) {
  const run = vms.filter((v) => v.status === 'Running').length;
  const attn = vms.filter((v) => ['Degraded', 'Failed'].includes(v.status));
  const word = attn.length ? 'Attention' : 'Operational';
  const memGiB = vms.filter((v) => v.status === 'Running').reduce((a, v) => {
    const m = String(v.memory || '');
    if (m.endsWith('Gi')) return a + parseFloat(m);
    if (m.endsWith('Mi')) return a + parseFloat(m) / 1024;
    return a;
  }, 0);
  return (
    <div className="mission"><div className="mwrap">
      <section className="mhero">
        <small>Veyron · Private cloud · {nodes.length} hosts</small>
        <h1>{word}</h1>
        <p>{run} of {vms.length} machines running. {attn.length ? `${attn.length} ${attn.length === 1 ? 'thing needs' : 'things need'} a look.` : 'Nothing needs a look.'}</p>
        <div className="acts">
          <button className="pill" onClick={() => go('vms')}>Virtual machines<ChevronRight size={16} /></button>
        </div>
      </section>
      <div className="specs">
        <div><b>{run}<small>/{vms.length}</small></b><span>VMs running</span></div>
        <div><b>{nodes.length}</b><span>Hosts</span></div>
        <div><b>{Math.round(memGiB)}<small> GiB</small></b><span>Memory in use</span></div>
        <div><b>{attn.length || 'None'}</b><span>Alerts</span></div>
      </div>
      <section className="chapter">
        <h2>Needs attention</h2>
        <p>Machines that aren't in the state you asked for.</p>
        <div className="rows">
          {attn.length === 0 && <p style={{ padding: '14px 4px', color: 'var(--ink-3)', fontSize: 13 }}>Everything looks healthy.</p>}
          {attn.map((v) => (
            <button key={v.name} onClick={() => go('vms', v)}>
              <Status s={v.status} /><b>{v.name}</b><span className="why">on {v.node || 'unknown host'}</span><ChevronRight size={16} />
            </button>
          ))}
        </div>
      </section>
    </div></div>
  );
}

function Login({ onSubmit }) {
  const [key, setKey] = useState('');
  return (
    <div className="login-wrap">
      <form className="login-box" onSubmit={(e) => { e.preventDefault(); onSubmit(key); }}>
        <h2>Veyron Console</h2>
        <p>Enter your API key (same one used on /dashboard).</p>
        <input type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder="API key" autoFocus />
        <button className="btn primary" style={{ width: '100%', justifyContent: 'center' }} type="submit">Continue</button>
      </form>
    </div>
  );
}

export default function App() {
  const [theme, setTheme] = useState(() => (typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'));
  const [authed, setAuthed] = useState(() => !!getApiKey());
  const [page, setPage] = useState('mission');
  const [vms, setVms] = useState([]);
  const [nodes, setNodes] = useState([]);
  const [pods, setPods] = useState([]);
  const [pvcs, setPvcs] = useState([]);
  const [snapshots, setSnapshots] = useState([]);
  const [backups, setBackups] = useState([]);
  // Generic per-page focus (which row is selected/inspected) for every simple
  // read-only list page (hosts, pods, storage, snapshots, backups) — sort is
  // self-contained inside ResourceTable, so this is the only state each new
  // resource type needs. Avoids a new pair of useState hooks per resource.
  const [listFocus, setListFocusState] = useState({});
  const setListFocus = (id, focus) => setListFocusState((s) => ({ ...s, [id]: focus }));
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState(null);
  const [view, setView] = useState('list');
  const [q, setQ] = useState('');
  const [selected, setSelected] = useState(new Set());
  const [focus, setFocus] = useState(null);
  const [menu, setMenu] = useState(null);
  const [sheetVm, setSheetVm] = useState(null);
  const [showSrc, setShowSrc] = useState(true);
  const [showInsp, setShowInsp] = useState(true);
  const [sort, setSort] = useState({ k: 'name', d: 1 });
  const [busy, setBusy] = useState(false);
  const history = useRef({});

  const load = useCallback(async () => {
    if (!authed) return;
    try {
      const [vmList, nodeList, podList, pvcList, snapList, backupList] = await Promise.all([
        api.listVms('all'), api.listNodes(), api.listPods('all'), api.listPvcs(), api.listSnapshots(), api.listBackups(),
      ]);
      setVms(vmList);
      setNodes(nodeList);
      setPods(podList);
      setPvcs(pvcList);
      setSnapshots(snapList);
      setBackups(backupList);
      setErr(null);
      vmList.forEach((v) => {
        const key = `${v.namespace || 'default'}/${v.name}`;
        const cpuPct = v.status === 'Running' ? (parseFloat(v.cpu) || 1) * 10 : 0;
        const h = history.current[key] || { cpu: [], mem: [] };
        h.cpu = [...h.cpu.slice(-23), cpuPct];
        h.mem = [...h.mem.slice(-23), v.status === 'Running' ? 50 : 0];
        history.current[key] = h;
      });
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setLoading(false);
    }
  }, [authed]);

  useEffect(() => { load(); }, [load]);
  useEffect(() => {
    if (!authed) return;
    const t = setInterval(load, 8000);
    return () => clearInterval(t);
  }, [authed, load]);

  const filtered = useMemo(() => vms.filter((v) => !q || `${v.name} ${v.node || ''} ${v.namespace || ''}`.toLowerCase().includes(q.toLowerCase())), [vms, q]);
  const focusVm = vms.find((v) => `${v.namespace || 'default'}/${v.name}` === focus);

  const go = (p, vm) => {
    setPage(p);
    if (vm) {
      const key = `${vm.namespace || 'default'}/${vm.name}`;
      setSelected(new Set([key]));
      setFocus(key);
    }
  };

  const onRow = (r, e) => {
    const key = `${r.namespace || 'default'}/${r.name}`;
    if (e.metaKey || e.ctrlKey) {
      const n = new Set(selected);
      n.has(key) ? n.delete(key) : n.add(key);
      setSelected(n);
      setFocus(key);
    } else if (e.shiftKey && focus) {
      const keys = filtered.map((x) => `${x.namespace || 'default'}/${x.name}`);
      const a = keys.indexOf(focus), b = keys.indexOf(key);
      setSelected(new Set(keys.slice(Math.min(a, b), Math.max(a, b) + 1)));
    } else {
      setSelected(new Set([key]));
      setFocus(key);
    }
  };
  const onMenu = (r, e) => {
    e.preventDefault();
    const key = `${r.namespace || 'default'}/${r.name}`;
    if (!selected.has(key)) { setSelected(new Set([key])); setFocus(key); }
    setMenu({ x: e.clientX, y: e.clientY, vm: r });
  };

  const runOnKeys = async (fn, keys) => {
    setBusy(true);
    try {
      await Promise.all([...keys].map((k) => {
        const [ns, name] = k.split('/');
        return fn(ns, name);
      }));
      await load();
    } catch (e) {
      setErr(e.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  const act = async (action, keys = selected) => {
    setMenu(null);
    if (action === 'console') { setSheetVm(focusVm); return; }
    if (action === 'delete') {
      if (!confirm(`Delete ${keys.size} machine(s)? This cannot be undone.`)) return;
      await runOnKeys((ns, name) => api.deleteVm(ns, name), keys);
      setSelected(new Set());
      setFocus(null);
      return;
    }
    await runOnKeys((ns, name) => api.vmAction(ns, name, action), keys);
  };

  if (!authed) return <div className="vy" data-theme={theme}><Login onSubmit={(k) => { setApiKey(k); setAuthed(true); }} /></div>;

  return (
    <div className="vy" data-theme={theme}>
      <header className="bar">
        <div className="brand"><i>Z</i><b>Veyron</b><small>Console (preview)</small></div>
        <button className="tb" onClick={() => setShowSrc((s) => !s)}><PanelLeft size={16} /></button>
        {page === 'vms' && (
          <div className="seg">
            <button className="tb" aria-pressed={view === 'list'} onClick={() => setView('list')}><List size={15} /></button>
            <button className="tb" aria-pressed={view === 'grid'} onClick={() => setView('grid')}><LayoutGrid size={15} /></button>
          </div>
        )}
        <div className="title">
          {page === 'mission' ? 'Mission Control' : LIST_PAGES[page] ? LIST_PAGES[page].title : 'Virtual machines'}
          <small>
            {page === 'vms' ? machineWord(filtered.length)
              : LIST_PAGES[page] ? (() => { const n = LIST_PAGES[page].rows({ nodes, pods, pvcs, snapshots, backups }).length; return `${n} ${LIST_PAGES[page].noun}${n === 1 ? '' : 's'}`; })()
              : 'Veyron'}
          </small>
        </div>
        <label className="tsearch"><Search size={14} /><input value={q} onChange={(e) => { setQ(e.target.value); if (page === 'mission') setPage('vms'); }} placeholder="Search" /></label>
        <span className="live"><i />Live</span>
        <button className="tb" title="Notifications"><Bell size={15} />{vms.filter((v) => v.status === 'Degraded' || v.status === 'Failed').length > 0 && <span className="badge">{vms.filter((v) => v.status === 'Degraded' || v.status === 'Failed').length}</span>}</button>
        <button className="tb" onClick={() => setTheme((t) => (t === 'dark' ? 'light' : 'dark'))}>{theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}</button>
        <button className="tb" onClick={() => setShowInsp((s) => !s)}><PanelRight size={16} /></button>
        <a className="primary" href="/dashboard#vms" title="Create a VM from the full dashboard for now"><Plus size={14} />New</a>
        <div className="avatar">A</div>
      </header>

      <div className="body">
        <nav className={`source ${showSrc ? '' : 'hide'}`}>
          {NAV.map((g) => (
            <div className="sg" key={g.g}>
              <div className="sg-h">{g.g}</div>
              {g.items.map((it) => {
                const Icon = it.I;
                if (it.internal) {
                  const count = it.id === 'vms' ? vms.length : LIST_PAGES[it.id] ? LIST_PAGES[it.id].rows({ nodes, pods, pvcs, snapshots, backups }).length : null;
                  return (
                    <button key={it.id} className="srow" aria-current={page === it.id ? 'page' : undefined} onClick={() => setPage(it.id)}>
                      <Icon size={15} strokeWidth={1.9} />{it.l}
                      {count != null && <span className="n">{count}</span>}
                    </button>
                  );
                }
                return (
                  <a key={it.id} className="srow" href={it.href}>
                    <Icon size={15} strokeWidth={1.9} />{it.l}
                    <ChevronRight size={12} className="ext" />
                  </a>
                );
              })}
            </div>
          ))}
        </nav>

        {loading ? (
          <div className="center"><div className="empty"><div><b>Loading…</b>Fetching your fleet.</div></div></div>
        ) : err ? (
          <div className="center"><div className="empty"><div><b>Couldn't load data</b>{err}<div style={{ marginTop: 10 }}><button className="btn" onClick={load}><RefreshCw size={12} />Retry</button></div></div></div></div>
        ) : page === 'mission' ? (
          <Mission vms={vms} nodes={nodes} go={go} />
        ) : LIST_PAGES[page] ? (
          (() => {
            const cfg = LIST_PAGES[page];
            const rows = cfg.rows({ nodes, pods, pvcs, snapshots, backups });
            const focusKey = listFocus[page];
            return (
              <div className="center">
                <div className="list">
                  {rows.length === 0 ? (
                    <div className="empty" style={{ height: 300 }}><div><b>No {cfg.noun}s</b>Nothing matches this view.</div></div>
                  ) : (
                    <ResourceTable rows={rows} cols={cfg.cols} rowKey={cfg.rowKey} statusKey={cfg.statusKey} statusMap={cfg.statusMap}
                      focus={focusKey} onRow={(r) => setListFocus(page, cfg.rowKey(r))} />
                  )}
                </div>
                <div className="statusbar"><span>{rows.length} {cfg.noun}{rows.length === 1 ? '' : 's'}</span></div>
              </div>
            );
          })()
        ) : (
          <div className="center">
            <div className="list">
              {filtered.length === 0 ? (
                <div className="empty" style={{ height: 300 }}><div><b>No machines</b>Nothing matches this view.</div></div>
              ) : view === 'grid' ? (
                <div className="grid">
                  {filtered.map((r) => {
                    const key = `${r.namespace || 'default'}/${r.name}`;
                    return (
                      <button key={key} className="gcard" aria-selected={selected.has(key)} onClick={(e) => onRow(r, e)} onContextMenu={(e) => onMenu(r, e)}>
                        <b>{r.name}</b><Status s={r.status} />
                        <div className="m">{r.cpu ?? '—'} vCPU · {r.memory ?? '—'} · {r.node ?? '—'}</div>
                      </button>
                    );
                  })}
                </div>
              ) : (
                <VmTable rows={filtered} selected={selected} focus={focus} onRow={onRow} onMenu={onMenu} sort={sort} setSort={setSort} />
              )}
            </div>
            {selected.size > 1 && (
              <div className="bulk">
                <b>{selected.size} selected</b>
                <button className="tb" disabled={busy} onClick={() => act('start')}><Play size={13} />Start</button>
                <button className="tb" disabled={busy} onClick={() => act('stop')}><Square size={13} />Stop</button>
                <button className="tb" disabled={busy} onClick={() => act('migrate')}><ArrowLeftRight size={14} />Migrate</button>
                <button className="tb" disabled={busy} onClick={() => act('delete')}><Trash2 size={13} />Delete</button>
                <button className="tb" onClick={() => setSelected(new Set())}><X size={14} /></button>
              </div>
            )}
            <div className="statusbar">
              <span>{selected.size ? `${selected.size} of ${filtered.length} selected` : machineWord(filtered.length)}</span>
              <span>{nodes.length} hosts</span>
              <span>{vms.filter((v) => v.status === 'Running').length} running</span>
            </div>
          </div>
        )}

        {page === 'vms' && !loading && !err && (
          <Inspector vm={focusVm} hide={!showInsp} history={history.current} busy={busy} onAct={(a) => act(a, new Set([focus]))} />
        )}
        {LIST_PAGES[page] && !loading && !err && (() => {
          const cfg = LIST_PAGES[page];
          const rows = cfg.rows({ nodes, pods, pvcs, snapshots, backups });
          const item = rows.find((r) => cfg.rowKey(r) === listFocus[page]);
          return (
            <ResourceInspector item={item} hide={!showInsp} kind={cfg.kind} statusKey={cfg.statusKey} statusMap={cfg.statusMap}
              fields={cfg.fields} title={(r) => r.name} emptyLabel={`Select a ${cfg.noun} to inspect it.`} />
          );
        })()}
        {sheetVm && <ConsoleSheet vm={sheetVm} onClose={() => setSheetVm(null)} />}
      </div>

      {menu && (
        <div className="menu" style={{ left: menu.x, top: menu.y }} onClick={(e) => e.stopPropagation()} onMouseLeave={() => setMenu(null)}>
          {menu.vm.status === 'Running'
            ? <button onClick={() => act('stop', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><Square size={13} />Stop</button>
            : <button onClick={() => act('start', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><Play size={13} />Start</button>}
          <button onClick={() => act('restart', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><RefreshCw size={13} />Restart</button>
          <button onClick={() => act('pause', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><Pause size={13} />Pause</button>
          <hr />
          <button onClick={() => { setSheetVm(menu.vm); setMenu(null); }}><Terminal size={14} />Open console</button>
          <button onClick={() => act('migrate', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><ArrowLeftRight size={14} />Migrate to other host</button>
          <button onClick={() => setMenu(null)}><Info size={14} />Get info</button>
          <button onClick={() => { navigator.clipboard?.writeText(menu.vm.name); setMenu(null); }}><Copy size={14} />Copy name</button>
          <hr />
          <button className="danger" onClick={() => act('delete', new Set([`${menu.vm.namespace || 'default'}/${menu.vm.name}`]))}><Trash2 size={14} />Delete</button>
        </div>
      )}
    </div>
  );
}
