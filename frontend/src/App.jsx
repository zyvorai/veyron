import { useState, useEffect, useMemo, useRef, useCallback, lazy, Suspense } from 'react';
import {
  Search, Plus, List, LayoutGrid, Trash2, X, Info, RefreshCw, Download, Activity, Sparkles,
} from 'lucide-react';
import {
  api, getToken, clearSession, getAuthUser, getSavedTheme, setSavedTheme, UNAUTHORIZED_EVENT,
} from './api.js';
import { TONE_BY_PAGE, PAGE_ORDER, pageLabel, pageBlurb, pageGroup } from './resources.js';
import { useResources } from './useResources.js';
import { Table } from './Table.jsx';
import { Inspector, ACT } from './Inspector.jsx';
import { Mission, ConsoleHub, SettingsPage, ConsoleSheet, NewSheet, Login } from './pages.jsx';
const chapter = (name) => lazy(() => import('./chapters.jsx').then((m) => ({ default: m[name] })));
const EnterprisePage = lazy(() => import('./EnterprisePage.jsx'));
const MonitoringPage = chapter('MonitoringPage');
const TopologyPage = chapter('TopologyPage');
const DrPage = chapter('DrPage');
const NetraPage = chapter('NetraPage');
const GryviaPage = chapter('GryviaPage');
const PaqtraPage = chapter('PaqtraPage');
const insight = (name) => lazy(() => import('./insights.jsx').then((m) => ({ default: m[name] })));
const INSIGHTS = {
  costs: insight('CostsPage'),
  slo: insight('SloPage'),
  logs: insight('LogsPage'),
  compliance: insight('CompliancePage'),
  hunting: insight('HuntingPage'),
  users: insight('UsersPage'),
  gitops: insight('GitopsPage'),
  clusters: insight('ClustersPage'),
  storagehealth: insight('StorageHealthPage'),
  cilium: insight('CiliumPage'),
};
const aiPage = (name) => lazy(() => import('./aipages.jsx').then((m) => ({ default: m[name] })));
const AI_PAGE_COMPONENTS = {
  ai: aiPage('AiHomePage'),
  proposals: aiPage('ProposalsPage'),
  investigations: aiPage('InvestigationsPage'),
  sandboxes: aiPage('SandboxesPage'),
  models: aiPage('ModelsPage'),
};
import { Status } from './status.jsx';
import { GlobalNav } from './GlobalNav.jsx';
import { CommandPalette } from './CommandPalette.jsx';
import { InvestigationStrip } from './InvestigationStrip.jsx';
import { Assistant } from './Assistant.jsx';
import { EmptyArt, OsBadge, Reveal } from './story.jsx';
import { useFocusTrap } from './a11y.js';

function warnCount(rows) {
  return (rows || []).filter((x) => ['Degraded', 'Failed', 'Pending', 'Cordoned'].includes(x.status)).length;
}

