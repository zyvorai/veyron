import { useCallback, useEffect, useRef, useState } from 'react';
import {
  api, getToken,
  mapVm, mapNode, mapPod, mapPvc, mapClass,
  mapSnapshot, mapBackup, mapImage, mapNetwork, mapTemplate,
  mapGpu, mapAlert, mapSoc, mapAtlasVol, mapAtlasSnap, mapMigration,
  mapCatalogTemplate, mapDataSource,
} from './api.js';
import { emptyData, RES_META } from './resources.js';

export const REFRESH_MS = 30_000;

/** Resources Mission Control and the nav counts need first. */
export const CORE_KEYS = ['vms', 'hosts', 'pvcs', 'alerts', 'templates'];

/** Pods are loaded only on their own page; listing every pod can be megabytes. */
const LAZY_KEYS = new Set(['pods']);

const pods = { includeCompleted: false };

async function extraRows(key, fetchRows, mapFn) {
  try {
    return { title: RES_META[key].extraTitle, cols: RES_META[key].extraCols, rows: ((await fetchRows()) || []).map(mapFn) };
  } catch {
    return { title: RES_META[key].extraTitle, cols: RES_META[key].extraCols, rows: [] };
  }
}

const mapProfile = (p, i) => ({
  id: p.name || `p-${i}`,
  name: p.name || `profile-${i}`,
  os: p.family || p.description || '—',
  cpu: p.cpu ?? p.cpus ?? '—',
  ram: p.memory || p.ram || '—',
});

/** key -> async () => ({ rows, extra? }) */
const LOADERS = {
  vms: async () => ({ rows: ((await api.listVms('all')) || []).map(mapVm) }),
  hosts: async () => ({ rows: ((await api.listNodes()) || []).map(mapNode) }),
  gpus: async () => {
    const d = await api.listGpus();
    const list = Array.isArray(d?.nodes) ? d.nodes : Array.isArray(d) ? d : [];
    return { rows: list.map(mapGpu) };
  },
  pods: async () => ({ rows: ((await api.listPods('all', { active: !pods.includeCompleted })) || []).map(mapPod) }),
  pvcs: async () => {
    const [rows, extra] = await Promise.all([
      api.listPvcs('all'),
      extraRows('pvcs', () => api.listStorageClasses(), mapClass),
    ]);
    return { rows: (rows || []).map(mapPvc), extra };
  },
  snapshots: async () => ({ rows: ((await api.listSnapshots()) || []).map(mapSnapshot) }),
  backups: async () => ({ rows: ((await api.listBackups()) || []).map(mapBackup) }),
  images: async () => {
    const [rows, extra] = await Promise.all([
      api.listImages(),
      extraRows('images', () => api.listDataSources(), mapDataSource),
    ]);
    return { rows: (rows || []).map(mapImage), extra };
  },
  networks: async () => ({ rows: ((await api.listNads('all')) || []).map(mapNetwork) }),
  templates: async () => {
    const catalogP = api.listCatalogTemplates().catch(() => null);
    const builtinP = api.listTemplates().catch(() => null);
    const extraP = extraRows('templates', () => api.listCatalogProfiles(), mapProfile);
    const [catalog, builtin] = await Promise.all([catalogP, builtinP]);
    const sizing = Object.fromEntries((builtin || []).map((t) => [t.name, t]));
    const raw = catalog?.length ? catalog : builtin;
    const rows = (raw || []).map((t, i) =>
      t.family != null || t.tags != null ? mapCatalogTemplate(t, i, sizing) : mapTemplate(t, i),
    );
    return { rows, extra: await extraP };
  },
  migrations: async () => ({ rows: ((await api.listMigrations()) || []).map(mapMigration) }),
  alerts: async () => ({ rows: ((await api.listAlerts()) || []).map(mapAlert) }),
  soc: async () => ({ rows: ((await api.listSocDetections()) || []).map(mapSoc) }),
  atlas: async () => {
    const status = await api.atlasStatus().catch(() => null);
    if (status && status.configured === false) return { rows: [], meta: { configured: false } };
    const [rows, extra] = await Promise.all([
      api.listAtlasVolumes(),
      extraRows('atlas', () => api.listAtlasSnapshots(), mapAtlasSnap),
    ]);
    return { rows: (rows || []).map(mapAtlasVol), extra, meta: { configured: true } };
  },
};

