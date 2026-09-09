const TOKEN_STORAGE = 'veyron_auth_token';
const USER_STORAGE = 'veyron_auth_user';
const SAVED_LOGIN = 'veyron-saved-login';
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

export function getSavedUsername() {
  try {
    const raw = localStorage.getItem(SAVED_LOGIN);
    if (!raw) return '';
    const parsed = JSON.parse(raw);
    return parsed?.username || '';
  } catch {
    return '';
  }
}

export function setSavedUsername(username) {
  try {
    if (username) localStorage.setItem(SAVED_LOGIN, JSON.stringify({ username }));
    else localStorage.removeItem(SAVED_LOGIN);
  } catch {
    /* ignore */
  }
}

async function request(path, opts = {}) {
  const { skipAuth, rawText, ...fetchOpts } = opts;
  const token = getToken();
  const headers = Object.assign({}, fetchOpts.headers || {});
  if (token && !skipAuth) headers.Authorization = `Bearer ${token}`;
  if (fetchOpts.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';

  const resp = await fetch(path, { ...fetchOpts, headers });
  const text = await resp.text();
  if (rawText) {
    if (resp.status === 401 && !skipAuth) clearSession();
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
    clearSession();
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

function unwrap(j) {
  if (j == null) return j;
  if (typeof j === 'object' && 'success' in j && j.success === true && 'data' in j) return j.data;
  return j;
}

function asArray(j) {
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

function parseMemGi(v) {
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
  if (v == null) return 0;
  if (typeof v === 'number') return v;
  const s = String(v);
  const m = s.match(/([\d.]+)\s*(Gi|G|Ti|T|Mi)?/i);
  if (!m) return Number(s) || 0;
  const n = parseFloat(m[1]);
  const u = (m[2] || 'G').toLowerCase();
  if (u.startsWith('ti') || u === 't') return Math.round(n * 1024);
  if (u.startsWith('mi')) return Math.round(n / 1024);
  return Math.round(n);
}

function ageOf(obj) {
  return obj?.uptime || obj?.age || obj?.creation_timestamp || obj?.created || '—';
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
    cpu: raw.cpu ?? raw.cpus ?? raw.vcpus ?? raw.spec?.cpu ?? '—',
    ram: parseMemGi(raw.memory || raw.ram || raw.memory_gi),
    disk: parseDiskGb(raw.disk || raw.disk_gb || raw.storage),
    host: raw.node || raw.host || raw.node_name || '—',
    ip: raw.ip || raw.ips?.[0] || raw.interfaces?.[0]?.ipAddress || '—',
    age: ageOf(raw),
    os: raw.os || raw.guest_os || raw.template || '—',
  };
}

export function mapNode(raw, i) {
  const name = raw.name || raw.metadata?.name || `node-${i}`;
  const ready = raw.ready === true || raw.status === 'Ready' || raw.ready === undefined;
  const unschedulable = raw.unschedulable === true || raw.spec?.unschedulable === true;
  return {
    id: name,
    name,
    status: unschedulable ? 'Cordoned' : ready ? 'Ready' : String(raw.status || 'NotReady'),
    cpu: Math.round(Number(raw.cpu_percent ?? raw.cpu_usage ?? raw.cpu ?? 0)),
    mem: Math.round(Number(raw.memory_percent ?? raw.mem_usage ?? raw.memory ?? 0)),
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
    used: Math.round(Number(raw.used_percent ?? raw.used ?? 0)),
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
    used: Math.round(Number(raw.used_percent ?? 0)),
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
  health: () => request('/api/v1/health', { skipAuth: true }).then(unwrap),
  listVms: (ns = 'all') => request(`/api/v1/vms${nsQ(ns)}`).then(asArray),
  listNodes: () => request('/api/v1/nodes').then(asArray),
  listPods: (ns = 'all') => request(`/api/v1/pods${nsQ(ns)}`).then(asArray),
  listPvcs: (ns = 'all') => request(`/api/v1/storage/pvcs${nsQ(ns)}`).then(asArray),
  listStorageClasses: () => request('/api/v1/storage/classes').then(asArray),
  listSnapshots: () => request('/api/v1/snapshots').then(asArray),
  listBackups: () => request('/api/v1/backups').then(asArray),
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
};
