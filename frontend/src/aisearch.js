/** Evaluate a `/ai/search` filter over rows the console already has. */

function num(v) {
  if (typeof v === 'number') return v;
  const n = parseFloat(String(v ?? '').replace(/[^\d.-]/g, ''));
  return Number.isFinite(n) ? n : null;
}

function str(v) {
  return String(v ?? '').toLowerCase();
}

export function matches(row, cond) {
  const v = row[cond.field];
  const want = cond.value;
  switch (cond.op) {
    case 'eq':
      return typeof want === 'number' ? num(v) === want : str(v) === str(want);
    case 'ne':
      return typeof want === 'number' ? num(v) !== want : str(v) !== str(want);
    case 'contains':
      return str(v).includes(str(want));
    case 'not_contains':
      return !str(v).includes(str(want));
    case 'in':
      return (Array.isArray(want) ? want : [want]).some((w) => str(v) === str(w));
    case 'gt':
    case 'gte':
    case 'lt':
    case 'lte': {
      const n = num(v);
      if (n == null) return false;
      if (cond.op === 'gt') return n > want;
      if (cond.op === 'gte') return n >= want;
      if (cond.op === 'lt') return n < want;
      return n <= want;
    }
    default:
      return true;
  }
}

export function applyFilter(rows, filter) {
  let out = (rows || []).filter((r) => (filter?.where || []).every((c) => matches(r, c)));
  const s = filter?.sort;
  if (s?.field) {
    const dir = s.dir === 'asc' ? 1 : -1;
    out = [...out].sort((a, b) => {
      const x = num(a[s.field]);
      const y = num(b[s.field]);
      if (x != null && y != null) return (x - y) * dir;
      if (x == null && y != null) return 1;
      if (y == null && x != null) return -1;
      return str(a[s.field]).localeCompare(str(b[s.field])) * dir;
    });
  }
  if (filter?.limit) out = out.slice(0, filter.limit);
  return out;
}

/** Free text that reads like a question rather than a name. */
export function looksLikeQuestion(q) {
  const t = q.trim();
  if (t.startsWith('?')) return true;
  const words = t.split(/\s+/).filter(Boolean);
  return words.length >= 3 || /\?$/.test(t) || /^(which|what|why|how|show|list|find|who|where)\b/i.test(t);
}
