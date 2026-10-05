import { useEffect, useRef, useState } from 'react';

/** Fade-and-rise when the element scrolls into view (apple.com chapter reveal). */
export function Reveal({ children, as: Tag = 'div', delay = 0, className = '', ...rest }) {
  const ref = useRef(null);
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    if (typeof IntersectionObserver === 'undefined') {
      setShown(true);
      return undefined;
    }
    const io = new IntersectionObserver(
      ([e]) => {
        if (e.isIntersecting) {
          setShown(true);
          io.disconnect();
        }
      },
      { threshold: 0.12, rootMargin: '0px 0px -40px 0px' },
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);
  return (
    <Tag
      ref={ref}
      className={`reveal${shown ? ' in' : ''} ${className}`}
      style={{ transitionDelay: `${delay}ms` }}
      {...rest}
    >
      {children}
    </Tag>
  );
}

/** Rolling client-side history of a few numbers, sampled whenever `sample` changes. */
export function useSeries(sample, max = 24) {
  const [series, setSeries] = useState({});
  const key = JSON.stringify(sample);
  useEffect(() => {
    setSeries((prev) => {
      const next = {};
      for (const [k, v] of Object.entries(sample)) {
        const arr = [...(prev[k] || []), Number(v) || 0];
        next[k] = arr.slice(-max);
      }
      return next;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, max]);
  return series;
}

function smoothPath(points) {
  if (points.length < 2) return '';
  let d = `M${points[0][0]},${points[0][1]}`;
  for (let i = 1; i < points.length; i += 1) {
    const [x0, y0] = points[i - 1];
    const [x1, y1] = points[i];
    const cx = (x0 + x1) / 2;
    d += ` C${cx},${y0} ${cx},${y1} ${x1},${y1}`;
  }
  return d;
}

/** Big figure + gradient sparkline, Netra PulseFigure style. */
export function PulseFigure({ label, value, unit, series = [], tone = 'sky', sub }) {
  const w = 220;
  const h = 56;
  const arr = series.length > 1 ? series : [0, ...(series.length ? series : [0])];
  const hi = Math.max(1, ...arr);
  const lo = Math.min(...arr);
  const span = hi - lo || 1;
  const pts = arr.map((v, i) => [(i / (arr.length - 1)) * w, h - 6 - ((v - lo) / span) * (h - 14)]);
  const line = smoothPath(pts);
  const area = line ? `${line} L${w},${h} L0,${h} Z` : '';
  const gid = `pf-${label.replace(/\W+/g, '')}`;
  return (
    <div className="pulse" data-tone={tone}>
      <span className="pulse-label">{label}</span>
      <b className="pulse-value">
        {value}
        {unit && <small>{unit}</small>}
      </b>
      {sub && <span className="pulse-sub">{sub}</span>}
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" aria-hidden>
        <defs>
          <linearGradient id={gid} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--tone-color)" stopOpacity="0.32" />
            <stop offset="1" stopColor="var(--tone-color)" stopOpacity="0" />
          </linearGradient>
        </defs>
        {area && <path d={area} fill={`url(#${gid})`} />}
        {line && <path d={line} fill="none" stroke="var(--tone-color)" strokeWidth="2" strokeLinecap="round" />}
        {pts.length > 0 && <circle className="pulse-dot" cx={pts[pts.length - 1][0]} cy={pts[pts.length - 1][1]} r="3.5" />}
      </svg>
    </div>
  );
}

const OS_STYLES = [
  ['ubuntu', 'Ubuntu', '#E95420', '#ff8a4c'],
  ['debian', 'Debian', '#A80030', '#e0336a'],
  ['fedora', 'Fedora', '#294172', '#51a2da'],
  ['centos', 'CentOS Stream', '#262577', '#9ccd2a'],
  ['almalinux', 'AlmaLinux', '#0f4266', '#ffcb12'],
  ['rocky', 'Rocky Linux', '#10b981', '#34d399'],
  ['opensuse', 'openSUSE', '#73ba25', '#35b9ab'],
  ['windows', 'Windows', '#0067b8', '#2fa8ff'],
];

/** Brand family, display name and gradient for an OS/template name. */
export function osInfo(name) {
  const n = String(name || '').toLowerCase();
  const hit = OS_STYLES.find(([k]) => n.includes(k));
  const version = n.replace(/^[a-z-]*?-?(?=\d)/, '').replace(/^(stream|leap)-?/, '');
  if (!hit) return { family: 'linux', label: name || 'Linux', version: '', a: '#636366', b: '#8e8e93' };
  return { family: hit[0], label: hit[1], version: /\d/.test(version) ? version : 'latest', a: hit[2], b: hit[3] };
}

/** Rounded gradient glyph with the OS initial. */
export function OsBadge({ name, size = 40 }) {
  const o = osInfo(name);
  return (
    <span
      className="os-badge"
      style={{ width: size, height: size, background: `linear-gradient(145deg, ${o.b}, ${o.a})`, fontSize: size * 0.42 }}
      aria-hidden
    >
      {o.family === 'windows' ? (
        <svg width={size * 0.46} height={size * 0.46} viewBox="0 0 20 20">
          <path fill="#fff" d="M0 2.8 8 1.7v7.7H0zM9 1.5 20 0v9.4H9zM0 10.6h8v7.7L0 17.2zM9 10.6h11V20L9 18.5z" />
        </svg>
      ) : (
        o.label.charAt(0)
      )}
    </span>
  );
}

/** Inline SVG art for empty states: a soft glass stack with a tone glow. */
export function EmptyArt({ tone = 'sky' }) {
  return (
    <svg className="empty-art" data-tone={tone} viewBox="0 0 160 120" aria-hidden>
      <defs>
        <radialGradient id="eaGlow" cx="50%" cy="55%" r="55%">
          <stop offset="0" stopColor="var(--tone-color)" stopOpacity="0.35" />
          <stop offset="1" stopColor="var(--tone-color)" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="eaCard" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--bg-elevated)" />
          <stop offset="1" stopColor="var(--bg-elevated-2)" />
        </linearGradient>
      </defs>
      <ellipse cx="80" cy="70" rx="72" ry="46" fill="url(#eaGlow)" />
      <rect x="38" y="62" width="84" height="30" rx="9" fill="url(#eaCard)" stroke="var(--hairline-1)" />
      <rect x="32" y="44" width="96" height="30" rx="9" fill="url(#eaCard)" stroke="var(--hairline-1)" />
      <rect x="26" y="26" width="108" height="30" rx="9" fill="url(#eaCard)" stroke="var(--border)" />
      <circle cx="42" cy="41" r="4" fill="var(--tone-color)" />
      <rect x="52" y="37" width="44" height="8" rx="4" fill="var(--hairline-1)" />
      <rect x="102" y="37" width="20" height="8" rx="4" fill="var(--tone-color)" opacity="0.5" />
    </svg>
  );
}
