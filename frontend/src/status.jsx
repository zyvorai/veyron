export const SC = {
  Running: 'var(--green)',
  Ready: 'var(--green)',
  Bound: 'var(--green)',
  Healthy: 'var(--green)',
  Succeeded: 'var(--green)',
  Stopped: 'var(--gray)',
  Degraded: 'var(--orange)',
  Pending: 'var(--yellow)',
  Provisioning: 'var(--accent)',
  Paused: 'var(--yellow)',
  Failed: 'var(--red)',
  Up: 'var(--green)',
  Cordoned: 'var(--orange)',
  Unknown: 'var(--gray)',
};

export function Status({ s }) {
  return (
    <span className="st" style={{ '--c': SC[s] || 'var(--gray)' }}>
      <i />
      {s || 'Unknown'}
    </span>
  );
}

export function Meter({ v }) {
  if (v == null || v === '—') return <span className="dim">—</span>;
  const n = Number(v) || 0;
  return (
    <>
      <span className="meter">
        <i style={{ width: `${Math.min(100, Math.max(0, n))}%`, '--c': n > 85 ? 'var(--orange)' : 'var(--accent)' }} />
      </span>
      {n}%
    </>
  );
}

export function Spark({ data, color }) {
  const w = 100;
  const h = 28;
  const arr = data?.length ? data : [0, 0];
  const p = arr
    .map((v, i) => `${(i / Math.max(1, arr.length - 1)) * w},${h - ((Number(v) || 0) / 100) * h}`)
    .join(' ');
  return (
    <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none">
      <polyline points={p} fill="none" stroke={color} strokeWidth="1.6" strokeLinejoin="round" />
    </svg>
  );
}

