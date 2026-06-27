/* ============================================================
   MetricTile — KPI tile with optional inline sparkline.
   Usage:
     <MetricTile label="Running VMs" value={42} unit="" signal="ok" />
     <MetricTile label="Avg CPU" value={67} unit="%" signal="warn" spark={samples} />
   ============================================================ */
import { Signal } from './types';

const SIG_COLOR: Record<Signal, string> = {
  ok:   'var(--nominal)',
  warn: 'var(--caution)',
  crit: 'var(--critical)',
  off:  'var(--inert)',
};

interface MetricTileProps {
  label: string;
  value: number | string;
  unit?: string;
  signal?: Signal;
  spark?: number[];   // last N samples, 0-100
  onClick?: () => void;
  className?: string;
}

function Sparkline({ data, color }: { data: number[]; color: string }) {
  if (data.length < 2) return null;
  const W = 60, H = 22;
  const min = Math.min(...data);
  const max = Math.max(...data) || 1;
  const pts = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * W;
      const y = H - ((v - min) / (max - min + 0.001)) * H;
      return `${x},${y}`;
    })
    .join(' ');
  return (
    <svg
      width={W} height={H}
      viewBox={`0 0 ${W} ${H}`}
      style={{ display: 'block', overflow: 'visible' }}
      aria-hidden="true"
    >
      <polyline
        points={pts}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity={0.75}
      />
    </svg>
  );
}

export function MetricTile({ label, value, unit = '', signal = 'ok', spark, onClick, className }: MetricTileProps) {
  const color = SIG_COLOR[signal];
  return (
    <div
      className={className}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      style={{
        background: 'var(--panel)',
        border: '1px solid var(--hairline)',
        borderRadius: 12,
        padding: '14px 16px 12px',
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        cursor: onClick ? 'pointer' : undefined,
        transition: 'border-color .15s, box-shadow .15s',
      }}
      onMouseEnter={e => {
        if (onClick) {
          (e.currentTarget as HTMLElement).style.borderColor = 'var(--hairline-hi)';
          (e.currentTarget as HTMLElement).style.boxShadow = '0 4px 20px rgba(0,0,0,.3)';
        }
      }}
      onMouseLeave={e => {
        (e.currentTarget as HTMLElement).style.borderColor = 'var(--hairline)';
        (e.currentTarget as HTMLElement).style.boxShadow = 'none';
      }}
    >
      <div style={{
        fontFamily: 'var(--sans-deck)',
        fontSize: 10,
        fontWeight: 600,
        letterSpacing: '0.08em',
        textTransform: 'uppercase',
        color: 'var(--ink-3)',
      }}>
        {label}
      </div>

      <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: 8 }}>
        <div>
          <span style={{
            fontFamily: 'var(--mono-deck)',
            fontSize: 26,
            fontWeight: 600,
            color,
            fontFeatureSettings: '"tnum"',
            lineHeight: 1,
          }}>
            {value}
          </span>
          {unit && (
            <span style={{
              fontFamily: 'var(--mono-deck)',
              fontSize: 13,
              color: 'var(--ink-3)',
              marginLeft: 3,
            }}>
              {unit}
            </span>
          )}
        </div>
        {spark && spark.length >= 2 && <Sparkline data={spark} color={color} />}
      </div>
    </div>
  );
}
