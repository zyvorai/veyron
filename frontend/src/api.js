const TOKEN_STORAGE = 'veyron_auth_token';
const USER_STORAGE = 'veyron_auth_user';
const THEME_STORAGE = 'veyron_console_theme';

export function getToken() {
  try {
    return localStorage.getItem(TOKEN_STORAGE) || '';
  } catch {
    return '';
  }
}

export function setToken(token) {
  try {
    if (token) localStorage.setItem(TOKEN_STORAGE, token);
    else localStorage.removeItem(TOKEN_STORAGE);
  } catch {
    /* ignore */
  }
}

export function getAuthUser() {
  try {
    const raw = localStorage.getItem(USER_STORAGE);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function setAuthUser(user) {
  try {
    if (user) localStorage.setItem(USER_STORAGE, JSON.stringify(user));
    else localStorage.removeItem(USER_STORAGE);
  } catch {
    /* ignore */
  }
}

export function clearSession() {
  setToken('');
  setAuthUser(null);
}

/** Fired when the API rejects the session, so the shell can sign out immediately. */
export const UNAUTHORIZED_EVENT = 'veyron:unauthorized';

function onUnauthorized() {
  clearSession();
  window.dispatchEvent(new Event(UNAUTHORIZED_EVENT));
}

/** Reads give up after 20s, mutations after 120s; pass `timeout: 0` to disable. */
const READ_TIMEOUT_MS = 20_000;
const WRITE_TIMEOUT_MS = 120_000;
const HEALTH_TIMEOUT_MS = 5_000;

export function getSavedTheme() {
  try {
    return localStorage.getItem(THEME_STORAGE) || '';
  } catch {
    return '';
  }
}

export function setSavedTheme(theme) {
  try {
    if (theme) localStorage.setItem(THEME_STORAGE, theme);
    else localStorage.removeItem(THEME_STORAGE);
  } catch {
    /* ignore */
  }
}

function httpError(message, status) {
  const e = new Error(message);
  e.status = status;
  return e;
}

/** Netra-style sign-in errors: never show raw server or browser text. */
export function loginErrorMessage(e) {
  if (!e) return '';
  if (e.status === 401 || e.status === 403) return 'Wrong username or password.';
  if (e.network || e.status >= 500 || e.status === 404) return 'Could not reach Veyron. Check the URL and try again.';
  return /invalid|wrong|unauthori[sz]ed|credential/i.test(e.message || '')
    ? 'Wrong username or password.'
    : 'Could not reach Veyron. Check the URL and try again.';
}

async function request(path, opts = {}) {
  const { skipAuth, rawText, timeout, ...fetchOpts } = opts;
  const token = getToken();
  const headers = Object.assign({}, fetchOpts.headers || {});
  if (token && !skipAuth) headers.Authorization = `Bearer ${token}`;
  if (fetchOpts.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';

  const method = (fetchOpts.method || 'GET').toUpperCase();
  const limit = timeout ?? (method === 'GET' ? READ_TIMEOUT_MS : WRITE_TIMEOUT_MS);
  const ctrl = new AbortController();
  const timer = limit ? setTimeout(() => ctrl.abort(), limit) : null;
  let resp;
  let text;
  try {
    resp = await fetch(path, { ...fetchOpts, headers, signal: fetchOpts.signal || ctrl.signal });
    text = await resp.text();
  } catch (e) {
    const err = e.name === 'AbortError' ? new Error(`Timed out after ${Math.round(limit / 1000)}s`) : e;
    err.network = true;
    throw err;
  } finally {
    if (timer) clearTimeout(timer);
  }
  if (rawText) {
    if (resp.status === 401 && !skipAuth) onUnauthorized();
    if (!resp.ok) throw httpError(text.slice(0, 200) || `HTTP ${resp.status}`, resp.status);
    return text;
  }
  let body = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      if (!resp.ok) throw httpError(text.slice(0, 200) || `HTTP ${resp.status}`, resp.status);
      throw new Error('Invalid JSON from API');
    }
  }
  if (resp.status === 401 && !skipAuth) {
    onUnauthorized();
  }
  if (!resp.ok) {
    const msg =
      (body && (typeof body.error === 'string' ? body.message || body.error : body.error?.message ?? body.message)) ||
      `HTTP ${resp.status}`;
    throw httpError(String(msg), resp.status);
  }
  if (body && typeof body === 'object' && body.success === false) {
    throw new Error(String(body.error?.message ?? body.message ?? 'Request failed'));
  }
  return body;
}

export function unwrap(j) {
  if (j == null) return j;
  if (typeof j === 'object' && 'success' in j && j.success === true && 'data' in j) return j.data;
  return j;
}

export function asArray(j) {
  const u = unwrap(j);
  if (Array.isArray(u)) return u;
  if (u && Array.isArray(u.items)) return u.items;
  if (u && Array.isArray(u.vms)) return u.vms;
  if (u && Array.isArray(u.nodes)) return u.nodes;
  if (u && Array.isArray(u.pods)) return u.pods;
  if (u && Array.isArray(u.pvcs)) return u.pvcs;
  if (u && Array.isArray(u.snapshots)) return u.snapshots;
  if (u && Array.isArray(u.backups)) return u.backups;
  if (u && Array.isArray(u.templates)) return u.templates;
  if (u && Array.isArray(u.classes)) return u.classes;
  if (u && Array.isArray(u.nads)) return u.nads;
  if (u && Array.isArray(u.images)) return u.images;
  if (u && Array.isArray(u.catalog)) return u.catalog;
  if (u && Array.isArray(u.integrations)) return u.integrations;
  if (u && Array.isArray(u.notifications)) return u.notifications;
  return [];
}

function nsQ(ns) {
  return ns && ns !== 'all' ? `?namespace=${encodeURIComponent(ns)}` : '?namespace=all';
}

/** API sends vCPU as a number or a label like "2 cores". */
export function parseCount(v) {
  const n = parseInt(String(v ?? ''), 10);
  return Number.isFinite(n) ? n : '—';
}

export function parseMemGi(v) {
  if (v == null) return 0;
  if (typeof v === 'number') return v > 64 ? Math.round(v / 1024) : v;
  const s = String(v);
  const m = s.match(/([\d.]+)\s*(Gi|G|Mi|M|Ki|K)?/i);
  if (!m) return Number(s) || 0;
  const n = parseFloat(m[1]);
  const u = (m[2] || 'Gi').toLowerCase();
  if (u.startsWith('mi') || u === 'm') return Math.round(n / 1024);
  if (u.startsWith('ki') || u === 'k') return Math.round(n / 1024 / 1024);
  return Math.round(n);
}

function parseDiskGb(v) {
  if (v == null) return '—';
  if (typeof v === 'number') return v;
  const s = String(v).trim();
  const m = s.match(/^([\d.]+)\s*(Gi|G|Ti|T|Mi)?$/i);
  if (!m) return '—';
  const n = parseFloat(m[1]);
  const u = (m[2] || 'G').toLowerCase();
  if (u.startsWith('ti') || u === 't') return Math.round(n * 1024);
  if (u.startsWith('mi')) return Math.round(n / 1024);
  return Math.round(n);
}

/** API placeholders for "no value" ("N/A", "", "<none>") become a dash. */
function present(v) {
  if (v == null) return '—';
  const s = String(v).trim();
  return !s || /^(n\/a|none|<none>|unknown|null)$/i.test(s) ? '—' : v;
}

/** "2026-10-06T04:42:06Z" → "12m ago"; anything unparseable is returned as-is. */
export function relTime(v) {
  if (!v || v === '—') return '—';
  const t = Date.parse(v);
  if (!Number.isFinite(t)) return v;
  const s = Math.max(0, Math.round((Date.now() - t) / 1000));
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86400)}d ago`;
}

function ageOf(obj) {
  return obj?.uptime || obj?.age || obj?.creation_timestamp || obj?.created || '—';
}

function fmtBytes(n) {
  if (n == null || Number.isNaN(n)) return '—';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function mapVm(raw, i) {
  const name = raw.name || raw.metadata?.name || `vm-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  const status = raw.status || raw.phase || raw.printables_status || 'Unknown';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    managed: raw.veyron_managed !== false,
    status: String(status).replace(/^VMI?Phase/, '') || 'Unknown',
    cpu: parseCount(raw.cpu ?? raw.cpus ?? raw.vcpus ?? raw.spec?.cpu),
    ram: parseMemGi(raw.memory || raw.ram || raw.memory_gi),
    disk: parseDiskGb(raw.disk_gb ?? raw.disk ?? raw.storage),
    diskSrc: present(raw.disk),
    host: present(raw.node || raw.host || raw.node_name),
    ip: present(raw.ip || raw.ips?.[0] || raw.interfaces?.[0]?.ipAddress),
    age: ageOf(raw),
    os: raw.os || raw.guest_os || raw.template || '—',
  };
}

