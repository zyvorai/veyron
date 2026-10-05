import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import {
  Search, Plus, List, LayoutGrid, Trash2, X, Info, RefreshCw, Download, Activity,
} from 'lucide-react';
import {
  api, getToken, clearSession, getAuthUser, getSavedTheme, setSavedTheme,
  mapVm, mapNode, mapPod, mapPvc, mapClass,
  mapSnapshot, mapBackup, mapImage, mapNetwork, mapTemplate,
  mapGpu, mapAlert, mapSoc, mapAtlasVol, mapAtlasSnap, mapMigration,
  mapCatalogTemplate, mapDataSource,
} from './api.js';
import { emptyData, RES_META, TONE_BY_PAGE, pageLabel, pageBlurb, pageGroup } from './resources.js';
import { Table } from './Table.jsx';
import { Inspector, ACT } from './Inspector.jsx';
import { Mission, ConsoleHub, SettingsPage, ConsoleSheet, NewSheet, Login } from './pages.jsx';
import { MonitoringPage, TopologyPage, DrPage, PacketWolfPage } from './chapters.jsx';
import { Status } from './status.jsx';
import { GlobalNav } from './GlobalNav.jsx';
import { CommandPalette } from './CommandPalette.jsx';
import { EmptyArt, OsBadge, Reveal } from './story.jsx';

