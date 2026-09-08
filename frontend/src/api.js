// Talks to veyron's existing REST API — the same one the vanilla dashboard.html
// uses (X-API-Key header, key cached in localStorage under 'veyron_api_key').
// Same-origin, so a session already logged into /dashboard is picked up here too.

const KEY_STORAGE = 'veyron_api_key';

export function getApiKey() {
  try {
    return localStorage.getItem(KEY_STORAGE) || sessionStorage.getItem(KEY_STORAGE) || '';
  } catch {
    return '';
  }
}

export function setApiKey(key) {
  try {
    localStorage.setItem(KEY_STORAGE, key);
  } catch {
    /* ignore */
  }
}

async function request(path, opts = {}) {
  const key = getApiKey();
  const headers = Object.assign({}, opts.headers || {});
  if (key) headers['X-API-Key'] = key;
  if (opts.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';

  const resp = await fetch(path, { ...opts, headers });
  const text = await resp.text();
  let body = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      if (!resp.ok) throw new Error(text.slice(0, 200) || `HTTP ${resp.status}`);
      throw new Error('Invalid JSON from API');
    }
  }
  if (!resp.ok) {
    const msg = (body && (body.error?.message ?? body.message)) || `HTTP ${resp.status}`;
    throw new Error(String(msg));
  }
  if (body && typeof body === 'object' && body.success === false) {
    throw new Error(String(body.error?.message ?? body.message ?? 'Request failed'));
  }
  return body;
}

/** Unwrap { success, data } envelopes; pass bare arrays/objects through. */
function unwrap(j) {
  if (j == null) return j;
  if (typeof j === 'object' && 'success' in j && j.success === true && 'data' in j) return j.data;
  return j;
}

function asArray(j) {
  const u = unwrap(j);
  if (Array.isArray(u)) return u;
  if (u && Array.isArray(u.items)) return u.items;
  return [];
}

export const api = {
  listVms: (ns) => request(`/api/v1/vms${ns && ns !== 'all' ? `?namespace=${encodeURIComponent(ns)}` : ''}`).then(asArray),
  listNodes: () => request('/api/v1/nodes').then(asArray),
  listPods: (ns) => request(`/api/v1/pods${ns && ns !== 'all' ? `?namespace=${encodeURIComponent(ns)}` : ''}`).then(asArray),
  listPvcs: () => request('/api/v1/storage/pvcs').then(asArray),
  listSnapshots: () => request('/api/v1/snapshots').then(asArray),
  listBackups: () => request('/api/v1/backups').then(asArray),
  listAlerts: () => request('/api/v1/alerts').then(asArray),
  vmAction: (ns, name, action) =>
    request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/${encodeURIComponent(action)}`, { method: 'POST' }),
  deleteVm: (ns, name) => request(`/api/v1/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}`, { method: 'DELETE' }),
};
