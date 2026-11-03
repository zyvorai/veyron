import { useEffect, useMemo, useRef, useState } from 'react';
import { Search, CornerDownLeft } from 'lucide-react';

/**
 * Cmd/Ctrl+K palette: jump to any page or VM, or run a global action.
 * `items` = [{ id, label, hint, group, run }] built by the caller so the palette stays dumb.
 */
export function CommandPalette({ open, onClose, items }) {
  const [q, setQ] = useState('');
  const [idx, setIdx] = useState(0);
  const inputRef = useRef(null);

  useEffect(() => {
    if (open) {
      setQ('');
      setIdx(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  const results = useMemo(() => {
    const t = q.trim().toLowerCase();
    if (!t) return items.slice(0, 30);
    const score = (it) => {
      const l = it.label.toLowerCase();
      if (l === t) return 0;
      if (l.startsWith(t)) return 1;
      if (l.includes(t)) return 2;
      if ((it.hint || '').toLowerCase().includes(t) || (it.group || '').toLowerCase().includes(t)) return 3;
      return 9;
    };
    return items
      .map((it) => [score(it), it])
      .filter(([s]) => s < 9)
      .sort((a, b) => a[0] - b[0])
      .map(([, it]) => it)
      .slice(0, 30);
  }, [q, items]);

  useEffect(() => setIdx(0), [q]);

  if (!open) return null;

  const runAt = (i) => {
    const it = results[i];
    if (!it) return;
    onClose();
    it.run();
  };

  const onKey = (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); setIdx((i) => Math.min(i + 1, results.length - 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setIdx((i) => Math.max(i - 1, 0)); }
    else if (e.key === 'Enter') { e.preventDefault(); runAt(idx); }
    else if (e.key === 'Escape') { e.preventDefault(); onClose(); }
  };

  let lastGroup = null;
  return (
    <div className="scrim cmdk-scrim" onClick={onClose}>
      <div className="cmdk" role="dialog" aria-label="Command palette" onClick={(e) => e.stopPropagation()}>
        <div className="cmdk-in">
          <Search size={16} />
          <input
            ref={inputRef}
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={onKey}
            placeholder="Jump to a page or VM, or run an action…"
            aria-label="Search commands"
          />
          <kbd>esc</kbd>
        </div>
        <div className="cmdk-list" role="listbox">
          {results.length === 0 && <div className="cmdk-empty">No matches</div>}
          {results.map((it, i) => {
            const head = it.group !== lastGroup ? <div className="cmdk-g">{it.group}</div> : null;
            lastGroup = it.group;
            return (
              <div key={it.id}>
                {head}
                <button
                  role="option"
                  aria-selected={i === idx}
                  className={`cmdk-i ${i === idx ? 'on' : ''}`}
                  onMouseEnter={() => setIdx(i)}
                  onClick={() => runAt(i)}
                >
                  <span>{it.label}</span>
                  {it.hint && <small>{it.hint}</small>}
                  {i === idx && <CornerDownLeft size={13} />}
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