/** Usage percentage, or null when the API has no measurement (never a fake 0). */
function pctOrNull(v) {
  if (v == null || v === '') return null;
  const n = Number(v);
  return Number.isFinite(n) ? Math.round(n) : null;
}

export function mapNode(raw, i) {
  const name = raw.name || raw.metadata?.name || `node-${i}`;
  const ready = raw.ready === true || raw.status === 'Ready' || raw.ready === undefined;
  const unschedulable = raw.unschedulable === true || raw.spec?.unschedulable === true;
  return {
    id: name,
    name,
    status: unschedulable ? 'Cordoned' : ready ? 'Ready' : String(raw.status || 'NotReady'),
    cpu: pctOrNull(raw.cpu_percent ?? raw.cpu_usage),
    mem: pctOrNull(raw.memory_percent ?? raw.mem_usage),
    vms: raw.vm_count ?? raw.vms ?? raw.running_vms ?? '—',
    kernel: raw.kernel || raw.kernel_version || '—',
    age: ageOf(raw),
    cores: raw.cpu_capacity || raw.cores || '—',
    memTotal: raw.memory_capacity || raw.mem_total || '—',
    cordoned: unschedulable,
  };
}

export function mapPod(raw, i) {
  const name = raw.name || raw.metadata?.name || `pod-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    status: raw.status || raw.phase || 'Unknown',
    ready: raw.ready || raw.ready_str || '—',
    restarts: raw.restarts ?? raw.restart_count ?? 0,
    node: raw.node || raw.node_name || '—',
    age: ageOf(raw),
    image: raw.image || raw.images?.[0] || '—',
  };
}

export function mapPvc(raw, i) {
  const name = raw.name || raw.metadata?.name || `pvc-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    status: raw.status || raw.phase || 'Unknown',
    cls: raw.storage_class || raw.storageClassName || raw.class || '—',
    size: raw.size || raw.capacity || raw.request || '—',
    used: pctOrNull(raw.used_percent ?? raw.used),
    access: raw.access_modes?.[0] || raw.access || '—',
    vol: raw.volume_name || raw.volume || '—',
  };
}

export function mapClass(raw, i) {
  const name = raw.name || raw.metadata?.name || `sc-${i}`;
  return {
    id: name,
    name,
    cls: raw.is_default ? `${name} (default)` : name,
    prov: raw.provisioner || '—',
    total: raw.capacity || raw.total || '—',
    used: pctOrNull(raw.used_percent),
    vols: raw.volume_count ?? raw.vols ?? '—',
  };
}

