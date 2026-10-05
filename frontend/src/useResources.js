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
export const CORE_KEYS = ['vms', 'hosts', 'pods', 'pvcs', 'alerts', 'templates'];

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
  pods: async () => ({ rows: ((await api.listPods('all')) || []).map(mapPod) }),
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
    const extraP = extraRows('templates', () => api.listCatalogProfiles(), mapProfile);
    const catalog = await catalogP;
    const raw = catalog?.length ? catalog : await api.listTemplates();
    const rows = (raw || []).map((t, i) =>
      t.family != null || t.tags != null ? mapCatalogTemplate(t, i) : mapTemplate(t, i),
    );
    return { rows, extra: await extraP };
  },
  migrations: async () => ({ rows: ((await api.listMigrations()) || []).map(mapMigration) }),
  alerts: async () => ({ rows: ((await api.listAlerts()) || []).map(mapAlert) }),
  soc: async () => ({ rows: ((await api.listSocDetections()) || []).map(mapSoc) }),
  atlas: async () => {
    const [rows, extra] = await Promise.all([
      api.listAtlasVolumes(),
      extraRows('atlas', () => api.listAtlasSnapshots(), mapAtlasSnap),
    ]);
    return { rows: (rows || []).map(mapAtlasVol), extra };
  },
};

export const ALL_KEYS = Object.keys(LOADERS);

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
      const { rows, extra } = await loader();
      setData((prev) => ({ ...prev, [key]: { ...prev[key], rows, ...(extra ? { extra } : {}) } }));
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

  const refresh = useCallback(
    (keys = ALL_KEYS) => {
      if (!getToken()) return Promise.resolve();
      return Promise.all([
        ...keys.map(loadKey),
        api.health().then(() => setHealthOk(true)).catch(() => setHealthOk(false)),
        api.listNotifications().then(setNotifications).catch(() => {}),
      ]);
    },
    [loadKey],
  );

  const pageRef = useRef(page);
  pageRef.current = page;
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
      if (!cancelled) refresh(ALL_KEYS.filter((k) => !visibleKeys().includes(k)));
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

  return {
    data,
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
