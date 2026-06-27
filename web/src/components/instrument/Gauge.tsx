/* ============================================================
   Gauge — SVG arc gauge for a single 0-100 metric.
   Usage: <Gauge value={72} label="CPU" signal="warn" size={96} />
   ============================================================ */
import { Signal } from './types';

const SIG_COLOR: Record<Signal, string> = {
  ok:   '#1D9E75',
  warn: '#EF9F27',
  crit: '#E24B4A',
  off:  '#5F5E5A',
};

interface GaugeProps {
  value: number;       // 0-100
  label?: string;
  subLabel?: string;
  signal?: Signal;
  size?: number;
  thickness?: number;
  className?: string;
}

export function Gauge({
  value,
  label,
  subLabel,
  signal = 'ok',
  size = 96,
  thickness = 8,
  className,
}: GaugeProps) {
  const color = SIG_COLOR[signal];
  const r = (size - thickness) / 2;
  const cx = size / 2;
  const cy = size / 2;
  const START_ANG = -225; // degrees — opens at bottom
  const SWEEP = 270;
  const pct = Math.max(0, Math.min(100, value)) / 100;

  function polar(deg: number) {
    const rad = (deg * Math.PI) / 180;
    return { x: cx + r * Math.cos(rad), y: cy + r * Math.sin(rad) };
  }

  function arcPath(fromDeg: number, toDeg: number) {
    const start = polar(fromDeg);
    const end = polar(toDeg);
    const large = toDeg - fromDeg > 180 ? 1 : 0;
    return `M ${start.x} ${start.y} A ${r} ${r} 0 ${large} 1 ${end.x} ${end.y}`;
  }

  const trackEnd = START_ANG + SWEEP;
  const fillEnd = START_ANG + SWEEP * pct;

  return (
    <div
      className={className}
      style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-label={`${label ?? ''} ${value}%`}>
        {/* Track */}
        <path
          d={arcPath(START_ANG, trackEnd)}
          fill="none"
          stroke="var(--hairline)"
          strokeWidth={thickness}
          strokeLinecap="round"
        />
        {/* Fill */}
        {pct > 0 && (
          <path
            d={arcPath(START_ANG, fillEnd)}
            fill="none"
            stroke={color}
            strokeWidth={thickness}
            strokeLinecap="round"
            style={{ filter: `drop-shadow(0 0 4px ${color}66)` }}
          />
        )}
        {/* Center value */}
        <text
          x={cx} y={cy + 2}
          textAnchor="middle"
          dominantBaseline="middle"
          fill={color}
          fontFamily="'IBM Plex Mono', monospace"
          fontSize={size * 0.2}
          fontWeight={600}
          style={{ fontFeatureSettings: '"tnum"' }}
        >
          {Math.round(value)}%
        </text>
        {subLabel && (
          <text
            x={cx} y={cy + size * 0.17}
            textAnchor="middle"
            fill="#5f6884"
            fontFamily="'IBM Plex Mono', monospace"
            fontSize={size * 0.09}
            letterSpacing="0.06em"
          >
            {subLabel}
          </text>
        )}
      </svg>
      {label && (
        <span style={{
          fontFamily: 'var(--sans-deck)',
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
          color: 'var(--ink-3)',
        }}>
          {label}
        </span>
      )}
    </div>
  );
}