export function mapSnapshot(raw, i) {
  const name = raw.name || raw.metadata?.name || `snap-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    vm: raw.vm || raw.vm_name || raw.source_vm || raw.virtual_machine || '—',
    size: raw.size || '—',
    age: ageOf(raw),
    status: raw.status || raw.ready ? 'Ready' : raw.phase || 'Unknown',
  };
}

export function mapBackup(raw, i) {
  const name = raw.name || raw.id || `backup-${i}`;
  const ns = raw.namespace || 'default';
  const vm = raw.vm_name || raw.vm || '—';
  return {
    id: String(name),
    name,
    ns,
    vm,
    target: raw.target || raw.location || raw.bucket || vm,
    items: raw.items ?? raw.item_count ?? '—',
    size: raw.size || (raw.size_bytes != null ? `${Math.round(raw.size_bytes / 1024 / 1024)} Mi` : '—'),
    age: ageOf(raw) || raw.created_at || raw.last_run || '—',
    status: raw.status || 'Healthy',
    sched: raw.schedule || '—',
    ret: raw.retention || '—',
  };
}

export function mapImage(raw, i) {
  const name = raw.name || raw.metadata?.name || `img-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    type: raw.type || raw.format || raw.kind || '—',
    size: raw.size || raw.capacity || '—',
    os: raw.os || raw.guest_os || '—',
    age: ageOf(raw) || raw.age || '—',
    src: raw.source || raw.url || raw.source_type || '—',
  };
}

export function mapNetwork(raw, i) {
  const name = raw.name || raw.metadata?.name || `net-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    type: raw.type || raw.cni || 'NAD',
    cidr: raw.cidr || raw.config?.cidr || '—',
    vlan: raw.vlan ?? '—',
    attached: raw.attached ?? raw.attachments ?? '—',
    status: raw.status || 'Up',
    gw: raw.gateway || '—',
  };
}

export function mapTemplate(raw, i) {
  const name = raw.name || raw.id || `tpl-${i}`;
  return {
    id: name,
    name,
    os: raw.os || raw.guest_os || raw.description || '—',
    cpu: raw.cpu ?? raw.cpus ?? '—',
    ram: parseMemGi(raw.memory || raw.ram),
    disk: parseDiskGb(raw.disk || raw.storage),
    uses: raw.uses ?? '—',
    ci: raw.cloud_init ? 'yes' : raw.cloud_init === false ? 'no' : '—',
  };
}

export function wsUrl(path) {
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${window.location.host}${path}`;
}