export const ALL_KEYS = Object.keys(LOADERS);
const BACKGROUND_KEYS = ALL_KEYS.filter((k) => !LAZY_KEYS.has(k));

/**
 * Progressive resource loading: every resource renders as soon as its own request
 * finishes, failures keep the previous rows and are reported in `errors`, and the
 * core set plus the current page refresh every REFRESH_MS while the tab is visible.
 */
export function useResources({ enabled, page }) {
  const [data, setData] = useState(emptyData);
  const [errors, setErrors] = useState({});
  const [pending, setPending] = useState(0);
  const [healthOk, setHealthOk] = useState(true);
  const [podSummary, setPodSummary] = useState(null);
  const [includeCompleted, setIncludeCompletedState] = useState(false);
  const healthMisses = useRef(0);
  const [notifications, setNotifications] = useState([]);
  const [lastLoaded, setLastLoaded] = useState(0);
  const loadedAt = useRef({});
  const inFlight = useRef(new Set());

  const loadKey = useCallback(async (key) => {
    const loader = LOADERS[key];
    if (!loader || inFlight.current.has(key) || !getToken()) return;
    inFlight.current.add(key);
    setPending((n) => n + 1);
    try {
      const { rows, extra, meta } = await loader();
      setData((prev) => ({ ...prev, [key]: { ...prev[key], rows, ...(extra ? { extra } : {}), ...(meta ? { meta } : {}) } }));
      setErrors((prev) => {
        if (!(key in prev)) return prev;
        const next = { ...prev };
        delete next[key];
        return next;
      });
      loadedAt.current[key] = Date.now();
      setLastLoaded(Date.now());
    } catch (e) {
      console.warn(key, e);
      setErrors((prev) => ({ ...prev, [key]: e.message || String(e) }));
    } finally {
      inFlight.current.delete(key);
      setPending((n) => n - 1);
    }
  }, []);

  const pageRef = useRef(page);
  pageRef.current = page;
  const refresh = useCallback(
    (keys = LAZY_KEYS.has(pageRef.current) ? [...BACKGROUND_KEYS, pageRef.current] : BACKGROUND_KEYS) => {
      if (!getToken()) return Promise.resolve();
      return Promise.all([
        ...keys.map(loadKey),
        api
          .health()
          .then(() => {
            healthMisses.current = 0;
            setHealthOk(true);
          })
          .catch(() => {
            healthMisses.current += 1;
            if (healthMisses.current >= 2) setHealthOk(false);
          }),
        api.podSummary().then(setPodSummary).catch(() => {}),
        api.listNotifications().then(setNotifications).catch(() => {}),
      ]);
    },
    [loadKey],
  );

  const visibleKeys = useCallback(() => {
    const keys = new Set(CORE_KEYS);
    if (LOADERS[pageRef.current]) keys.add(pageRef.current);
    return [...keys];
  }, []);

  useEffect(() => {
    if (!enabled) {
      setData(emptyData());
      setErrors({});
      loadedAt.current = {};
      return undefined;
    }
    let cancelled = false;
    refresh(visibleKeys()).then(() => {
      if (!cancelled) refresh(BACKGROUND_KEYS.filter((k) => !visibleKeys().includes(k)));
    });
    const tick = () => {
      if (document.visibilityState === 'visible') refresh(visibleKeys());
    };
    const id = setInterval(tick, REFRESH_MS);
    document.addEventListener('visibilitychange', tick);
    return () => {
      cancelled = true;
      clearInterval(id);
      document.removeEventListener('visibilitychange', tick);
    };
  }, [enabled, refresh, visibleKeys]);

  useEffect(() => {
    if (!enabled || !LOADERS[page]) return;
    if (Date.now() - (loadedAt.current[page] || 0) > REFRESH_MS) loadKey(page);
  }, [enabled, page, loadKey]);

  const setIncludeCompleted = useCallback(
    (v) => {
      pods.includeCompleted = v;
      setIncludeCompletedState(v);
      loadKey('pods');
    },
    [loadKey],
  );

  return {
    data,
    podSummary,
    includeCompleted,
    setIncludeCompleted,
    errors,
    loading: pending > 0,
    loaded: (key) => !!loadedAt.current[key],
    lastLoaded,
    healthOk,
    notifications,
    setNotifications,
    refresh,
    refreshVisible: () => refresh(visibleKeys()),
  };
}
