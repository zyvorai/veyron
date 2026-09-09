import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import {
  ChevronLeft, Search, Plus, List, LayoutGrid, Terminal, Trash2,
  Sparkles, Settings, Bell, X, Info, RefreshCw, Sun, Moon, PanelLeft, PanelRight,
  Download,
} from 'lucide-react';
import {
  api, getToken, clearSession, getAuthUser, getSavedTheme, setSavedTheme,
  mapVm, mapNode, mapPod, mapPvc, mapClass,
  mapSnapshot, mapBackup, mapImage, mapNetwork, mapTemplate,
} from './api.js';
import { emptyData, NAV, RES_META, PAGE_ORDER } from './resources.js';
import { Table } from './Table.jsx';
import { Inspector, ACT } from './Inspector.jsx';
import { Mission, ConsoleHub, SettingsPage, ConsoleSheet, NewSheet, Login } from './pages.jsx';
import { Status } from './status.jsx';
import { usePageSwipe } from './usePageSwipe.js';

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
    if (saved === 'light' || saved === 'dark') return saved;
    return typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  });
  const setTheme = useCallback((t) => {
    setThemeState(t);
    setSavedTheme(t);
  }, []);

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
  const [showSrc, setShowSrc] = useState(true);
  const [showInsp, setShowInsp] = useState(true);
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
    const settle = async (label, fn, mapFn, key, extra) => {
      try {
        const rows = await fn();
        next[key].rows = (rows || []).map(mapFn);
        if (extra) await extra(next);
      } catch (e) {
        console.warn(label, e);
        next[key].rows = [];
      }
    };

    await Promise.all([
      settle('vms', () => api.listVms('all'), mapVm, 'vms'),
      settle('hosts', () => api.listNodes(), mapNode, 'hosts'),
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
      settle('images', () => api.listImages(), mapImage, 'images'),
      settle('networks', () => api.listNads('all'), mapNetwork, 'networks'),
      settle('templates', () => api.listTemplates(), mapTemplate, 'templates'),
      api.health().then(() => setHealthOk(true)).catch(() => setHealthOk(false)),
      api.listNotifications().then(setNotifications).catch(() => setNotifications([])),
    ]);
    setData(next);
    setLoading(false);
    if (!getToken()) {
      setAuthed(false);
      setUser(null);
    }
  }, []);

  useEffect(() => {
    if (authed) load();
  }, [authed, load]);

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

  const go = (p, id) => {
    back.current.push(page);
    setPage(p);
    setSelected(id != null ? new Set([id]) : new Set());
    setFocus(id ?? null);
    setQ('');
  };

  const swipeTo = useCallback(
    (p) => {
      if (p === page) return;
      back.current.push(page);
      setPage(p);
      setSelected(new Set());
      setFocus(null);
      setQ('');
    },
    [page],
  );

  const { trackRef, pageIndex } = usePageSwipe({
    page,
    onPage: swipeTo,
    enabled: authed && !sheet && !menu && !showBell,
  });

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
      setSheetMode('console');
      setSheet('console');
      return;
    }
    if (a === 'logs') {
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
            const next = `${Math.ceil(n * 1.5)}${unit.startsWith('G') || unit.startsWith('g') ? 'Gi' : unit}`;
            await api.resizePvc(t.ns || 'default', t.name, next);
          } else if (a === 'delete') await api.deletePvc(t.ns || 'default', t.name);
          else {
            showToast(`Unsupported storage action: ${a}`, true);
            return;
          }
        }
      } else if (page === 'images') {
        for (const t of targets) {
          if (a === 'delete') await api.deleteImage(t.ns || 'default', t.name);
          else {
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
          } else if (a === 'delete') await api.deleteTemplate(t.name);
          else {
            showToast(`Unsupported template action: ${a}`, true);
            return;
          }
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

  const create = async ({ name, template, host, cls, size, url, cidr }) => {
    try {
      const kind = (res || data.vms).kind;
      if (kind === 'VirtualMachine') {
        const body = {
          name,
          template: template || undefined,
          namespace: 'default',
          start: true,
        };
        if (size && size !== '10 Gi') {
          body.disk_size = String(size).replace(/\s+/g, '');
        }
        void host;
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
      if (e.key === 'Escape') {
        setMenu(null);
        setSheet(null);
        setShowBell(false);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'n' && res) {
        e.preventDefault();
        setSheet('new');
      }
      if ((e.metaKey || e.ctrlKey) && e.altKey && e.key === 's') {
        e.preventDefault();
        setShowSrc((s) => !s);
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

  const title =
    page === 'mission'
      ? 'Mission Control'
      : page === 'console'
        ? 'ConsoleHub'
        : page === 'settings'
          ? 'Settings'
          : res?.l || 'Veyron';

  const navIcon = (id, I) => {
    if (id === 'mission') return Sparkles;
    if (id === 'console') return Terminal;
    if (id === 'settings') return Settings;
    return I || RES_META[id]?.I || MonitorFallback;
  };

  const sheetTarget =
    sheetMode === 'logs'
      ? focusRow || data.pods.rows[0]
      : focusRow?.name
        ? focusRow
        : data.vms.rows[0];

  return (
    <div className="vy" data-theme={theme}>
      {toast && <div className={`toast ${toast.err ? 'err' : ''}`}>{toast.msg}</div>}
      <header className="bar">
        <div className="brand">
          <i>Z</i>
          <b>Veyron</b>
          <small>Private cloud</small>
        </div>
        <button className="tb" onClick={() => setShowSrc(!showSrc)}>
          <PanelLeft size={16} />
        </button>
        <button
          className="tb"
          onClick={() => {
            const p = back.current.pop();
            if (p) setPage(p);
          }}
          disabled={!back.current.length}
        >
          <ChevronLeft size={16} />
        </button>
        {res && (
          <div className="seg">
            <button className="tb" aria-pressed={view === 'list'} onClick={() => setView('list')}>
              <List size={15} />
            </button>
            <button className="tb" aria-pressed={view === 'grid'} onClick={() => setView('grid')}>
              <LayoutGrid size={15} />
            </button>
          </div>
        )}
        <div className="title">
          {title}
          <small>
            {res ? `${rows.length} ${res.kind}${rows.length === 1 ? '' : 's'}` : loading ? 'Loading…' : 'Veyron'}
          </small>
        </div>
        <label className="tsearch">
          <Search size={14} />
          <input
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              if (!res) setPage('vms');
            }}
            placeholder="Search"
          />
        </label>
        <span className={`live ${!healthOk || alerts ? 'warn' : ''}`}>
          <i />
          {loading ? 'Sync' : healthOk ? 'Live' : 'Degraded'}
        </span>
        <button className="tb" onClick={() => load()} title="Refresh">
          <RefreshCw size={15} />
        </button>
        <button
          className="tb"
          onClick={(e) => {
            e.stopPropagation();
            setShowBell((s) => !s);
          }}
          title="Notifications"
        >
          <Bell size={15} />
          {alerts > 0 && <span className="badge">{alerts}</span>}
        </button>
        <button className="tb" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>
          {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
        </button>
        <button className="tb" onClick={() => setShowInsp(!showInsp)}>
          <PanelRight size={16} />
        </button>
        <button
          className="primary"
          onClick={() => {
            if (!res) setPage('vms');
            setSheet('new');
          }}
        >
          <Plus size={14} />
          New
        </button>
        <div
          className="avatar"
          title={user?.display_name || user?.username ? `Sign out (${user.display_name || user.username})` : 'Sign out'}
          onClick={onLogout}
          role="button"
        >
          {(user?.display_name || user?.username || 'A').charAt(0).toUpperCase()}
        </div>
      </header>

      {showBell && (
        <div className="bell-panel" onClick={(e) => e.stopPropagation()}>
          <h4>Notifications</h4>
          {(notifications || []).length === 0 ? (
            <div className="bell-empty">
              {alerts
                ? `${alerts} resource alert${alerts === 1 ? '' : 's'} in lists — no stored notifications.`
                : 'No notifications.'}
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

      <div className="body">
        <nav className={`source ${showSrc ? '' : 'hide'}`}>
          {NAV.map((g) => (
            <div className="sg" key={g.g}>
              <div className="sg-h">{g.g}</div>
              {g.items.map(([id, l]) => {
                const r = data[id];
                const Icon = navIcon(id, r?.I);
                const warn = r ? warnCount(r.rows) : 0;
                return (
                  <button
                    key={id}
                    className="srow"
                    aria-current={page === id ? 'page' : undefined}
                    onClick={() => go(id)}
                  >
                    <Icon size={15} strokeWidth={1.9} />
                    {l || r?.l || id}
                    {r && <span className={`n ${warn ? 'warn' : ''}`}>{warn || r.rows.length}</span>}
                  </button>
                );
              })}
            </div>
          ))}
        </nav>

        <div className="swipe-host">
          <div className="swipe-track" ref={trackRef}>
            {page === 'mission' ? (
              <Mission
                data={data}
                go={go}
                hostCount={data.hosts.rows.length}
                onCreate={() => setSheet('new')}
              />
            ) : page === 'settings' ? (
              <SettingsPage theme={theme} setTheme={setTheme} />
            ) : page === 'console' ? (
              <div className="center">
                <ConsoleHub
                  vms={data.vms.rows}
                  onCreate={() => {
                    setPage('vms');
                    setSheet('new');
                  }}
                  onOpen={(v) => {
                    setPage('vms');
                    setFocus(v.id);
                    setSelected(new Set([v.id]));
                    setSheetMode('console');
                    setSheet('console');
                  }}
                />
              </div>
            ) : (
              <div className="center">
                <div className="list">
                  <div className="list-kicker">
                    <div className="kicker">{res.kind}</div>
                    <h1>{res.l}</h1>
                    <p>
                      {loading
                        ? 'Loading…'
                        : `${rows.length} ${res.l.toLowerCase()} in this view.`}
                    </p>
                  </div>
                  {rows.length === 0 ? (
                    <div className="empty" style={{ minHeight: 320 }}>
                      <div>
                        <b>No {res.kind.toLowerCase()}s</b>
                        <p className="lede">
                          {loading
                            ? 'Loading…'
                            : q
                              ? 'Nothing matches this search.'
                              : page === 'vms'
                                ? 'Create a machine to get started.'
                                : 'Nothing matches this view.'}
                        </p>
                        {!loading && !q && page === 'vms' && (
                          <div className="acts">
                            <button className="pill" onClick={() => setSheet('new')}>
                              <Plus size={16} />
                              Create a machine
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
                          <b>{r.name}</b>
                          {r.status && <Status s={r.status} />}
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
                    <Table cols={res.cols} rows={rows} selected={selected} focus={focus} onRow={onRow} onMenu={onMenu} />
                  )}
                  {res.extra && (
                    <>
                      <div className="sect">
                        <h2>{res.extra.title}</h2>
                        <small>{res.extra.rows.length}</small>
                        <div className="r">
                          <button className="btn" onClick={() => load()}>
                            <RefreshCw size={12} />
                            Refresh
                          </button>
                          <button
                            className="btn"
                            onClick={() =>
                              downloadCsv(
                                'storage-pools.csv',
                                res.extra.cols,
                                res.extra.rows,
                              )
                            }
                          >
                            <Download size={12} />
                            CSV
                          </button>
                        </div>
                      </div>
                      <Table cols={res.extra.cols} rows={res.extra.rows} />
                    </>
                  )}
                </div>
                {selected.size > 1 && res.acts?.length > 0 && (
                  <div className="bulk">
                    <b>{selected.size} selected</b>
                    {res.acts
                      .filter((a) => a !== 'console' && a !== 'logs')
                      .slice(0, 4)
                      .map((a) => {
                        const [l, I] = ACT[a];
                        return (
                          <button key={a} className="tb" onClick={() => act(a)}>
                            <I size={13} />
                            {l}
                          </button>
                        );
                      })}
                    <button className="tb" onClick={() => setSelected(new Set())}>
                      <X size={14} />
                    </button>
                  </div>
                )}
                <div className="statusbar">
                  <span>{selected.size ? `${selected.size} of ${rows.length} selected` : `${rows.length} items`}</span>
                  <span>
                    {data.hosts.rows.length} host{data.hosts.rows.length === 1 ? '' : 's'}
                  </span>
                  <span>{alerts} alerts</span>
                </div>
              </div>
            )}
          </div>
          <div className="swipe-dots" aria-hidden>
            {PAGE_ORDER.map((id, i) => (
              <i key={id} className={i === pageIndex ? 'on' : ''} />
            ))}
          </div>
        </div>

        {res && <Inspector res={res} row={focusRow} hide={!showInsp} onAct={(a) => act(a, new Set([focus]))} />}
        {sheet === 'console' && sheetTarget && (
          <ConsoleSheet vm={sheetTarget} mode={sheetMode} onClose={() => setSheet(null)} />
        )}
        {sheet === 'new' && (res || data.vms) && (
          <NewSheet
            res={res || data.vms}
            templates={data.templates.rows}
            hosts={data.hosts.rows}
            storageClasses={data.pvcs.extra?.rows || []}
            onClose={() => setSheet(null)}
            onCreate={create}
          />
        )}
      </div>

      {menu && res && (
        <div className="menu" style={{ left: menu.x, top: menu.y }} onClick={(e) => e.stopPropagation()}>
          {(res.acts || [])
            .filter((a) => a !== 'delete')
            .map((a) => {
              const [l, I] = ACT[a];
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

function MonitorFallback(props) {
  return <Sparkles {...props} />;
}
