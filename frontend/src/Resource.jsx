import { useMemo, useState } from 'react';
import { ChevronDown } from 'lucide-react';

// Generic sortable list + inspector pair used by every read-only resource page
// (Hosts, Pods, Storage, Snapshots, Backups, ...) so each new resource type is just
// a column/field definition, not another copy of the table+sort+inspector plumbing.

function StatusDot({ value, statusMap }) {
  return <span className="st" style={{ '--c': (statusMap && statusMap[value]) || 'var(--gray)' }}><i />{value || 'Unknown'}</span>;
}

export function ResourceTable({ cols, rows, rowKey, focus, onRow, statusMap, statusKey }) {
  const [sort, setSort] = useState({ k: cols[0][0], d: 1 });
  const sorted = useMemo(() => {
    const arr = [...rows];
    arr.sort((a, b) => {
      const av = a[sort.k], bv = b[sort.k];
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
            <th key={c[0]} className={c[2] === 'n' ? 'num' : ''} onClick={() => setSort((s) => ({ k: c[0], d: s.k === c[0] ? -s.d : 1 }))}>
              {c[1]}
              {sort.k === c[0] && <span className="s"><ChevronDown size={11} style={{ transform: sort.d < 0 ? 'rotate(180deg)' : '' }} /></span>}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => {
          const key = rowKey(r);
          return (
            <tr key={key} aria-selected={focus === key} className={focus === key ? 'focus' : ''} onClick={() => onRow(r)}>
              {cols.map((c, i) => {
                const v = c.render ? c.render(r) : r[c[0]];
                if (statusKey && c[0] === statusKey) return <td key={c[0]}><StatusDot value={v} statusMap={statusMap} /></td>;
                return <td key={c[0]} className={c[2] === 'n' ? 'num' : i === 0 ? 'name' : 'dim'}>{v ?? '—'}</td>;
              })}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

export function ResourceInspector({ item, hide, title, kind, statusKey, statusMap, fields, emptyLabel }) {
  if (!item) return <aside className={`insp ${hide ? 'hide' : ''}`}><div className="empty"><div><b>No selection</b>{emptyLabel || 'Select an item to inspect it.'}</div></div></aside>;
  return (
    <aside className={`insp ${hide ? 'hide' : ''}`}>
      <div className="insp-h">
        <h2>{title(item)}</h2>
        <div className="kind">{kind}{statusKey ? <> · <StatusDot value={item[statusKey]} statusMap={statusMap} /></> : null}</div>
      </div>
      <div className="form" style={{ paddingTop: 16 }}>
        {fields.map(([l, v]) => <div className="frow" key={l}><label>{l}</label><span className="mono">{typeof v === 'function' ? v(item) : item[v] ?? '—'}</span></div>)}
      </div>
    </aside>
  );
}
