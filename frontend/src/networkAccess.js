// Pure helpers for the Network access views (no React, no network), so they can be tested.

/** How a VM's QEMU process may reach the network, as the console names it. `inherit` = no policy of its own. */
export const MODES = {
  loopback: { id: 'loopback', label: 'Isolated', tone: 'rose', blurb: 'Nothing leaves the VM, not even to Veyron.' },
  allowlist: { id: 'allowlist', label: 'Allow-list', tone: 'sky', blurb: 'Only the listed destinations are reachable.' },
  audit: { id: 'audit', label: 'Learning', tone: 'amber', blurb: 'Everything is allowed and recorded.' },
  open: { id: 'open', label: 'Open', tone: 'danger', blurb: 'No restrictions.' },
  inherit: { id: 'inherit', label: 'Node default', tone: 'gray', blurb: 'Follows the node’s default policy.' },
};

export const MODE_ORDER = ['loopback', 'allowlist', 'audit', 'open'];

export function modeInfo(mode) {
  return MODES[mode] || MODES.inherit;
}

/** `10.0.0.0/8 · tcp/5151`, `0.0.0.0/0 · udp/53`, `192.168.0.0/16 · all ports`. */
export function ruleLabel(r) {
  if (!r) return '';
  const where = r.port ? `${r.proto && r.proto !== 'any' ? r.proto : 'tcp'}/${r.port}` : 'all ports';
  return `${r.cidr} · ${where}`;
}

export function ruleKey(r) {
  return `${r.cidr}|${r.proto || 'any'}|${r.port ?? ''}`;
}

export const sameRule = (a, b) => ruleKey(a) === ruleKey(b);

/** Add `rule` unless an identical one is present; remove it if it is (the chip toggle). */
export function toggleRule(rules, rule) {
  const list = rules || [];
  return list.some((r) => sameRule(r, rule)) ? list.filter((r) => !sameRule(r, rule)) : [...list, rule];
}

/** The preset a policy is exactly equal to (same mode and the same rules, notes ignored), if any. */
export function matchPreset(policy, presets) {
  if (!policy) return null;
  const keys = (rules) => (rules || []).map(ruleKey).sort().join(',');
  return (presets || []).find((p) => p.mode === policy.mode && keys(p.rules) === keys(policy.rules)) || null;
}

/** Parse "host", "host:port" or "host:port/udp" typed into the explain box. */
export function parseTarget(text) {
  const m = String(text || '')
    .trim()
    .match(/^([A-Za-z0-9._-]+)(?::(\d{1,5}))?(?:\/(tcp|udp))?$/);
  if (!m) return null;
  const port = m[2] ? Number(m[2]) : 443;
  if (port < 1 || port > 65535) return null;
  return { host: m[1], port, proto: m[3] || 'tcp' };
}

/** Split the node's observed list into refused flows and audit-mode "would be refused" flows. */
export function summarizeFlows(flows) {
  const list = Array.isArray(flows) ? flows : [];
  const blocked = list.filter((f) => !f.audit);
  const wouldBlock = list.filter((f) => f.audit);
  const sum = (a) => a.reduce((n, f) => n + (f.packets || 0), 0);
  return { blocked, wouldBlock, blockedPackets: sum(blocked), wouldBlockPackets: sum(wouldBlock) };
}

export function flowLabel(f) {
  return `${f.dst}${f.port ? `:${f.port}` : ''}${f.proto && f.proto !== 'any' ? `/${f.proto}` : ''}`;
}

/** Body for PUT /vms/:ns/:name/egress. A preset alone, or a full replacement from rules. */
export function putBody({ preset, mode, rules, expiresIn } = {}) {
  const body = {};
  if (preset) body.preset = preset;
  if (mode) body.mode = mode;
  if (rules && (!preset || rules.length)) body.rules = rules.map(({ cidr, proto, port, note }) => ({ cidr, proto: proto || 'any', ...(port ? { port } : {}), ...(note ? { note } : {}) }));
  if (expiresIn) body.expires_in = expiresIn;
  return body;
}

export const DURATIONS = [
  { id: '30m', label: '30 minutes' },
  { id: '1h', label: '1 hour' },
  { id: '4h', label: '4 hours' },
  { id: '1d', label: '1 day' },
];

/** "59m 12s", "2h 05m", or "ending…" once the time is up. */
export function formatRemaining(ms) {
  if (!Number.isFinite(ms) || ms <= 0) return 'ending…';
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`;
  return `${m}m ${String(s % 60).padStart(2, '0')}s`;
}

/** 1 → just started, 0 → time is up. */
export function remainingFraction(createdAt, expiresAt, now = Date.now()) {
  const start = Date.parse(createdAt);
  const end = Date.parse(expiresAt);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) return 0;
  return Math.max(0, Math.min(1, (end - now) / (end - start)));
}

/** Heat level of a fleet tile: 0 quiet … 3 loud, from refused-flow count. */
export function heatLevel(blocked) {
  if (!blocked) return 0;
  if (blocked < 3) return 1;
  if (blocked < 10) return 2;
  return 3;
}

/** Fleet tiles filtered by mode / problem chips and a text query. */
export function filterTiles(items, { mode = 'all', only = '', query = '' } = {}) {
  const q = query.trim().toLowerCase();
  return (items || []).filter((i) => {
    if (mode !== 'all' && i.mode !== mode) return false;
    if (only === 'blocked' && !(i.blocked > 0)) return false;
    if (only === 'noagent' && !(i.running && i.agent_connected === false)) return false;
    if (only === 'running' && !i.running) return false;
    if (q && !`${i.namespace}/${i.name}`.toLowerCase().includes(q)) return false;
    return true;
  });
}

/** Counts for the filter chips. */
export function tallies(items) {
  const list = items || [];
  const byMode = {};
  for (const i of list) byMode[i.mode] = (byMode[i.mode] || 0) + 1;
  return {
    total: list.length,
    byMode,
    blocked: list.filter((i) => i.blocked > 0).length,
    noAgent: list.filter((i) => i.running && i.agent_connected === false).length,
    running: list.filter((i) => i.running).length,
  };
}

/** Ease a number toward a target (used by count-up animations); pure so it can be tested. */
export function easeCount(from, to, t) {
  const p = Math.max(0, Math.min(1, t));
  const eased = 1 - (1 - p) ** 3;
  return Math.round(from + (to - from) * eased);
}

/** Position `i` of `n` destinations along the right edge of the flow diagram (SVG y). */
export function lane(i, n, top = 36, bottom = 204) {
  if (n <= 1) return (top + bottom) / 2;
  return top + ((bottom - top) * i) / (n - 1);
}

/** Destinations to draw: allowed rules first, then refused flows, capped so the diagram stays legible. */
export function diagramNodes(policy, flows, cap = 6) {
  const rules = (policy?.rules || []).slice(0, Math.max(0, cap - 2)).map((r) => ({ kind: 'allowed', label: r.note || r.cidr, sub: ruleLabel(r) }));
  const seen = summarizeFlows(flows);
  const refused = seen.blocked.slice(0, 3).map((f) => ({ kind: 'blocked', label: flowLabel(f), sub: `${f.packets} pkt` }));
  const audit = seen.wouldBlock.slice(0, 3).map((f) => ({ kind: 'learning', label: flowLabel(f), sub: `${f.packets} pkt` }));
  const nodes = [...rules, ...refused, ...audit];
  if (policy?.mode === 'open' && !nodes.length) nodes.push({ kind: 'allowed', label: 'anywhere', sub: 'no restrictions' });
  return nodes.slice(0, cap);
}