function pageFromHash() {
  const id = window.location.hash.replace(/^#\/?/, '');
  return PAGE_ORDER.includes(id) ? id : 'mission';
}

/** Summary figures above a resource table: [count, label, warn?]. */
const sumBy = (rows, k) => rows.reduce((a, r) => a + (Number(r[k]) || 0), 0);

/** Per-page headline numbers; pages not listed get total / healthy / need a look. */
const PAGE_METRICS = {
  recommendations: (rows, count) => [
    [rows.length, 'ideas'],
    [count((r) => /high|critical/i.test(r.status)), 'high priority', true],
  ],
  events: (rows, count) => [
    [rows.length, 'recent'],
    [count((r) => r.status === 'Warning'), 'warnings', true],
  ],
  incidents: (rows, count, meta) => [
    [meta?.open ?? count((r) => r.status !== 'Resolved'), 'open', true],
    [meta?.critical ?? count((r) => /crit/i.test(r.status)), 'critical', true],
    [meta?.resolved24h ?? 0, 'resolved in 24h'],
  ],
  findings: (rows, count, meta) => [
    ...(meta?.score != null ? [[meta.score, 'posture score']] : []),
    [rows.length, 'findings'],
    [count((r) => /high|critical/i.test(r.status)), 'high or critical', true],
  ],
  audit: (rows, count) => [
    [rows.length, 'entries'],
    [new Set(rows.map((r) => r.user)).size, 'actors'],
    [count((r) => r.status === 'Warning'), 'warnings', true],
  ],
  operators: (rows, count) => [
    [rows.length, 'operators'],
    [count((r) => r.status === 'Running'), 'running'],
    [count((r) => r.status !== 'Running'), 'need a look', true],
  ],
  helm: (rows, count) => [
    [rows.length, 'releases'],
    [count((r) => r.status === 'deployed'), 'deployed'],
    [count((r) => !/deployed|superseded/.test(r.status)), 'need a look', true],
  ],
  quotas: (rows) => [[rows.length, 'quotas']],
  crds: (rows, count) => [
    [rows.length, 'definitions'],
    [count((r) => r.count > 0), 'in use'],
    [sumBy(rows, 'count'), 'objects'],
  ],
  workloads: (rows, count) => [
    [rows.length, 'workloads'],
    [count((r) => r.status === 'Ready'), 'ready'],
    [count((r) => r.status !== 'Ready'), 'need a look', true],
  ],
  orphans: (rows) => [[rows.length, 'unattached claims', true]],
  schedules: (rows, count) => [
    [rows.length, 'schedules'],
    [count((r) => r.status === 'Enabled'), 'enabled'],
  ],
  netpols: (rows) => [[rows.length, 'policies']],
};

function pageMetrics(page, rows, { okRows, warnRows, podSummary, includeCompleted, meta }) {
  const count = (f) => rows.filter(f).length;
  if (PAGE_METRICS[page]) return PAGE_METRICS[page](rows, count, meta);
  if (page === 'templates') {
    return [
      [rows.length, 'templates'],
      [count((r) => r.os === 'linux'), 'Linux'],
      [count((r) => r.os === 'windows'), 'Windows'],
    ];
  }
  if (page === 'alerts' || page === 'soc') {
    return [
      [rows.length, 'total'],
      [count((r) => /^warn/i.test(r.severity)), 'warning', true],
      [count((r) => /^(crit|high|error)/i.test(r.severity)), 'critical', true],
    ];
  }
  const out = [
    [rows.length, page === 'pods' ? 'shown' : 'total'],
    [okRows, 'healthy'],
    [warnRows, 'need a look', true],
  ];
  if (page === 'pods' && !includeCompleted && podSummary) {
    out.push([(podSummary.succeeded || 0) + (podSummary.failed || 0), 'completed hidden']);
  }
  return out;
}

const haystacks = new WeakMap();
function haystack(row) {
  let h = haystacks.get(row);
  if (h == null) {
    h = Object.values(row)
      .filter((v) => v != null && typeof v !== 'object' && typeof v !== 'function')
      .join(' ')
      .toLowerCase();
    haystacks.set(row, h);
  }
  return h;
}

function downloadCsv(filename, cols, rows) {
  const header = cols.map((c) => c[1] || c[0]).join(',');
  const body = rows
    .map((r) =>
      cols
        .map((c) => {
          const v = r[c[0]] ?? '';
          const s = String(v).replace(/"/g, '""');
          return /[",\n]/.test(s) ? `"${s}"` : s;
        })
        .join(','),
    )
    .join('\n');
  const blob = new Blob([`${header}\n${body}\n`], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export default function App() {
  const [theme, setThemeState] = useState(() => {
    const saved = getSavedTheme();
    return saved === 'light' ? 'light' : 'dark';
  });
  const setTheme = useCallback((t) => {
    setThemeState(t);
    setSavedTheme(t);
  }, []);
  const [authed, setAuthed] = useState(() => !!getToken());
  useEffect(() => {
    const shown = authed ? theme : 'light';
    document.documentElement.setAttribute('data-theme', shown);
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', shown === 'dark' ? '#000000' : '#ffffff');
  }, [theme, authed]);

  const [loginErr, setLoginErr] = useState('');
  const [user, setUser] = useState(() => getAuthUser());
  const [page, setPage] = useState(pageFromHash);
  const [view, setView] = useState('list');
  const [qInput, setQInput] = useState('');
  const [q, setQ] = useState('');
  useEffect(() => {
    const t = setTimeout(() => setQ(qInput.trim()), 150);
    return () => clearTimeout(t);
  }, [qInput]);
  const [selected, setSelected] = useState(new Set());
  const [focus, setFocus] = useState(null);
  const [menu, setMenu] = useState(null);
  const [sheet, setSheet] = useState(null);
  const [sheetMode, setSheetMode] = useState('console');
  const [showInsp, setShowInsp] = useState(false);
  const [palette, setPalette] = useState(false);
  const [initialTemplate, setInitialTemplate] = useState(null);
  const [showBell, setShowBell] = useState(false);
  const [toast, setToast] = useState(null);
  const [aiOpen, setAiOpen] = useState(false);
  const [aiSeed, setAiSeed] = useState(null);
  const back = useRef([]);
  const inspRef = useRef(null);

  const toastTimer = useRef(null);
  const showToast = useCallback((msg, err = false) => {
    setToast({ msg, err });
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(null), err ? 9000 : 4000);
  }, []);

  const {
    data, errors, loading, loaded, lastLoaded, healthOk, notifications, refreshVisible,
    podSummary, includeCompleted, setIncludeCompleted,
  } = useResources({ enabled: authed, page });
  const load = refreshVisible;

  useEffect(() => {
    const out = () => {
      setUser(null);
      setLoginErr('Your session expired. Sign in again.');
      setAuthed(false);
    };
    window.addEventListener(UNAUTHORIZED_EVENT, out);
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, out);
  }, []);

  useEffect(() => {
    setShowInsp(false);
    document.querySelector('.stage')?.scrollTo({ top: 0 });
  }, [page]);

  const onLogin = async (username, password) => {
    try {
      const res = await api.login(username, password);
      setLoginErr('');
      setUser({
        username,
        display_name: res.display_name || username,
        role: res.role || 'readonly',
      });
      setAuthed(true);
    } catch (e) {
      clearSession();
      setUser(null);
      setLoginErr('');
      throw e;
    }
  };

  const onLogout = () => {
    clearSession();
    setLoginErr('');
    setUser(null);
    setAuthed(false);
  };

  const res = data[page];
  const rows = useMemo(
    () => {
      if (!res) return [];
      if (!q) return res.rows;
      const needle = q.toLowerCase();
      return res.rows.filter((r) => haystack(r).includes(needle));
    },
    [res, q],
  );
  const focusRow = res?.rows.find((r) => r.id === focus);
  const alerts =
    warnCount(data.vms.rows) +
    warnCount(data.pods.rows) +
    warnCount(data.pvcs.rows) +
    (notifications || []).filter((n) => !n.read).length;

  useEffect(() => {
    const want = `#/${page}`;
    if (window.location.hash === want) return;
    if (window.location.hash) window.history.pushState(null, '', want);
    else window.history.replaceState(null, '', want);
  }, [page]);

  useEffect(() => {
    const onHash = () => {
      const p = pageFromHash();
      setPage((cur) => (cur === p ? cur : p));
    };
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  const go = useCallback(
    (p, id) => {
      back.current.push(page);
      setPage(p);
      setSelected(id != null ? new Set([id]) : new Set());
      setFocus(id ?? null);
      setQInput('');
      setQ('');
      if (id != null) setTimeout(() => setShowInsp(true), 0);
    },
    [page],
  );

  const openCreate = useCallback((tpl) => {
    setInitialTemplate(typeof tpl === 'string' ? tpl : null);
    setSheet('new');
  }, []);

  const createSchedule = async () => {
    const vmNames = (data.vms?.rows || []).map((v) => `${v.ns}/${v.name}`);
    const target = window.prompt(`Snapshot which VM? (namespace/name)\n\n${vmNames.slice(0, 12).join('\n')}`, vmNames[0] || '');
    if (!target) return;
    const [ns, vm] = target.includes('/') ? target.trim().split('/') : ['default', target.trim()];
    const cron = window.prompt('Cron schedule (UTC)', '0 2 * * *');
    if (!cron) return;
    const keep = window.prompt('Keep how many snapshots? (0 keeps all)', '7');
    if (keep == null) return;
    try {
      await api.createSnapshotSchedule({
        namespace: ns,
        vm_name: vm,
        cron: cron.trim(),
        max_snapshots: Math.max(0, parseInt(keep, 10) || 0),
      });
      await load();
      showToast(`Schedule for ${vm} created`);
    } catch (e) {
      showToast(e.message || String(e), true);
    }
  };

  const onNew = () => (page === 'schedules' ? createSchedule() : openCreate());

  const openConsole = useCallback((vm) => {
    setPage('vms');
    setFocus(vm.id);
    setSelected(new Set([vm.id]));
    setSheetMode('console');
    setSheet('console');
  }, []);

  const onRow = (r, e) => {
    if (e.metaKey || e.ctrlKey) {
      const n = new Set(selected);
      if (n.has(r.id)) n.delete(r.id);
      else n.add(r.id);
      setSelected(n);
      setFocus(r.id);
    } else if (e.shiftKey && focus != null) {
      const ids = rows.map((x) => x.id);
      const a = ids.indexOf(focus);
      const b = ids.indexOf(r.id);
      setSelected(new Set(ids.slice(Math.min(a, b), Math.max(a, b) + 1)));
    } else {
      setSelected(new Set([r.id]));
      setFocus(r.id);
      setShowInsp(true);
    }
  };

  const onMenu = (r, e) => {
    e.preventDefault();
    if (!selected.has(r.id)) {
      setSelected(new Set([r.id]));
      setFocus(r.id);
    }
    setMenu({ x: e.clientX, y: e.clientY });
  };

  const act = async (a, ids = selected) => {
    setMenu(null);
    if (a === 'console') {
      if (!focus && !(ids instanceof Set && ids.size)) {
        showToast('Select a VM first', true);
        return;
      }
      setSheetMode('console');
      setSheet('console');
      return;
    }
    if (a === 'logs') {
      const logTarget = (res?.rows || []).find((r) => ids.has(r.id)) || focusRow;
      if (!logTarget || res?.kind !== 'Pod') {
        showToast('Select a pod first', true);
        return;
      }
      setFocus(logTarget.id);
      setSheetMode('logs');
      setSheet('console');
      return;
    }

    const targets = (res?.rows || []).filter((r) => ids.has(r.id));
    if (!targets.length) {
      showToast('Select an item first', true);
      return;
    }

    try {
      if (page === 'vms') {
        const powerBulk = ['start', 'stop', 'restart', 'migrate', 'delete'];
        if (targets.length > 1 && powerBulk.includes(a)) {
          const byNs = new Map();
          for (const t of targets) {
            const ns = t.ns || 'default';
            if (!byNs.has(ns)) byNs.set(ns, []);
            byNs.get(ns).push(t.name);
          }
          for (const [ns, names] of byNs) {
            await api.bulkVmAction(ns, names, a);
          }
        } else {
          for (const t of targets) {
            const ns = t.ns || 'default';
            if (a === 'delete') await api.deleteVm(ns, t.name);
            else if (a === 'snapshot') await api.createSnapshot(ns, t.name);
            else if (a === 'clone') await api.cloneVm(ns, t.name, `${t.name}-clone`);
            else if (['start', 'stop', 'restart', 'migrate', 'pause', 'unpause'].includes(a)) {
              await api.vmAction(ns, t.name, a);
            } else {
              showToast(`Unsupported VM action: ${a}`, true);
              return;
            }
          }
        }
      } else if (page === 'hosts') {
        for (const t of targets) {
          if (a === 'cordon') await api.cordonNode(t.name);
          else if (a === 'uncordon') await api.uncordonNode(t.name);
          else if (a === 'reboot') {
            if (!window.confirm(`Reboot host ${t.name}? It will be cordoned first.`)) return;
            await api.rebootNode(t.name);
          } else {
            showToast(`Unsupported host action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'snapshots') {
        for (const t of targets) {
          const ns = t.ns || 'default';
          if (a === 'restore') await api.restoreSnapshot(ns, t.name);
          else if (a === 'delete') await api.deleteSnapshot(ns, t.name);
          else {
            showToast(`Unsupported snapshot action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'backups') {
        for (const t of targets) {
          if (a === 'run') {
            const vmName = t.vm && t.vm !== '—' ? t.vm : t.name;
            await api.createBackup({ vm_name: vmName, name: undefined });
          } else if (a === 'restore') await api.restoreBackup(t.name || t.id);
          else if (a === 'delete') await api.deleteBackup(t.name || t.id);
          else {
            showToast(`Unsupported backup action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'pvcs') {
        for (const t of targets) {
          if (a === 'resize') {
            const cur = String(t.size || '10Gi');
            const m = cur.match(/([\d.]+)\s*(Gi|G|Mi|Ti)?/i);
            const n = m ? parseFloat(m[1]) : 10;
            const unit = m?.[2] || 'Gi';
            const sug = `${Math.ceil(n * 1.5)}${unit.startsWith('G') || unit.startsWith('g') ? 'Gi' : unit}`;
            const next = window.prompt(`New size for ${t.name}`, sug);
            if (!next) return;
            await api.resizePvc(t.ns || 'default', t.name, next.trim());
          } else if (a === 'delete') await api.deletePvc(t.ns || 'default', t.name);
          else {
            showToast(`Unsupported storage action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'images') {
        for (const t of targets) {
          if (a === 'delete') await api.deleteImage(t.ns || 'default', t.name);
          else if (a === 'publish') {
            const ds = window.prompt('DataSource catalog name', t.name);
            if (!ds) return;
            await api.publishImage({
              name: t.name,
              namespace: t.ns || 'default',
              data_source: ds.trim(),
            });
          } else {
            showToast(`Unsupported image action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'pods') {
        for (const t of targets) {
          if (a === 'delete') await api.deletePod(t.ns || 'default', t.name);
          else {
            showToast(`Unsupported pod action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'networks') {
        for (const t of targets) {
          if (a === 'delete') await api.deleteNad(t.ns || 'default', t.name);
          else {
            showToast(`Unsupported network action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'templates') {
        for (const t of targets) {
          if (a === 'clone') {
            const name = `${t.name}-${Date.now().toString(36).slice(-4)}`;
            await api.createVm({ name, template: t.name, namespace: 'default', start: false });
          } else if (a === 'delete') {
            if (!t.catalog) {
              showToast('Builtin templates cannot be deleted', true);
              return;
            }
            await api.deleteTemplate(t.name);
          } else {
            showToast(`Unsupported template action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'alerts') {
        for (const t of targets) {
          if (a === 'resolve') await api.resolveAlert(t.id);
          else {
            showToast(`Unsupported alert action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'soc') {
        for (const t of targets) {
          if (a === 'ack') await api.ackSocDetection(t.id);
          else {
            showToast(`Unsupported SOC action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'migrations') {
        for (const t of targets) {
          if (a === 'delete') await api.cancelMigration(t.id || t.name);
          else {
            showToast(`Unsupported migration action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'atlas') {
        showToast('Use Atlas snapshot actions from the snapshots table below, or Ceph snap in VM Ops', true);
        return;
      } else if (page === 'schedules') {
        if (a !== 'delete') {
          showToast(`Unsupported schedule action: ${a}`, true);
          return;
        }
        if (!window.confirm(`Delete ${targets.length} snapshot schedule(s)? Existing snapshots are kept.`)) return;
        for (const t of targets) await api.deleteSnapshotSchedule(t.ns, t.name);
      } else if (page === 'orphans') {
        if (a !== 'reclaim') {
          showToast(`Unsupported volume action: ${a}`, true);
          return;
        }
        if (user?.role !== 'admin') {
          showToast('Reclaiming volumes needs an admin account', true);
          return;
        }
        for (const ns of new Set(targets.map((t) => t.ns))) {
          const preview = await api.reclaimOrphans({ namespace: ns });
          const names = (preview.candidates || preview.orphans || []).map((o) => o.name);
          if (!names.length) {
            showToast(`Nothing to reclaim in ${ns}`);
            continue;
          }
          const ok = window.confirm(
            `Dry run: reclaiming ${ns} would permanently delete ${names.length} volume(s):\n\n${names.join('\n')}\n\n` +
              'These claims are not referenced by any VM, but a pod may still mount them. Delete them?',
          );
          if (!ok) return;
          await api.reclaimOrphans({ namespace: ns, confirm: true });
        }
      } else {
        showToast(`${a} on ${page}: no API for this action`, true);
        return;
      }
      await load();
      showToast(`${ACT[a]?.[0] || a} · done`);
      if (a === 'delete') {
        setSelected(new Set());
        setFocus(null);
      }
    } catch (e) {
      showToast(e.message || String(e), true);
    }
  };

  const create = async ({ name, template, cls, size, url, cidr, cpus, memory }) => {
    try {
      const kind = (res || data.vms).kind;
      if (kind === 'VirtualMachine') {
        const body = {
          name,
          template: template || undefined,
          namespace: 'default',
          start: true,
        };
        if (cpus) body.cpus = cpus;
        if (memory) body.memory = memory;
        if (size && size !== '10 Gi' && size !== '10Gi') {
          body.disk_size = String(size).replace(/\s+/g, '');
        }
        await api.createVm(body);
        setSheet(null);
        await load();
        showToast(`Created ${name}`);
      } else if (kind === 'PersistentVolumeClaim') {
        await api.createPvc({
          name,
          namespace: 'default',
          size: String(size || '10Gi').replace(/\s+/g, ''),
          storage_class: cls || undefined,
        });
        setSheet(null);
        await load();
        showToast(`Created PVC ${name}`);
      } else if (kind === 'Image') {
        if (!url) {
          showToast('Image URL required', true);
          return;
        }
        await api.importImage({
          name,
          namespace: 'vm-images',
          url,
          size: String(size || '20Gi').replace(/\s+/g, ''),
          storage_class: cls || undefined,
        });
        setSheet(null);
        await load();
        showToast(`Importing image ${name}`);
      } else if (kind === 'Network') {
        await api.createNad({
          name,
          namespace: 'default',
          cidr: cidr || undefined,
        });
        setSheet(null);
        await load();
        showToast(`Created network ${name}`);
      } else {
        showToast(`Create ${kind}: not supported`, true);
      }
    } catch (e) {
      showToast(e.message || String(e), true);
    }
  };

  useEffect(() => {
    const k = (e) => {
      const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName || '');
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j') {
        e.preventDefault();
        setAiOpen((o) => !o);
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPalette((p) => !p);
        return;
      }
      if (e.key === '/' && !typing) {
        e.preventDefault();
        setPalette(true);
        return;
      }
      if (e.key === 'Escape') {
        setAiOpen(false);
        setMenu(null);
        setSheet(null);
        setShowBell(false);
        setShowInsp(false);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'n' && res) {
        e.preventDefault();
        setSheet('new');
      }
      if ((e.metaKey || e.ctrlKey) && e.altKey && e.key === 'i') {
        e.preventDefault();
        setShowInsp((s) => !s);
      }
      const inTable = document.activeElement === document.body || !!document.activeElement?.closest('.table-card');
      if ((e.metaKey || e.ctrlKey) && e.key === 'a' && res && !typing && inTable) {
        e.preventDefault();
        setSelected(new Set(rows.map((r) => r.id)));
      }
    };
    const c = () => {
      setMenu(null);
      setShowBell(false);
    };
    window.addEventListener('keydown', k);
    window.addEventListener('click', c);
    return () => {
      window.removeEventListener('keydown', k);
      window.removeEventListener('click', c);
    };
  }, [rows, res]);

  useFocusTrap(inspRef, !!(showInsp && focusRow));

  if (!authed) {
    return <Login onSubmit={onLogin} error={loginErr} />;
  }

  const tone = TONE_BY_PAGE[page] || 'sky';
  const canCreate = !!(res?.canCreate || (!res && page === 'mission'));
  const sheetTarget = sheetMode === 'logs' ? focusRow : focusRow?.name ? focusRow : null;
  const warnRows = res ? warnCount(res.rows) : 0;
  const okRows = res ? res.rows.filter((r) => ['Running', 'Ready', 'Bound', 'Healthy', 'Succeeded', 'Up'].includes(r.status)).length : 0;
  const atlasOff = page === 'atlas' && data.atlas?.meta?.configured === false;
  const askAi = (question) => {
    setAiSeed(question || null);
    setAiOpen(true);
  };
  const aiContext = {
    page,
    ...(page === 'vms' && focusRow ? { namespace: focusRow.ns || 'default', vm_name: focusRow.name } : {}),
  };
  const metrics = pageMetrics(page, res?.rows || [], { okRows, warnRows, podSummary, includeCompleted, meta: res?.meta });

  return (
    <div className="vy" data-theme={theme}>
      {toast && (
        <div className={`toast ${toast.err ? 'err' : ''}`}>
          {toast.msg}
          {toast.err && (
            <button
              type="button"
              className="toast-ai"
              onClick={() => {
                setToast(null);
                askAi(`This error appeared in the console: "${toast.msg}". What does it mean and how do I fix it?`);
              }}
            >
              <Sparkles size={12} /> Ask AI
            </button>
          )}
        </div>
      )}

      <GlobalNav
        page={page}
        go={go}
        data={data}
        theme={theme}
        onToggleTheme={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        onSearch={() => setPalette(true)}
        onAi={() => setAiOpen((o) => !o)}
        onBell={() =>
          setShowBell((b) => {
            const next = !b;
            if (next && notifications.length) {
              const ids = notifications.map((n) => n.id).filter(Boolean);
              if (ids.length) api.markNotificationsRead(ids).catch(() => {});
            }
            return next;
          })
        }
        alerts={alerts}
        healthOk={healthOk}
        loading={loading && !lastLoaded}
        canCreate={canCreate}
        onCreate={() => {
          if (!res) setPage('vms');
          onNew();
        }}
        user={user}
        onLogout={onLogout}
      />

      {showBell && (
        <div className="bell-panel" onClick={(e) => e.stopPropagation()}>
          <h4>Notifications</h4>
          {(notifications || []).length === 0 ? (
            <div className="bell-empty">
              {alerts
                ? `${alerts} resource alert${alerts === 1 ? '' : 's'} in lists — no stored notifications.`
                : 'You’re all caught up.'}
            </div>
          ) : (
            notifications.slice(0, 40).map((n) => (
              <button
                key={n.id}
                className="bell-item"
                onClick={() => {
                  setShowBell(false);
                  if (n.vm || n.resource) go('vms');
                }}
              >
                <b>{n.title || n.severity || 'Notice'}</b>
                <small>{n.message || n.created_at || ''}</small>
              </button>
            ))
          )}
        </div>
      )}

      <main className="stage" data-tone={tone} data-page={page}>
        <Suspense fallback={<div className="empty"><div><b>Loading…</b></div></div>}>
        {page === 'mission' ? (
          <Mission data={data} go={go} onCreate={openCreate} onConsole={openConsole} healthOk={healthOk} podSummary={podSummary} />
        ) : page === 'enterprise' ? (
          <EnterprisePage user={user} />
        ) : page === 'settings' ? (
          <SettingsPage theme={theme} setTheme={setTheme} />
        ) : page === 'monitoring' ? (
          <MonitoringPage />
        ) : page === 'topology' ? (
          <TopologyPage />
        ) : page === 'dr' ? (
          <DrPage showToast={showToast} />
        ) : page === 'netra' ? (
          <NetraPage />
        ) : page === 'paqtra' ? (
          <PaqtraPage />
        ) : page === 'gryvia' ? (
          <GryviaPage />
        ) : AI_PAGE_COMPONENTS[page] ? (
          (() => {
            const Page = AI_PAGE_COMPONENTS[page];
            return <Page key={page} user={user} showToast={showToast} onAsk={() => askAi()} />;
          })()
        ) : INSIGHTS[page] ? (
          (() => {
            const Page = INSIGHTS[page];
            return <Page key={page} user={user} showToast={showToast} />;
          })()
        ) : page === 'console' ? (
          <ConsoleHub
            vms={data.vms.rows}
            onCreate={() => {
              setPage('vms');
              openCreate();
            }}
            onOpen={openConsole}
          />
        ) : res ? (
          <div className="res-page">
            <div className="localnav">
              <div className="localnav-inner">
                <h1 className="localnav-title">
                  {res.I && <res.I size={18} strokeWidth={1.9} />}
                  {pageLabel(page)}
                </h1>
                <label className="tsearch">
                  <Search size={14} />
                  <input value={qInput} onChange={(e) => setQInput(e.target.value)} placeholder={`Search ${res.l.toLowerCase()}`} />
                  {qInput && (
                    <button type="button" onClick={() => { setQInput(''); setQ(''); }} aria-label="Clear search">
                      <X size={13} />
                    </button>
                  )}
                </label>
                <div className="seg">
                  <button className="tb" aria-pressed={view === 'list'} onClick={() => setView('list')} aria-label="List view">
                    <List size={15} />
                  </button>
                  <button className="tb" aria-pressed={view === 'grid'} onClick={() => setView('grid')} aria-label="Grid view">
                    <LayoutGrid size={15} />
                  </button>
                </div>
                {page === 'pods' && (
                  <button
                    className="btn sm secondary"
                    aria-pressed={includeCompleted}
                    onClick={() => setIncludeCompleted(!includeCompleted)}
                    title="Completed and failed pods are hidden by default"
                  >
                    {includeCompleted ? 'Hide completed' : 'Include completed'}
                  </button>
                )}
                <button className="tb" onClick={() => load()} title="Refresh" aria-label="Refresh">
                  <RefreshCw size={15} className={loading ? 'spin' : ''} />
                </button>
                {res.canCreate && (
                  <button className="btn primary sm" onClick={onNew}>
                    <Plus size={14} />
                    New
                  </button>
                )}
              </div>
            </div>

            <div className="list">
              <Reveal className="res-hero">
                <p className="kicker">{pageGroup(page) || res.kind}</p>
                <h2>{pageLabel(page)}.</h2>
                <p className="res-lede">{pageBlurb(page)}</p>
                {!atlasOff && (
                <div className="res-metrics">
                  {metrics.map(([n, label, warn]) => (
                    <div key={label} data-warn={(warn && n > 0) || undefined}>
                      <b>{n.toLocaleString()}</b>
                      <span>{label}</span>
                    </div>
                  ))}
                  {q && (
                    <div>
                      <b>{rows.length}</b>
                      <span>match “{q}”</span>
                    </div>
                  )}
                </div>
                )}
              </Reveal>

              {errors[page] && (
                <div className="load-error" role="alert">
                  <span>
                    <b>Couldn’t {rows.length ? 'refresh' : 'load'} {res.l.toLowerCase()}.</b> {errors[page]}
                  </span>
                  <button className="btn sm secondary" onClick={() => load()}>
                    <RefreshCw size={12} />
                    Retry
                  </button>
                </div>
              )}

              {page === 'incidents' && <InvestigationStrip onOpen={() => go('investigations')} />}

              {rows.length === 0 ? (
                errors[page] && !loaded(page) ? null : (
                <div className="empty">
                  <div>
                    <EmptyArt tone={tone} />
                    <b>
                      {!loaded(page)
                        ? 'Loading…'
                        : q
                          ? 'Nothing matches.'
                          : atlasOff
                            ? 'Atlas isn’t connected.'
                            : res.empty || `No ${res.l.toLowerCase()} yet.`}
                    </b>
                    <p className="lede">
                      {!loaded(page)
                        ? 'Fetching live data from your cluster.'
                        : q
                          ? 'Try a different search, or clear it.'
                          : atlasOff
                            ? 'Set VEYRON_ATLAS_URL (and VEYRON_ATLAS_TOKEN if Atlas requires auth) on the Veyron deployment to enable Ceph snapshots and off-cluster backups.'
                            : page === 'pods' && !includeCompleted
                              ? 'Completed pods are hidden. Use “Include completed” to show them.'
                              : page === 'vms'
                            ? 'Create a machine from a current OS template. It takes about a minute.'
                            : pageBlurb(page)}
                    </p>
                    {loaded(page) && !q && res.canCreate && (
                      <div className="acts">
                        <button className="pill" onClick={onNew}>
                          <Plus size={16} />
                          {page === 'vms' ? 'Create a machine' : `New ${res.kind}`}
                        </button>
                      </div>
                    )}
                  </div>
                </div>
                )
              ) : view === 'grid' ? (
                <div className="grid">
                  {rows.map((r) => (
                    <button
                      key={r.id}
                      className="gcard"
                      aria-selected={selected.has(r.id)}
                      onClick={(e) => onRow(r, e)}
                      onContextMenu={(e) => onMenu(r, e)}
                    >
                      <div className="gcard-top">
                        {page === 'vms' || page === 'templates' ? (
                          <OsBadge name={page === 'templates' ? r.name : r.os} size={36} />
                        ) : (
                          res.I && (
                            <span className="gcard-ico">
                              <res.I size={18} />
                            </span>
                          )
                        )}
                        {r.status && <Status s={r.status} />}
                      </div>
                      <b>{r.name}</b>
                      <div className="m">
                        {res.cols
                          .slice(2, 5)
                          .map((c) => `${r[c[0]] ?? '—'}${c[3] || ''}`)
                          .join(' · ')}
                      </div>
                    </button>
                  ))}
                </div>
              ) : (
                <div className="table-card">
                  <Table
                    key={page}
                    cols={res.cols}
                    rows={rows}
                    selected={selected}
                    focus={focus}
                    onRow={onRow}
                    onMenu={onMenu}
                    {...(res.chrono ? { initialSort: null } : {})}
                  />
                </div>
              )}

              {res.extra && !atlasOff && (
                <>
                  <div className="sect">
                    <h2>{res.extra.title}</h2>
                    <small>{res.extra.rows.length}</small>
                    <div className="r">
                      <button className="btn sm secondary" onClick={() => load()}>
                        <RefreshCw size={12} />
                        Refresh
                      </button>
                      <button className="btn sm secondary" onClick={() => downloadCsv(`${page}-extra.csv`, res.extra.cols, res.extra.rows)}>
                        <Download size={12} />
                        CSV
                      </button>
                    </div>
                  </div>
                  <div className="table-card">
                    <Table cols={res.extra.cols} rows={res.extra.rows} />
                  </div>
                </>
              )}
            </div>

            {selected.size > 1 && res.acts?.length > 0 && (
              <div className="bulk">
                <b>{selected.size} selected</b>
                {res.acts
                  .filter((a) => a !== 'console' && a !== 'logs')
                  .map((a) => {
                    const [l, I] = ACT[a] || [a, Activity];
                    return (
                      <button key={a} className="tb" onClick={() => act(a)}>
                        <I size={13} />
                        {l}
                      </button>
                    );
                  })}
                <button className="tb" onClick={() => setSelected(new Set())} aria-label="Clear selection">
                  <X size={14} />
                </button>
              </div>
            )}
          </div>
        ) : null}
        </Suspense>
      </main>

      {res && (
        <>
          <div className={`insp-scrim${showInsp && focusRow ? ' on' : ''}`} onClick={() => setShowInsp(false)} />
          <div
            ref={inspRef}
            className={`insp-slide${showInsp && focusRow ? ' open' : ''}`}
            role="dialog"
            aria-modal={showInsp && focusRow ? 'true' : undefined}
            aria-hidden={showInsp && focusRow ? undefined : 'true'}
            {...(showInsp && focusRow ? {} : { inert: '' })}
            aria-label={focusRow ? `${focusRow.name} details` : 'Details'}
            tabIndex={-1}
          >
            <button className="insp-close tb" onClick={() => setShowInsp(false)} aria-label="Close details">
              <X size={15} />
            </button>
            <Inspector
              res={res}
              row={focusRow}
              hide={false}
              onAct={(a) => act(a, new Set([focus].filter(Boolean)))}
              onAskAi={
                res.kind === 'VirtualMachine'
                  ? (row) => askAi(`Check ${row.ns || 'default'}/${row.name}: is anything wrong with it, and is it sized right?`)
                  : undefined
              }
              onOpsDone={(l) => {
                showToast(`${l} · done`);
                load();
              }}
            />
          </div>
        </>
      )}

      {sheet === 'console' && sheetTarget && (
        <ConsoleSheet vm={sheetTarget} mode={sheetMode} onClose={() => setSheet(null)} />
      )}
      {sheet === 'new' && (res || data.vms)?.canCreate && (
        <NewSheet
          res={res?.canCreate ? res : data.vms}
          templates={data.templates.rows}
          storageClasses={data.pvcs.extra?.rows || []}
          initialTemplate={initialTemplate}
          onClose={() => setSheet(null)}
          onCreate={create}
          user={user}
        />
      )}

      <Assistant
        open={aiOpen}
        onClose={() => setAiOpen(false)}
        context={aiContext}
        user={user}
        seed={aiSeed}
        onSeedUsed={() => setAiSeed(null)}
      />

      <CommandPalette
        open={palette}
        onAsk={(q) => {
          setPalette(false);
          askAi(q);
        }}
        onClose={() => setPalette(false)}
        data={data}
        go={go}
        theme={theme}
        onToggleTheme={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        onCreate={() => {
          setPage('vms');
          openCreate();
        }}
        onConsole={openConsole}
      />

      {menu && res && (
        <div className="menu" style={{ left: menu.x, top: menu.y }} onClick={(e) => e.stopPropagation()}>
          {(res.acts || [])
            .filter((a) => a !== 'delete')
            .map((a) => {
              const [l, I] = ACT[a] || [a, Activity];
              return (
                <button key={a} onClick={() => act(a)}>
                  <I size={14} />
                  {l}
                </button>
              );
            })}
          <button
            onClick={() => {
              setMenu(null);
              setShowInsp(true);
            }}
          >
            <Info size={14} />
            Get info
          </button>
          {(res.acts || []).includes('delete') && (
            <>
              <hr />
              <button className="danger" onClick={() => act('delete')}>
                <Trash2 size={14} />
                Delete
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