function warnCount(rows) {
  return (rows || []).filter((x) => ['Degraded', 'Failed', 'Pending', 'Cordoned'].includes(x.status)).length;
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
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', theme === 'dark' ? '#000000' : '#ffffff');
  }, [theme]);

  const [authed, setAuthed] = useState(() => !!getToken());
  const [loginErr, setLoginErr] = useState('');
  const [user, setUser] = useState(() => getAuthUser());
  const [data, setData] = useState(emptyData);
  const [page, setPage] = useState('mission');
  const [view, setView] = useState('list');
  const [q, setQ] = useState('');
  const [selected, setSelected] = useState(new Set());
  const [focus, setFocus] = useState(null);
  const [menu, setMenu] = useState(null);
  const [sheet, setSheet] = useState(null);
  const [sheetMode, setSheetMode] = useState('console');
  const [showInsp, setShowInsp] = useState(false);
  const [palette, setPalette] = useState(false);
  const [initialTemplate, setInitialTemplate] = useState(null);
  const [showBell, setShowBell] = useState(false);
  const [notifications, setNotifications] = useState([]);
  const [healthOk, setHealthOk] = useState(true);
  const [toast, setToast] = useState(null);
  const [loading, setLoading] = useState(false);
  const back = useRef([]);

  const showToast = useCallback((msg, err = false) => {
    setToast({ msg, err });
    setTimeout(() => setToast(null), 4000);
  }, []);

  const load = useCallback(async () => {
    if (!getToken()) return;
    setLoading(true);
    const next = emptyData();
    const failed = new Set();
    const settle = async (label, fn, mapFn, key, extra) => {
      try {
        const rows = await fn();
        next[key].rows = (rows || []).map(mapFn);
        if (extra) await extra(next);
      } catch (e) {
        console.warn(label, e);
        failed.add(key);
      }
    };

    await Promise.all([
      settle('vms', () => api.listVms('all'), mapVm, 'vms'),
      settle('hosts', () => api.listNodes(), mapNode, 'hosts'),
      settle('gpus', async () => {
        const data = await api.listGpus();
        return Array.isArray(data?.nodes) ? data.nodes : Array.isArray(data) ? data : [];
      }, mapGpu, 'gpus'),
      settle('pods', () => api.listPods('all'), mapPod, 'pods'),
      settle('pvcs', () => api.listPvcs('all'), mapPvc, 'pvcs', async (n) => {
        try {
          const classes = await api.listStorageClasses();
          n.pvcs.extra = {
            title: RES_META.pvcs.extraTitle,
            cols: RES_META.pvcs.extraCols,
            rows: classes.map(mapClass),
          };
        } catch {
          n.pvcs.extra = { title: RES_META.pvcs.extraTitle, cols: RES_META.pvcs.extraCols, rows: [] };
        }
      }),
      settle('snapshots', () => api.listSnapshots(), mapSnapshot, 'snapshots'),
      settle('backups', () => api.listBackups(), mapBackup, 'backups'),
      settle('images', () => api.listImages(), mapImage, 'images', async (n) => {
        try {
          const ds = await api.listDataSources();
          n.images.extra = {
            title: RES_META.images.extraTitle,
            cols: RES_META.images.extraCols,
            rows: (ds || []).map(mapDataSource),
          };
        } catch {
          n.images.extra = { title: RES_META.images.extraTitle, cols: RES_META.images.extraCols, rows: [] };
        }
      }),
      settle('networks', () => api.listNads('all'), mapNetwork, 'networks'),
      settle('templates', async () => {
        try {
          const catalog = await api.listCatalogTemplates();
          if (catalog?.length) return catalog;
        } catch {
          /* fall through to builtins */
        }
        return api.listTemplates();
      }, (raw, i) => {
        if (raw.family != null || raw.tags != null) return mapCatalogTemplate(raw, i);
        return mapTemplate(raw, i);
      }, 'templates', async (n) => {
        try {
          const profiles = await api.listCatalogProfiles();
          n.templates.extra = {
            title: RES_META.templates.extraTitle,
            cols: RES_META.templates.extraCols,
            rows: (profiles || []).map((p, i) => ({
              id: p.name || `p-${i}`,
              name: p.name || `profile-${i}`,
              os: p.family || p.description || '—',
              cpu: p.cpu ?? p.cpus ?? '—',
              ram: p.memory || p.ram || '—',
            })),
          };
        } catch {
          n.templates.extra = { title: RES_META.templates.extraTitle, cols: RES_META.templates.extraCols, rows: [] };
        }
      }),
      settle('migrations', () => api.listMigrations(), mapMigration, 'migrations'),
      settle('alerts', () => api.listAlerts(), mapAlert, 'alerts'),
      settle('soc', () => api.listSocDetections(), mapSoc, 'soc'),
      settle('atlas', () => api.listAtlasVolumes(), mapAtlasVol, 'atlas', async (n) => {
        try {
          const snaps = await api.listAtlasSnapshots();
          n.atlas.extra = {
            title: RES_META.atlas.extraTitle,
            cols: RES_META.atlas.extraCols,
            rows: (snaps || []).map(mapAtlasSnap),
          };
        } catch {
          n.atlas.extra = { title: RES_META.atlas.extraTitle, cols: RES_META.atlas.extraCols, rows: [] };
        }
      }),
      api.health().then(() => setHealthOk(true)).catch(() => setHealthOk(false)),
      api.listNotifications().then(setNotifications).catch(() => setNotifications([])),
    ]);
    setData((prev) => {
      for (const key of failed) next[key] = prev?.[key] || next[key];
      return next;
    });
    setLoading(false);
    if (!getToken()) {
      setAuthed(false);
      setUser(null);
    }
  }, []);

  useEffect(() => {
    if (authed) load();
  }, [authed, load]);

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
      setLoginErr(e.message || 'Invalid username or password');
      throw e;
    }
  };

  const onLogout = () => {
    clearSession();
    setUser(null);
    setAuthed(false);
    setData(emptyData());
  };

  const res = data[page];
  const rows = useMemo(
    () => (res ? res.rows.filter((r) => !q || JSON.stringify(r).toLowerCase().includes(q.toLowerCase())) : []),
    [res, q],
  );
  const focusRow = res?.rows.find((r) => r.id === focus);
  const alerts =
    warnCount(data.vms.rows) +
    warnCount(data.pods.rows) +
    warnCount(data.pvcs.rows) +
    (notifications || []).filter((n) => !n.read).length;

  const go = useCallback(
    (p, id) => {
      back.current.push(page);
      setPage(p);
      setSelected(id != null ? new Set([id]) : new Set());
      setFocus(id ?? null);
      setQ('');
      if (id != null) setTimeout(() => setShowInsp(true), 0);
    },
    [page],
  );

  const openCreate = useCallback((tpl) => {
    setInitialTemplate(typeof tpl === 'string' ? tpl : null);
    setSheet('new');
  }, []);

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
      if ((e.metaKey || e.ctrlKey) && e.key === 'a' && res) {
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

  if (!authed) {
    return <Login onSubmit={onLogin} error={loginErr} />;
  }

  const tone = TONE_BY_PAGE[page] || 'sky';
  const canCreate = !!(res?.canCreate || (!res && page === 'mission'));
  const sheetTarget = sheetMode === 'logs' ? focusRow : focusRow?.name ? focusRow : null;
  const warnRows = res ? warnCount(res.rows) : 0;
  const okRows = res ? res.rows.filter((r) => ['Running', 'Ready', 'Bound', 'Healthy', 'Succeeded', 'Up'].includes(r.status)).length : 0;

  return (
    <div className="vy" data-theme={theme}>
      {toast && <div className={`toast ${toast.err ? 'err' : ''}`}>{toast.msg}</div>}

      <GlobalNav
        page={page}
        go={go}
        data={data}
        theme={theme}
        onToggleTheme={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        onSearch={() => setPalette(true)}
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
        loading={loading}
        canCreate={canCreate}
        onCreate={() => {
          if (!res) setPage('vms');
          openCreate();
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
        {page === 'mission' ? (
          <Mission data={data} go={go} onCreate={openCreate} onConsole={openConsole} healthOk={healthOk} />
        ) : page === 'settings' ? (
          <SettingsPage theme={theme} setTheme={setTheme} />
        ) : page === 'monitoring' ? (
          <MonitoringPage />
        ) : page === 'topology' ? (
          <TopologyPage />
        ) : page === 'dr' ? (
          <DrPage showToast={showToast} />
        ) : page === 'network-brain' ? (
          <PacketWolfPage />
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
                  <input value={q} onChange={(e) => setQ(e.target.value)} placeholder={`Search ${res.l.toLowerCase()}`} />
                  {q && (
                    <button type="button" onClick={() => setQ('')} aria-label="Clear search">
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
                <button className="tb" onClick={() => load()} title="Refresh" aria-label="Refresh">
                  <RefreshCw size={15} className={loading ? 'spin' : ''} />
                </button>
                {res.canCreate && (
                  <button className="btn primary sm" onClick={() => openCreate()}>
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
                <div className="res-metrics">
                  <div>
                    <b>{res.rows.length}</b>
                    <span>total</span>
                  </div>
                  <div>
                    <b>{okRows}</b>
                    <span>healthy</span>
                  </div>
                  <div data-warn={warnRows > 0 || undefined}>
                    <b>{warnRows}</b>
                    <span>need a look</span>
                  </div>
                  {q && (
                    <div>
                      <b>{rows.length}</b>
                      <span>match “{q}”</span>
                    </div>
                  )}
                </div>
              </Reveal>

              {rows.length === 0 ? (
                <div className="empty">
                  <div>
                    <EmptyArt tone={tone} />
                    <b>{loading ? 'Loading…' : q ? 'Nothing matches.' : `No ${res.l.toLowerCase()} yet.`}</b>
                    <p className="lede">
                      {loading
                        ? 'Fetching live data from your cluster.'
                        : q
                          ? 'Try a different search, or clear it.'
                          : page === 'vms'
                            ? 'Create a machine from a current OS template. It takes about a minute.'
                            : pageBlurb(page)}
                    </p>
                    {!loading && !q && res.canCreate && (
                      <div className="acts">
                        <button className="pill" onClick={() => openCreate()}>
                          <Plus size={16} />
                          {page === 'vms' ? 'Create a machine' : `New ${res.kind}`}
                        </button>
                      </div>
                    )}
                  </div>
                </div>
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
                  <Table cols={res.cols} rows={rows} selected={selected} focus={focus} onRow={onRow} onMenu={onMenu} />
                </div>
              )}

              {res.extra && (
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
      </main>

      {res && (
        <>
          <div className={`insp-scrim${showInsp && focusRow ? ' on' : ''}`} onClick={() => setShowInsp(false)} />
          <div className={`insp-slide${showInsp && focusRow ? ' open' : ''}`}>
            <button className="insp-close tb" onClick={() => setShowInsp(false)} aria-label="Close details">
              <X size={15} />
            </button>
            <Inspector
              res={res}
              row={focusRow}
              hide={false}
              onAct={(a) => act(a, new Set([focus].filter(Boolean)))}
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
        />
      )}

      <CommandPalette
        open={palette}
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
