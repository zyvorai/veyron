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
    if (e.name === 'AbortError') throw new Error(`Timed out after ${Math.round(limit / 1000)}s`);
    throw e;
  } finally {
    if (timer) clearTimeout(timer);
  }
  if (rawText) {
    if (resp.status === 401 && !skipAuth) onUnauthorized();
    if (!resp.ok) throw new Error(text.slice(0, 200) || `HTTP ${resp.status}`);
    return text;
  }
  let body = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      if (!resp.ok) throw new Error(text.slice(0, 200) || `HTTP ${resp.status}`);
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
    throw new Error(String(msg));
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
};

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
