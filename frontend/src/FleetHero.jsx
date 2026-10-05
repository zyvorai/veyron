import { useMemo, useState } from 'react';
import { SC } from './status.jsx';

const W = 1000;
const H = 440;
const MAX_HOSTS = 6;
const MAX_DOTS = 28;

function trunc(s, n) {
  const t = String(s || '');
  return t.length > n ? `${t.slice(0, n - 1)}…` : t;
}

/**
 * Live fleet diagram: the control plane on top, hosts as glass cards with their VMs
 * orbiting as status-colored dots, flowing links to storage and network below.
 */
export function FleetHero({ hosts, vms, onVm, onHost }) {
  const [hover, setHover] = useState(null);

  const layout = useMemo(() => {
    const byHost = new Map();
    for (const h of hosts) byHost.set(h.name, []);
    const pending = [];
    const parked = [];
    for (const v of vms) {
      if (byHost.has(v.host)) byHost.get(v.host).push(v);
      else if (['Stopped', 'Halted', 'Paused'].includes(v.status)) parked.push(v);
      else pending.push(v);
    }
    let list = hosts.map((h) => ({ host: h, vms: byHost.get(h.name) || [] }));
    list.sort((a, b) => b.vms.length - a.vms.length);
    let overflow = 0;
    if (list.length > MAX_HOSTS) {
      overflow = list.length - (MAX_HOSTS - 1);
      const rest = list.slice(MAX_HOSTS - 1);
      list = list.slice(0, MAX_HOSTS - 1);
      list.push({
        host: { name: `+${overflow} more`, status: 'Ready', cpu: 0, synthetic: true },
        vms: rest.flatMap((r) => r.vms),
      });
    }
    if (pending.length) {
      list.push({ host: { name: 'Scheduling', status: 'Pending', cpu: 0, synthetic: true }, vms: pending });
    }
    if (parked.length) {
      list.push({ host: { name: 'Powered off', status: 'Stopped', cpu: 0, synthetic: true }, vms: parked });
    }
    if (!list.length) list = [{ host: { name: 'No hosts yet', status: 'Unknown', cpu: 0, synthetic: true }, vms: [] }];
    const n = list.length;
    const gap = W / (n + 1);
    return list.map((item, i) => ({ ...item, x: gap * (i + 1), y: 238 }));
  }, [hosts, vms]);

  const ctrl = { x: W / 2, y: 64 };
  const r = layout.length > 4 ? 52 : 62;

  return (
    <div className="fleet">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Live fleet diagram">
        <defs>
          <radialGradient id="fhGlow" cx="50%" cy="40%" r="60%">
            <stop offset="0" stopColor="#2997ff" stopOpacity="0.28" />
            <stop offset="1" stopColor="#2997ff" stopOpacity="0" />
          </radialGradient>
          <linearGradient id="fhCard" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="rgba(255,255,255,0.14)" />
            <stop offset="1" stopColor="rgba(255,255,255,0.04)" />
          </linearGradient>
          <linearGradient id="fhLink" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#2997ff" stopOpacity="0.9" />
            <stop offset="1" stopColor="#64d2ff" stopOpacity="0.35" />
          </linearGradient>
          <filter id="fhBlur" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="6" />
          </filter>
        </defs>

        <ellipse cx={W / 2} cy={H * 0.45} rx={W * 0.55} ry={H * 0.6} fill="url(#fhGlow)" />

        {layout.map((l) => (
          <path
            key={`link-${l.host.name}`}
            className="fh-link"
            d={`M${ctrl.x},${ctrl.y + 26} C${ctrl.x},${(ctrl.y + l.y) / 2} ${l.x},${(ctrl.y + l.y) / 2 - 20} ${l.x},${l.y - r - 8}`}
            stroke="url(#fhLink)"
          />
        ))}
        {layout.map((l) => (
          <path
            key={`down-${l.host.name}`}
            className="fh-link fh-link--down"
            d={`M${l.x},${l.y + r + 24} C${l.x},${l.y + r + 60} ${W / 2},${H - 70} ${W / 2},${H - 40}`}
            stroke="url(#fhLink)"
          />
        ))}

        <g className="fh-ctrl" transform={`translate(${ctrl.x},${ctrl.y})`}>
          <circle r="34" fill="#2997ff" opacity="0.25" filter="url(#fhBlur)" />
          <rect x="-92" y="-24" width="184" height="48" rx="24" fill="url(#fhCard)" stroke="rgba(255,255,255,0.22)" />
          <circle cx="-66" cy="0" r="9" fill="#2997ff" className="fh-beat" />
          <text x="-48" y="-3" className="fh-t1">Veyron</text>
          <text x="-48" y="12" className="fh-t2">control plane</text>
        </g>

        {layout.map((l, hi) => {
          const dots = l.vms.slice(0, MAX_DOTS);
          const extra = l.vms.length - dots.length;
          const running = l.vms.filter((v) => v.status === 'Running').length;
          const warn = ['Cordoned', 'NotReady', 'Pending', 'Unknown'].includes(l.host.status);
          const dur = 38 + hi * 7;
          return (
            <g key={l.host.name} transform={`translate(${l.x},${l.y})`}>
              <circle r={r + 14} fill="none" stroke="rgba(255,255,255,0.08)" />
              <circle r={r + 14} fill="none" stroke="rgba(41,151,255,0.35)" strokeDasharray="2 10" className="fh-ring" />
              <g className="fh-orbit" style={{ animationDuration: `${dur}s` }}>
                {dots.map((v, i) => {
                  const a = (i / Math.max(1, dots.length)) * Math.PI * 2;
                  const cx = Math.cos(a) * (r + 14);
                  const cy = Math.sin(a) * (r + 14);
                  const color = SC[v.status] || 'var(--gray)';
                  return (
                    <g
                      key={v.id}
                      className={`fh-vm${v.status === 'Running' ? ' run' : ''}`}
                      transform={`translate(${cx},${cy})`}
                      onClick={() => onVm?.(v)}
                      onMouseEnter={() => setHover({ v, x: l.x, y: l.y - r - 22 })}
                      onMouseLeave={() => setHover(null)}
                    >
                      <circle r="9" fill={color} opacity="0.22" className="fh-halo" />
                      <circle r="5" fill={color} stroke="rgba(0,0,0,0.35)" strokeWidth="0.5" />
                    </g>
                  );
                })}
              </g>
              <g
                className={`fh-host${l.host.synthetic ? '' : ' clickable'}`}
                onClick={() => !l.host.synthetic && onHost?.(l.host)}
              >
                <rect
                  x={-r}
                  y={-r * 0.62}
                  width={r * 2}
                  height={r * 1.24}
                  rx="16"
                  fill="url(#fhCard)"
                  stroke={warn ? 'rgba(255,159,10,0.7)' : 'rgba(255,255,255,0.22)'}
                />
                <text y={-r * 0.18} className="fh-host-name" textAnchor="middle">
                  {trunc(l.host.name, 14)}
                </text>
                <text y={r * 0.12} className="fh-host-sub" textAnchor="middle">
                  {l.vms.length ? `${running}/${l.vms.length} running` : warn ? l.host.status : 'idle'}
                  {extra > 0 ? ` · +${extra}` : ''}
                </text>
                {!l.host.synthetic && (
                  <g transform={`translate(${-r * 0.6},${r * 0.32})`}>
                    <rect width={r * 1.2} height="5" rx="2.5" fill="rgba(255,255,255,0.14)" />
                    <rect
                      width={Math.max(3, (r * 1.2 * Math.min(100, Number(l.host.cpu) || 0)) / 100)}
                      height="5"
                      rx="2.5"
                      fill={Number(l.host.cpu) > 85 ? '#ff9f0a' : '#2997ff'}
                    />
                  </g>
                )}
              </g>
            </g>
          );
        })}

        <g transform={`translate(${W / 2},${H - 28})`}>
          <rect x="-170" y="-16" width="340" height="32" rx="16" fill="url(#fhCard)" stroke="rgba(255,255,255,0.16)" />
          <text x="0" y="5" textAnchor="middle" className="fh-t2">
            storage · network · snapshots
          </text>
        </g>
      </svg>
      {hover && (
        <div className="fh-tip" style={{ left: `${(hover.x / W) * 100}%`, top: `${(hover.y / H) * 100}%` }}>
          <b>{hover.v.name}</b>
          <span>
            {hover.v.status} · {hover.v.cpu} vCPU · {hover.v.ram} GiB
          </span>
          {hover.v.ip && hover.v.ip !== '—' && <span className="mono">{hover.v.ip}</span>}
        </div>
      )}
    </div>
  );
}