export const api = {
  login: async (username, password) => {
    const body = await request('/api/v1/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
      skipAuth: true,
    });
    const data = unwrap(body) || body;
    if (!data?.token) throw new Error('Login response missing token');
    setToken(data.token);
    setAuthUser({
      username: username,
      display_name: data.display_name || username,
      role: data.role || 'readonly',
    });
    return data;
  },
  health: () => request('/api/v1/health', { skipAuth: true, timeout: HEALTH_TIMEOUT_MS }).then(unwrap),
  listVms: (ns = 'all') => request(`/api/v1/vms${nsQ(ns)}`).then(asArray),
  listNodes: () => request('/api/v1/nodes').then(asArray),
  listPods: (ns = 'all', { active = false } = {}) => request(`/api/v1/pods${nsQ(ns)}${active ? '&active=true' : ''}`).then(asArray),
  podSummary: () => request('/api/v1/pods/summary?namespace=all').then(unwrap),
  listPvcs: (ns = 'all') => request(`/api/v1/storage/pvcs${nsQ(ns)}`).then(asArray),
  listStorageClasses: () => request('/api/v1/storage/classes').then(asArray),
  listSnapshots: () => request('/api/v1/snapshots?namespace=all').then(asArray),
  listBackups: () => request('/api/v1/backups?namespace=all').then(asArray),
  listImages: () => request('/api/v1/images/catalog').then(asArray),
  listNads: (ns = 'all') => request(`/api/v1/network/nads${nsQ(ns)}`).then(asArray),
  listTemplates: () => request('/api/v1/templates').then(asArray),
  vmAction: (ns, name, action) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/${encodeURIComponent(action)}`, {
      method: 'POST',
    }),
  cloneVm: (ns, name, newName) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/clone`, {
      method: 'POST',
      body: JSON.stringify({ new_name: newName }),
    }),
  deleteVm: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  createVm: (body) => request('/api/v1/vms', { method: 'POST', body: JSON.stringify(body) }),
  createSnapshot: (ns, vm, snapshotName) =>
    request(`/api/v1/snapshots/${encodeURIComponent(ns)}/${encodeURIComponent(vm)}/create`, {
      method: 'POST',
      body: JSON.stringify(snapshotName ? { snapshot_name: snapshotName } : {}),
    }),
  restoreSnapshot: (ns, name) =>
    request(`/api/v1/snapshots/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/restore`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  deleteSnapshot: (ns, name) =>
    request(`/api/v1/snapshots/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/delete`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  createBackup: ({ vm_name, name, compress = false, encrypt = false }) =>
    request('/api/v1/backups', {
      method: 'POST',
      body: JSON.stringify({ vm_name, name, compress, encrypt }),
    }),
  restoreBackup: (id) =>
    request(`/api/v1/backups/${encodeURIComponent(id)}/restore`, { method: 'POST', body: JSON.stringify({}) }),
  deleteBackup: (id) =>
    request(`/api/v1/backups/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  resizePvc: (ns, name, newSize) =>
    request(`/api/v1/storage/pvcs/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, {
      method: 'PATCH',
      body: JSON.stringify({ new_size: newSize }),
    }),
  createPvc: ({ name, namespace = 'default', size, storage_class }) =>
    request('/api/v1/storage/pvcs', {
      method: 'POST',
      body: JSON.stringify({ name, namespace, size, storage_class }),
    }),
  deletePvc: (ns, name) =>
    request(`/api/v1/storage/pvcs/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  cordonNode: (name) => request(`/api/v1/nodes/${encodeURIComponent(name)}/cordon`, { method: 'POST' }),
  uncordonNode: (name) => request(`/api/v1/nodes/${encodeURIComponent(name)}/uncordon`, { method: 'POST' }),
  rebootNode: (name) =>
    request(`/api/v1/nodes/${encodeURIComponent(name)}/reboot`, {
      method: 'POST',
      body: JSON.stringify({ confirm: true }),
    }),
  deletePod: (ns, name) =>
    request(`/api/v1/pods/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  deleteImage: (ns, name) =>
    request(`/api/v1/images/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  importImage: (body) => request('/api/v1/images/import', { method: 'POST', body: JSON.stringify(body) }),
  createNad: (body) => request('/api/v1/network/nads', { method: 'POST', body: JSON.stringify(body) }),
  deleteNad: (ns, name) =>
    request(`/api/v1/network/nads/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  deleteTemplate: (name) =>
    request(`/api/v1/crds/templates/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  podLogs: async (name, ns) => {
    const body = await request(
      `/api/v1/pods/${encodeURIComponent(name)}/logs?namespace=${encodeURIComponent(ns || 'default')}`,
    );
    const data = unwrap(body);
    if (typeof data === 'string') return data;
    if (Array.isArray(data)) {
      return data
        .map((e) => {
          const ts = e.timestamp ? `${e.timestamp} ` : '';
          return `${ts}${e.message || e.line || JSON.stringify(e)}`;
        })
        .join('\n');
    }
    if (data?.logs) return typeof data.logs === 'string' ? data.logs : JSON.stringify(data.logs, null, 2);
    return JSON.stringify(data, null, 2);
  },
  wsTicket: async () => {
    const body = await request('/api/v1/ws/ticket', { method: 'POST', body: JSON.stringify({}) });
    const data = unwrap(body) || body;
    if (!data?.ticket) throw new Error('Missing WS ticket');
    return data;
  },
  listNotifications: () => request('/api/v1/notifications?namespace=all').then(asArray),
  integrationsStatus: async () => {
    const body = await request('/api/v1/integrations/status');
    return unwrap(body) || body;
  },
  platformVersions: async () => unwrap(await request('/api/v1/platform/versions')) || {},
  platformCapabilities: async () => unwrap(await request('/api/v1/platform/capabilities')) || {},
  getSelfHealingPolicy: async () => unwrap(await request('/api/v1/self-healing/policy')) || {},
  setSelfHealingPolicy: (policy) =>
    request('/api/v1/self-healing/policy', { method: 'POST', body: JSON.stringify(policy) }),
  vmEvents: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/events`).then(asArray),
  vmMetrics: async (vm, ns) => {
    const q = ns ? `?namespace=${encodeURIComponent(ns)}` : '';
    return unwrap(await request(`/api/v1/metrics/${encodeURIComponent(vm)}${q}`)) || null;
  },
  getExpose: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/expose`).then(unwrap),
  putExpose: (ns, name, body) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/expose`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),
  deleteExpose: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/expose`, {
      method: 'DELETE',
    }),
  getRdpExpose: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/rdp-expose`).then(unwrap),
  putRdpExpose: (ns, name, body) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/rdp-expose`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),
  deleteRdpExpose: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/rdp-expose`, {
      method: 'DELETE',
    }),
  hotplugVm: (ns, name, { sockets, memory } = {}) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/hotplug`, {
      method: 'POST',
      body: JSON.stringify({ sockets, memory }),
    }),
  setRunStrategy: (ns, name, strategy) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/run-strategy`, {
      method: 'PUT',
      body: JSON.stringify({ strategy }),
    }),
  guestSoftReboot: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest/softreboot`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  guestFreeze: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest/freeze`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  guestUnfreeze: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest/unfreeze`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  getGuestStatus: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest/status`).then(unwrap),
  bulkVmAction: (namespace, names, action) =>
    request('/api/v1/vms/bulk', {
      method: 'POST',
      body: JSON.stringify({ namespace, names, action }),
    }),

  getDrift: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/drift`).then(unwrap),
  remediateDrift: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/drift/remediate`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  getInternet: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/network/internet`).then(unwrap),
  putInternet: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/network/internet`, {
      method: 'PUT',
      body: JSON.stringify({}),
    }),
  deleteInternet: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/network/internet`, {
      method: 'DELETE',
    }),
  guestPatch: (ns, name, body = {}) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest/patch`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  disksReclaim: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/disks/reclaim`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  enableRdpGuest: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest-agent/enable-rdp`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  disableRdpGuest: (ns, name) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/guest-agent/disable-rdp`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  runSelfHealing: (heal = false) =>
    request(`/api/v1/self-healing/run?heal=${heal ? 'true' : 'false'}`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  markNotificationsRead: (ids) =>
    request('/api/v1/notifications/read', {
      method: 'POST',
      body: JSON.stringify({ notification_ids: ids }),
    }),
  publishImage: (body) => request('/api/v1/images/publish', { method: 'POST', body: JSON.stringify(body) }),
  listDataSources: () => request('/api/v1/images/datasources').then(asArray),
  listCatalogTemplates: async () => {
    const body = await request('/api/v1/crds/templates');
    const data = unwrap(body) || body;
    if (Array.isArray(data)) return data;
    if (Array.isArray(data?.items)) return data.items;
    return asArray(body);
  },
  listCatalogProfiles: async () => {
    const body = await request('/api/v1/crds/profiles');
    const data = unwrap(body) || body;
    if (Array.isArray(data)) return data;
    if (Array.isArray(data?.items)) return data.items;
    return asArray(body);
  },
  listGpus: async () => {
    const body = await request('/api/v1/gpus');
    const data = unwrap(body) || body;
    return data;
  },
  listAlerts: () => request('/api/v1/alerts?namespace=all').then(asArray),
  resolveAlert: (id) =>
    request(`/api/v1/alerts/${encodeURIComponent(id)}/resolve`, { method: 'PUT', body: JSON.stringify({}) }),
  listSocDetections: () => request('/api/v1/soc/detections').then(asArray),
  ackSocDetection: (id) =>
    request(`/api/v1/soc/detections/${encodeURIComponent(id)}/ack`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  atlasStatus: async () => unwrap(await request('/api/v1/atlas/status')) || {},
  listAtlasVolumes: () => request('/api/v1/atlas/volumes').then(asArray),
  listAtlasSnapshots: () => request('/api/v1/atlas/snapshots').then(asArray),
  restoreAtlasSnapshot: (id) =>
    request(`/api/v1/atlas/snapshots/${encodeURIComponent(id)}/restore`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  cloneAtlasSnapshot: (id) =>
    request(`/api/v1/atlas/snapshots/${encodeURIComponent(id)}/clone`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  deleteAtlasSnapshot: (id) =>
    request(`/api/v1/atlas/snapshots/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  atlasVmCephSnapshot: (ns, name) =>
    request(`/api/v1/atlas/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/ceph-snapshot`, {
      method: 'POST',
      body: JSON.stringify({}),
    }),
  listMigrations: () => request('/api/v1/migrations').then(asArray),
  cancelMigration: (id) =>
    request(`/api/v1/migrations/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  capacityHeadroom: async () => unwrap(await request('/api/v1/capacity/headroom')) || {},
  topologyMap: async () => unwrap(await request('/api/v1/topology/map')) || {},
  veleroStatus: async () => unwrap(await request('/api/v1/velero/status')) || {},
  createVeleroBackup: (body) =>
    request('/api/v1/velero/backups', { method: 'POST', body: JSON.stringify(body) }),
  createVeleroRestore: (body) =>
    request('/api/v1/velero/restores', { method: 'POST', body: JSON.stringify(body) }),
  drFailback: (body) => request('/api/v1/dr/failback', { method: 'POST', body: JSON.stringify(body) }),
  netraStatus: async () => unwrap(await request('/api/v1/netra/status')) || {},
  netraFlowSummary: async (ns = 'all') =>
    unwrap(await request(`/api/v1/netra/flows/summary?namespace=${encodeURIComponent(ns)}`)) || {},
  netraVms: async (ns = 'all') =>
    asArray(await request(`/api/v1/netra/vms?namespace=${encodeURIComponent(ns)}`)),
  paqtraStatus: async () => unwrap(await request('/api/v1/paqtra/status')) || {},
  paqtraFlows: async (verdict = '') =>
    unwrap(await request(`/api/v1/paqtra/flows?limit=50${verdict ? `&verdict=${verdict}` : ''}`))?.flows || [],
  paqtraDrops: async () => unwrap(await request('/api/v1/paqtra/drops'))?.drops || [],
  paqtraPosture: async () => unwrap(await request('/api/v1/paqtra/posture')) || {},

  // Operations
  costs: async () => listOf(await request('/api/v1/costs?namespace=all'), 'costs'),
  costSummary: async () => unwrap(await request('/api/v1/costs/summary?namespace=all')) || {},
  costForecast: async () => unwrap(await request('/api/v1/costs/forecast?namespace=all')) || {},
  listBudgets: async () => listOf(await request('/api/v1/costs/budgets'), 'budgets'),
  createBudget: (body) => request('/api/v1/costs/budgets', { method: 'POST', body: JSON.stringify(body) }),
  deleteBudget: (name) => request(`/api/v1/costs/budgets/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  listRecommendations: async () => listOf(await request('/api/v1/recommendations?namespace=all'), 'recommendations'),
  sloObjectives: async () => listOf(await request('/api/v1/slo/objectives?namespace=all'), 'objectives'),
  sloBurnRate: async () => listOf(await request('/api/v1/slo/burn-rate?namespace=all'), 'burn_rates'),
  recentEvents: async () => listOf(await request('/api/v1/events/recent?namespace=all'), 'events'),
  incidents: async () => unwrap(await request('/api/v1/incidents/timeline?namespace=all')) || {},
  logs: async () => unwrap(await request('/api/v1/logs?namespace=all')) || {},
  queryLogs: async ({ search = '', level = '', limit = 200 } = {}) => {
    const p = new URLSearchParams({ namespace: 'all', limit: String(limit) });
    if (search) p.set('search', search);
    if (level) p.set('level', level);
    return listOf(await request(`/api/v1/logs/query?${p}`), 'entries');
  },

  // Security
  complianceStatus: async () => listOf(await request('/api/v1/compliance/status?namespace=all'), 'frameworks'),
  complianceReports: async () => listOf(await request('/api/v1/compliance/reports?namespace=all'), 'reports'),
  auditTrail: async () => listOf(await request('/api/v1/audit/trail?namespace=all'), 'entries'),
  auditStats: async () => unwrap(await request('/api/v1/audit/stats?namespace=all')) || {},
  securityFindings: async () => listOf(await request('/api/v1/security/findings?namespace=all'), 'findings'),
  securityPosture: async () => unwrap(await request('/api/v1/security/posture?namespace=all')) || {},
  listUsers: async () => listOf(await request('/api/v1/auth/users'), 'users'),
  createUser: (body) => request('/api/v1/auth/users', { method: 'POST', body: JSON.stringify(body) }),
  deleteUser: (username) => request(`/api/v1/auth/users/${encodeURIComponent(username)}`, { method: 'DELETE' }),
  setUserPassword: (username, password) =>
    request(`/api/v1/auth/users/${encodeURIComponent(username)}/password`, {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),
  rbacRoles: async () => listOf(await request('/api/v1/rbac/roles?namespace=all'), 'roles'),
  rbacBindings: async () => listOf(await request('/api/v1/rbac/bindings?namespace=all'), 'bindings'),
  socHunts: async () => listOf(await request('/api/v1/soc/hunts'), 'hunts'),
  runSocHunt: (body) => request('/api/v1/soc/hunts/run', { method: 'POST', body: JSON.stringify(body) }),
  socPlaybooks: async () => listOf(await request('/api/v1/soc/playbooks'), 'playbooks'),
  triggerSocPlaybook: (id, eventType) =>
    request(`/api/v1/soc/playbooks/${encodeURIComponent(id)}/trigger`, {
      method: 'POST',
      body: JSON.stringify({ playbook_id: id, event_type: eventType }),
    }),
  socAttackSurface: async () => unwrap(await request('/api/v1/soc/attack-surface?namespace=all')) || {},
  socExportStatus: async () => listOf(await request('/api/v1/soc/export/status'), 'exporters'),
  socEvents: async () => listOf(await request('/api/v1/soc/events?limit=50'), 'events'),

  // Platform
  listOperators: async () => listOf(await request('/api/v1/operators?namespace=all'), 'operators'),
  listHelmReleases: async () => listOf(await request('/api/v1/helm/releases?namespace=all'), 'releases'),
  listNamespaces: async () => listOf(await request('/api/v1/namespaces'), 'namespaces'),
  listQuotas: async () => listOf(await request('/api/v1/quotas?namespace=all'), 'quotas'),
  gitopsStatus: async () => unwrap(await request('/api/v1/gitops/status?namespace=all')) || {},
  gitopsSync: (body = { force: false, dry_run: true }) =>
    request('/api/v1/gitops/sync', { method: 'POST', body: JSON.stringify(body) }).then(unwrap),
  catalogStatus: async () => unwrap(await request('/api/v1/catalog/status')) || {},
  catalogSync: () => request('/api/v1/catalog/sync', { method: 'POST' }).then(unwrap),
  listCustomResources: async () =>
    listOf(await request('/api/v1/custom-resources', { timeout: 45_000 }), 'resources'),
  listVeyronCrs: async (kind) => listOf(await request(`/api/v1/crds/${kind}?namespace=all`), 'items'),
  listClusters: async () => unwrap(await request('/api/v1/clusters')) || {},
  listWorkloads: async () => listOf(await request('/api/v1/workloads?namespace=all'), 'workloads'),

  // Storage & network
  storageUsage: async () => listOf(await request('/api/v1/storage/usage?namespace=all'), 'usage'),
  storagePools: async () => listOf(await request('/api/v1/storage/pools'), 'pools'),
  storageOrphans: async () => listOf(await request('/api/v1/storage/orphans?namespace=all'), 'orphans'),
  reclaimOrphans: async ({ namespace = 'all', confirm = false } = {}) =>
    unwrap(
      await request(`/api/v1/storage/orphans?namespace=${encodeURIComponent(namespace)}${confirm ? '&confirm=true' : ''}`, {
        method: 'DELETE',
      }),
    ) || {},
  listSnapshotSchedules: async () => listOf(await request('/api/v1/snapshot-schedules?namespace=all'), 'schedules'),
  createSnapshotSchedule: (body) =>
    request('/api/v1/snapshot-schedules', { method: 'POST', body: JSON.stringify(body) }),
  deleteSnapshotSchedule: (ns, name) =>
    request(`/api/v1/snapshot-schedules/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  ciliumStatus: async () => unwrap(await request('/api/v1/cilium/status')) || {},
  ciliumPolicies: async () => listOf(await request('/api/v1/cilium/policies?namespace=all'), 'policies'),
  ciliumFlows: async () => unwrap(await request('/api/v1/cilium/flows?namespace=all')) || {},
  networkPolicies: async () => listOf(await request('/api/v1/network-policies?namespace=all'), 'policies'),
  listIngresses: async () => listOf(await request('/api/v1/ingress?namespace=all'), 'ingresses'),

  // Per-VM extras
  guestDoctor: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/guest/doctor`, { timeout: 45_000 })),
  guestEvidence: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/guest/evidence`, { timeout: 45_000 })),
  guestFixPlan: async (ns, name) =>
    unwrap(await request(`${vmPath(ns, name)}/guest/fix-plan`, { method: 'POST', body: '{}' })),
  guestMetrics: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/guest/metrics`, { timeout: 45_000 })),
  guestMigrateScore: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/guest/migrate-score`, { timeout: 45_000 })),
  guestFilesystem: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/guest-filesystem`)) || {},
  vmSecurity: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/security`)) || {},
  vmMigrations: async (ns, name) => listOf(await request(`${vmPath(ns, name)}/migrations`), 'migrations'),
  cancelVmMigration: (ns, name, mig) =>
    request(`${vmPath(ns, name)}/migrations/${encodeURIComponent(mig)}`, { method: 'DELETE' }),
  vmVolumes: async (ns, name) => listOf(await request(`${vmPath(ns, name)}/volumes/status`), 'volumes'),
  hotplugVolume: (ns, name, volumeName, pvcName) =>
    request(`${vmPath(ns, name)}/volumes/hotplug`, {
      method: 'POST',
      body: JSON.stringify({ volume_name: volumeName, pvc_name: pvcName }),
    }),
  hotremoveVolume: (ns, name, volumeName) =>
    request(`${vmPath(ns, name)}/volumes/hotremove`, { method: 'POST', body: JSON.stringify({ volume_name: volumeName }) }),
  dataDiskDefaults: async (ns, name) => unwrap(await request(`${vmPath(ns, name)}/storage/data-disk/defaults`)) || {},
  addDataDisk: async (ns, name, body) =>
    unwrap(await request(`${vmPath(ns, name)}/storage/data-disk`, { method: 'POST', body: JSON.stringify(body) })),
};

function vmPath(ns, name) {
  return `/api/v1/vms/${encodeURIComponent(ns || 'default')}/${encodeURIComponent(name)}`;
}

/** Unwraps a list endpoint that answers with either a bare array or `{ <key>: [...] }`. */
export function listOf(j, key) {
  const u = unwrap(j);
  if (Array.isArray(u)) return u;
  if (u && Array.isArray(u[key])) return u[key];
  return [];
}

const titleCase = (s) => String(s || '').replace(/[_-]+/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());

export function money(v, currency = 'USD') {
  const n = Number(v);
  if (!Number.isFinite(n)) return '—';
  return n.toLocaleString(undefined, {
    style: 'currency',
    currency,
    currencyDisplay: 'narrowSymbol',
    maximumFractionDigits: n >= 100 ? 0 : 2,
  });
}

export function mapRecommendation(raw, i) {
  return {
    id: raw.id || `rec-${i}`,
    name: raw.title || raw.id || `Recommendation ${i + 1}`,
    status: raw.priority || 'Medium',
    category: titleCase(raw.category).replace(/\s+/g, ' ') || '—',
    resource: raw.resource || '—',
    savings: raw.estimated_savings != null ? money(raw.estimated_savings) : '—',
    effort: raw.effort || '—',
    impact: raw.impact || '—',
    description: raw.description || '—',
  };
}

export function mapClusterEvent(raw, i) {
  return {
    id: `${raw.timestamp || ''}-${raw.involved_object || ''}-${i}`,
    name: raw.involved_object || raw.reason || `event-${i}`,
    status: raw.type || 'Normal',
    reason: raw.reason || '—',
    ns: raw.namespace || '—',
    message: tidyEventMessage(raw.message) || '—',
    age: relTime(raw.timestamp),
    at: raw.timestamp || '—',
  };
}

export function mapIncident(raw, i) {
  return {
    id: raw.id ? `${raw.id}-${i}` : `inc-${i}`,
    name: raw.title || raw.id || `Incident ${i + 1}`,
    status: raw.resolved ? 'Resolved' : raw.severity || 'warning',
    kind: titleCase(raw.kind) || '—',
    vm: raw.vm_name || '—',
    ns: raw.namespace || '—',
    description: raw.description || '—',
    age: relTime(raw.timestamp),
    at: raw.timestamp || '—',
  };
}

export function mapAudit(raw, i) {
  return {
    id: raw.id || `audit-${i}`,
    name: raw.resource_name || raw.action || `entry-${i}`,
    status: raw.outcome || '—',
    user: raw.user || '—',
    action: raw.action || '—',
    type: raw.resource_type || '—',
    ns: raw.namespace || '—',
    details: tidyEventMessage(raw.details) || '—',
    age: relTime(raw.timestamp),
    at: raw.timestamp || '—',
  };
}

export function mapFinding(raw, i) {
  return {
    id: raw.id || `finding-${i}`,
    name: raw.title || raw.id || `Finding ${i + 1}`,
    status: raw.severity || 'Medium',
    category: raw.category || '—',
    resource: raw.resource || '—',
    description: raw.description || '—',
    recommendation: raw.recommendation || '—',
    age: relTime(raw.detected_at),
  };
}

export function mapUser(raw) {
  return {
    id: raw.username,
    name: raw.username,
    status: 'Active',
    role: raw.role || 'readonly',
    display: raw.display_name || '—',
    age: relTime(raw.created_at),
    created: raw.created_at || '—',
  };
}

export function mapRole(raw, i) {
  const rules = Array.isArray(raw.rules) ? raw.rules : [];
  const verbs = [...new Set(rules.flatMap((r) => r.verbs || []))];
  return {
    id: `${raw.namespace || 'cluster'}/${raw.name || i}`,
    name: raw.name || `role-${i}`,
    status: raw.namespace ? 'Namespaced' : 'Cluster',
    ns: raw.namespace || 'cluster-wide',
    rules: rules.length,
    verbs: verbs.includes('*') ? 'all (*)' : verbs.slice(0, 6).join(', ') || '—',
    groups: [...new Set(rules.flatMap((r) => r.api_groups || []))].map((g) => g || 'core').slice(0, 6).join(', ') || '—',
  };
}

export function mapBinding(raw, i) {
  const subjects = Array.isArray(raw.subjects) ? raw.subjects : [];
  return {
    id: `${raw.namespace || 'cluster'}/${raw.name || i}`,
    name: raw.name || `binding-${i}`,
    role: raw.role_name || '—',
    ns: raw.namespace || 'cluster-wide',
    subjects: subjects.map((s) => `${s.kind}:${s.namespace ? `${s.namespace}/` : ''}${s.name}`).join(', ') || '—',
  };
}

export function mapOperator(raw, i) {
  const managed = Array.isArray(raw.managed_resources) ? raw.managed_resources : [];
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `operator-${i}`,
    status: raw.status || 'Unknown',
    ns: raw.namespace || '—',
    version: present(raw.version),
    managed: managed.length,
    managedList: managed.join(', ') || '—',
  };
}

export function mapHelmRelease(raw, i) {
  const chart = [raw.chart, raw.chart_version].filter(Boolean).join('-');
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `release-${i}`,
    status: raw.status || 'unknown',
    ns: raw.namespace || '—',
    chart: chart || '—',
    app: present(raw.app_version),
    revision: raw.revision ?? '—',
    age: relTime(raw.updated_at),
  };
}

export function mapQuota(raw, i) {
  const pair = (used, limit) => (limit ? `${used || '0'} / ${limit}` : '—');
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `quota-${i}`,
    ns: raw.namespace || '—',
    cpu: pair(raw.cpu_used, raw.cpu_limit),
    memory: pair(raw.memory_used, raw.memory_limit),
    vms: raw.vm_limit != null ? `${raw.vm_count ?? 0} / ${raw.vm_limit}` : '—',
  };
}

export function mapCustomResource(raw, i) {
  return {
    id: raw.name || `crd-${i}`,
    name: raw.kind || raw.name || `crd-${i}`,
    crd: raw.name || '—',
    group: raw.group || '—',
    version: raw.version || '—',
    scope: raw.scope || '—',
    count: raw.instance_count ?? 0,
  };
}

export function mapWorkload(raw, i) {
  return {
    id: `${raw.namespace || ''}/${raw.workload_type || ''}/${raw.name || i}`,
    name: raw.name || `workload-${i}`,
    status: raw.status || 'Unknown',
    type: raw.workload_type || '—',
    ns: raw.namespace || '—',
    ready: raw.replicas != null ? `${raw.ready_replicas ?? 0}/${raw.replicas}` : '—',
    cpu: present(raw.cpu_request),
    memory: present(raw.memory_request),
  };
}

export function mapOrphan(raw, i) {
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `pvc-${i}`,
    status: 'Unattached',
    ns: raw.namespace || 'default',
    sc: raw.storage_class || '—',
    size: raw.capacity || '—',
  };
}

export function mapSchedule(raw, i) {
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `schedule-${i}`,
    status: raw.enabled === false ? 'Disabled' : 'Enabled',
    ns: raw.namespace || 'default',
    vm: raw.vm_name || '—',
    cron: raw.cron || '—',
    prefix: raw.snapshot_prefix || '—',
    keep: raw.max_snapshots ? raw.max_snapshots : 'all',
    lastRun: relTime(raw.last_run),
  };
}

const selectorText = (sel) =>
  sel && typeof sel === 'object' && Object.keys(sel).length
    ? Object.entries(sel).map(([k, v]) => `${k}=${v}`).join(', ')
    : 'all pods';

export function mapCiliumPolicy(raw, i) {
  return {
    id: `${raw.namespace || 'cluster'}/${raw.name || i}`,
    name: raw.name || `policy-${i}`,
    status: raw.enforcement || 'Enabled',
    kind: raw.policy_kind || 'CNP',
    ns: raw.namespace || 'cluster-wide',
    selector: selectorText(raw.endpoint_selector),
    ingress: raw.ingress_rules ?? 0,
    egress: raw.egress_rules ?? 0,
  };
}

export function mapNetworkPolicy(raw, i) {
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `netpol-${i}`,
    ns: raw.namespace || '—',
    types: (raw.policy_types || []).join(', ') || '—',
    selector: selectorText(raw.pod_selector),
    ingress: raw.ingress_rules ?? 0,
    egress: raw.egress_rules ?? 0,
    age: relTime(raw.created_at),
  };
}

export function mapIngress(raw, i) {
  const rules = Array.isArray(raw.rules) ? raw.rules : [];
  return {
    id: `${raw.namespace || ''}/${raw.name || i}`,
    name: raw.name || `ingress-${i}`,
    ns: raw.namespace || '—',
    cls: raw.class_name || '—',
    hosts: (raw.hosts || []).join(', ') || '—',
    tls: raw.tls ? 'Yes' : 'No',
    backends: rules.map((r) => `${r.path || '/'} → ${r.service_name}:${r.service_port}`).join(', ') || '—',
    age: relTime(raw.created_at),
  };
}

export function mapGpu(raw, i) {
  const node = raw.node || raw.name || `gpu-${i}`;
  const resources = raw.resources || [];
  const first = resources[0] || {};
  const alloc = resources.reduce((a, r) => a + (Number(r.allocatable) || 0), 0);
  return {
    id: node,
    name: node,
    status: alloc > 0 ? 'Ready' : 'Empty',
    product: raw.labels?.product || first.resource_name || '—',
    memory: raw.labels?.memory || '—',
    count: alloc || raw.labels?.count || 0,
    kind: first.kind || first.classification || '—',
    resources: resources.map((r) => r.resource_name || r.name).filter(Boolean).join(', ') || '—',
  };
}

/** Kubelet event text names pods as `name_namespace(uid)`; show `namespace/name` instead. */
export function tidyEventMessage(m) {
  if (!m) return m;
  return String(m).replace(/\b([a-z0-9][a-z0-9.-]*)_([a-z0-9][a-z0-9-]*)\([0-9a-f-]{36}\)/g, '$2/$1');
}

export function mapAlert(raw, i) {
  const id = raw.id || `alert-${i}`;
  return {
    id,
    name: raw.name || raw.reason || id,
    status: raw.status || 'firing',
    severity: raw.severity || 'warning',
    message: tidyEventMessage(raw.message) || '—',
    source: raw.source || '—',
    age: relTime(raw.fired_at || raw.age),
    firedAt: raw.fired_at || '—',
  };
}

export function mapSoc(raw, i) {
  const id = raw.id || raw.detection_id || `soc-${i}`;
  return {
    id,
    name: raw.title || raw.name || raw.rule || id,
    status: raw.status || (raw.acked ? 'Acked' : 'Open'),
    severity: raw.severity || raw.level || '—',
    message: raw.message || raw.summary || '—',
    source: raw.source || raw.rule_id || '—',
    age: raw.created_at || raw.timestamp || raw.age || '—',
  };
}

export function mapAtlasVol(raw, i) {
  const id = raw.id || raw.volume_id || `vol-${i}`;
  return {
    id,
    name: raw.name || raw.pvc_name || id,
    status: raw.status || raw.state || '—',
    ns: raw.kubernetes_namespace || raw.namespace || '—',
    size: raw.size || (raw.size_bytes != null ? fmtBytes(raw.size_bytes) : raw.capacity || '—'),
    backend: raw.pool_id || raw.backend || raw.cluster_id || '—',
    pvc: raw.pvc_name || raw.pvc || '—',
  };
}

export function mapAtlasSnap(raw, i) {
  const id = raw.id || raw.snapshot_id || `asnap-${i}`;
  return {
    id,
    name: raw.name || id,
    status: raw.state || raw.status || '—',
    volume: raw.volume_id || raw.volume || '—',
    size: raw.size || (raw.size_bytes != null ? fmtBytes(raw.size_bytes) : '—'),
    age: raw.created_at || raw.age || '—',
  };
}

export function mapMigration(raw, i) {
  const id = raw.id || raw.name || `mig-${i}`;
  return {
    id,
    name: id,
    status: raw.status || 'Unknown',
    vm: raw.vm_name || raw.vm || '—',
    source: raw.source_node || '—',
    target: raw.target_node || '—',
    type: raw.migration_type || 'live',
    progress: raw.progress_percent != null ? `${raw.progress_percent}%` : '—',
    age: raw.started_at || '—',
  };
}

export function mapCatalogTemplate(raw, i, sizing) {
  const name = raw.name || `tpl-${i}`;
  const size = sizing?.[name];
  return {
    id: name,
    name,
    os: raw.family || raw.os || raw.description || '—',
    cpu: size ? (size.cpu ?? size.default_cpus ?? '—') : '—',
    ram: size ? parseMemGi(size.memory || size.default_memory) : '—',
    disk: size ? parseDiskGb(size.disk || size.default_disk_size) : '—',
    uses: '—',
    ci: '—',
    catalog: true,
    tags: (raw.tags || []).join(', ') || '—',
  };
}

export function mapDataSource(raw, i) {
  const name = raw.name || raw.metadata?.name || `ds-${i}`;
  const ns = raw.namespace || raw.metadata?.namespace || 'default';
  return {
    id: `${ns}/${name}`,
    name,
    ns,
    cls: ns,
    prov: raw.source || raw.pvc || '—',
    total: raw.size || '—',
    used: 0,
    vols: '—',
  };
}
