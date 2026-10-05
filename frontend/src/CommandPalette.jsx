import { useEffect, useMemo, useRef, useState } from 'react';
import { Search, CornerDownLeft, Plus, Sun, Moon, Terminal, ArrowRight } from 'lucide-react';
import { NAV, RES_META, pageLabel, pageBlurb } from './resources.js';
import { Status } from './status.jsx';

const RECENT_KEY = 'veyron_palette_recent';

function score(hay, q) {
  if (!q) return 1;
  const h = hay.toLowerCase();
  const n = q.toLowerCase();
  const at = h.indexOf(n);
  if (at === 0) return 100;
  if (at > 0) return 60 - Math.min(at, 40);
  let i = 0;
  for (const c of h) if (c === n[i]) i += 1;
  return i === n.length ? 10 : 0;
}

function loadRecent() {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) || '[]');
  } catch {
    return [];
  }
}

/** ⌘K palette: jump to pages, resources and actions. */
export function CommandPalette({ open, onClose, data, go, theme, onToggleTheme, onCreate, onConsole }) {
  const [q, setQ] = useState('');
  const [idx, setIdx] = useState(0);
  const inputRef = useRef(null);
  const listRef = useRef(null);

  useEffect(() => {
    if (open) {
      setQ('');
      setIdx(0);
      setTimeout(() => inputRef.current?.focus(), 10);
    }
  }, [open]);

  const items = useMemo(() => {
    const out = [];
    out.push({ key: 'act:new', group: 'Actions', label: 'Create a machine', hint: 'New VM from a template', I: Plus, run: onCreate });
    out.push({
      key: 'act:theme',
      group: 'Actions',
      label: theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode',
      hint: 'Appearance',
      I: theme === 'dark' ? Sun : Moon,
      run: onToggleTheme,
    });
    for (const g of NAV) {
      for (const [id] of g.items) {
        out.push({
          key: `page:${id}`,
          group: 'Pages',
          label: pageLabel(id),
          hint: pageBlurb(id) || g.g,
          I: RES_META[id]?.I || ArrowRight,
          run: () => go(id),
        });
      }
    }
    for (const [id, res] of Object.entries(data)) {
      for (const r of (res.rows || []).slice(0, 400)) {
        if (!r?.name) continue;
        out.push({
          key: `res:${id}:${r.id}`,
          group: res.l,
          label: r.name,
          hint: [r.ns, r.os !== '—' ? r.os : null, r.host !== '—' ? r.host : null].filter(Boolean).join(' · '),
          status: r.status,
          I: res.I,
          run: () => go(id, r.id),
        });
        if (id === 'vms' && r.status === 'Running') {
          out.push({
            key: `console:${r.id}`,
            group: 'Actions',
            label: `Open console · ${r.name}`,
            hint: 'Live screen',
            I: Terminal,
            run: () => onConsole(r),
          });
        }
      }
    }
    return out;
  }, [data, theme, go, onCreate, onToggleTheme, onConsole]);

  const results = useMemo(() => {
    if (!q.trim()) {
      const recent = loadRecent();
      const byKey = new Map(items.map((it) => [it.key, it]));
      const rec = recent.map((k) => byKey.get(k)).filter(Boolean).map((it) => ({ ...it, group: 'Recent' }));
      const base = items.filter((it) => it.group === 'Actions' || it.group === 'Pages').slice(0, 14);
      return [...rec, ...base.filter((b) => !recent.includes(b.key))];
    }
    return items
      .map((it) => ({ it, s: Math.max(score(it.label, q), score(it.hint || '', q) * 0.5) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s)
      .slice(0, 40)
      .map((x) => x.it);
  }, [items, q]);

  useEffect(() => {
    setIdx(0);
  }, [q]);

  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }, [idx]);

  if (!open) return null;

  const choose = (it) => {
    if (!it) return;
    const rec = [it.key, ...loadRecent().filter((k) => k !== it.key)].slice(0, 5);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(rec));
    } catch {
      /* storage unavailable */
    }
    onClose();
    it.run?.();
  };

  const onKey = (e) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setIdx((i) => Math.min(results.length - 1, i + 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setIdx((i) => Math.max(0, i - 1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(results[idx]);
    } else if (e.key === 'Escape') {
      onClose();
    }
  };

  let lastGroup = null;
  return (
    <div className="cmdk-scrim" onMouseDown={onClose}>
      <div className="cmdk" role="dialog" aria-label="Command palette" onMouseDown={(e) => e.stopPropagation()}>
        <label className="cmdk-input">
          <Search size={18} />
          <input
            ref={inputRef}
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={onKey}
            placeholder="Search machines, hosts, pages and actions"
            aria-label="Search"
          />
          <kbd>esc</kbd>
        </label>
        <div className="cmdk-list" ref={listRef} role="listbox">
          {results.length === 0 && <div className="cmdk-empty">No results for “{q}”.</div>}
          {results.map((it, i) => {
            const head = it.group !== lastGroup ? it.group : null;
            lastGroup = it.group;
            const I = it.I || ArrowRight;
            return (
              <div key={`${it.key}-${i}`}>
                {head && <div className="cmdk-group">{head}</div>}
                <button
                  type="button"
                  role="option"
                  aria-selected={i === idx}
                  className="cmdk-item"
                  onMouseMove={() => setIdx(i)}
                  onClick={() => choose(it)}
                >
                  <span className="cmdk-ico">
                    <I size={15} strokeWidth={1.9} />
                  </span>
                  <span className="cmdk-text">
                    <b>{it.label}</b>
                    {it.hint && <small>{it.hint}</small>}
                  </span>
                  {it.status && <Status s={it.status} />}
                  {i === idx && <CornerDownLeft size={14} className="cmdk-enter" />}
                </button>
              </div>
            );
          })}
        </div>
        <div className="cmdk-foot">
          <span>
            <kbd>↑</kbd>
            <kbd>↓</kbd> to move
          </span>
          <span>
            <kbd>↵</kbd> to open
          </span>
          <span>
            <kbd>⌘K</kbd> anywhere
          </span>
        </div>
      </div>
    </div>
  );
}
