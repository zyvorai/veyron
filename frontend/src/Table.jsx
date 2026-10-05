import { useMemo, useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { Status, Meter } from './status.jsx';

function Cell({ col, row }) {
  const [k, , t, unit] = col;
  const v = row[k];
  if (t === 'st') return <Status s={v} />;
  if (t === 'meter') return <Meter v={v} />;
  if (k === 'name') return <span className="name">{v}</span>;
  return <span className={t === 'm' ? 'mono dim' : t === 'n' ? '' : ''}>{v ?? '—'}{unit && v != null && v !== '—' ? unit : ''}</span>;
}

export function Table({ cols, rows, selected, focus, onRow, onMenu }) {
  const [sort, setSort] = useState({ k: 'name', d: 1 });
  const sorted = useMemo(() => {
    const arr = [...rows];
    arr.sort((a, b) => {
      const av = a[sort.k];
      const bv = b[sort.k];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      return (av > bv ? 1 : av < bv ? -1 : 0) * sort.d;
    });
    return arr;
  }, [rows, sort]);

  return (
    <table className="vt">
      <thead>
        <tr>
          {cols.map((c) => (
            <th
              key={c[0]}
              className={c[2] === 'n' ? 'num' : ''}
              tabIndex={0}
              aria-sort={sort.k === c[0] ? (sort.d > 0 ? 'ascending' : 'descending') : 'none'}
              onClick={() => setSort({ k: c[0], d: sort.k === c[0] ? -sort.d : 1 })}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setSort({ k: c[0], d: sort.k === c[0] ? -sort.d : 1 });
                }
              }}
            >
              {c[1]}
              {sort.k === c[0] && (
                <span className="s">
                  <ChevronDown size={11} style={{ transform: sort.d < 0 ? 'rotate(180deg)' : '' }} />
                </span>
              )}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => (
          <tr
            key={r.id}
            aria-selected={selected?.has(r.id)}
            className={focus === r.id ? 'focus' : ''}
            tabIndex={onRow ? 0 : undefined}
            onClick={(e) => onRow?.(r, e)}
            onKeyDown={(e) => {
              if (!onRow) return;
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onRow(r, e);
              } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                const sib = e.key === 'ArrowDown' ? e.currentTarget.nextElementSibling : e.currentTarget.previousElementSibling;
                sib?.focus();
              }
            }}
            onContextMenu={(e) => onMenu?.(r, e)}
          >
            {cols.map((c) => (
              <td key={c[0]} className={c[2] === 'n' ? 'num' : ''}>
                <Cell col={c} row={r} />
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
