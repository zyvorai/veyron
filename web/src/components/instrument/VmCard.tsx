/* ============================================================
   VmCard — VM summary card with status strip, metric bars,
   and click-through to inspector.
   Usage: <VmCard vm={vm} onClick={(vm) => openConsole(vm)} />
   ============================================================ */
import { FleetVM, Signal } from './types';

const SIG: Record<Signal, { color: string; bg: string; label: string }> = {
  ok:   { color: '#1D9E75', bg: 'rgba(29,158,117,0.12)',  label: 'Running'  },
  warn: { color: '#EF9F27', bg: 'rgba(239,159,39,0.12)',  label: 'Warning'  },
  crit: { color: '#E24B4A', bg: 'rgba(226,75,74,0.12)',   label: 'Critical' },
  off:  { color: '#5F5E5A', bg: 'rgba(95,94,90,0.10)',    label: 'Stopped'  },
};

interface VmCardProps {
  vm: FleetVM;
  onClick?: (vm: FleetVM) => void;
  selected?: boolean;
  className?: string;
}

function Bar({ pct, color }: { pct: number; color: string }) {
  return (
    <div style={{ height: 3, background: 'var(--hairline)', borderRadius: 2, overflow: 'hidden' }}>
      <div style={{
        height: '100%',
        width: `${Math.max(0, Math.min(100, pct))}%`,
        background: color,
        borderRadius: 2,
        transition: 'width .4s ease',
      }} />
    </div>
  );
}

export function VmCard({ vm, onClick, selected = false, className }: VmCardProps) {
  const sig = SIG[vm.signal];
  const cpuSig: Signal = vm.cpuPct > 90 ? 'crit' : vm.cpuPct > 75 ? 'warn' : 'ok';
  const memSig: Signal = vm.memPct > 90 ? 'crit' : vm.memPct > 75 ? 'warn' : 'ok';

  return (
    <div
      className={className}
      onClick={() => onClick?.(vm)}
      tabIndex={onClick ? 0 : undefined}
      role={onClick ? 'button' : undefined}
      onKeyDown={e => e.key === 'Enter' && onClick?.(vm)}
      style={{
        position: 'relative',
        background: 'var(--panel)',
        border: `1px solid ${selected ? sig.color : 'var(--hairline)'}`,
        borderRadius: 12,
        overflow: 'hidden',
        cursor: onClick ? 'pointer' : undefined,
        transition: 'border-color .15s, box-shadow .15s, transform .12s',
        userSelect: 'none',
      }}
      onMouseEnter={e => {
        const el = e.currentTarget as HTMLElement;
        el.style.borderColor = sig.color;
        el.style.boxShadow = `0 6px 24px rgba(0,0,0,.35), 0 0 0 1px ${sig.color}22`;
        el.style.transform = 'translateY(-1px)';
      }}
      onMouseLeave={e => {
        const el = e.currentTarget as HTMLElement;
        el.style.borderColor = selected ? sig.color : 'var(--hairline)';
        el.style.boxShadow = 'none';
        el.style.transform = 'none';
      }}
    >
      {/* Signal strip on left edge */}
      <div style={{
        position: 'absolute',
        left: 0, top: 0, bottom: 0,
        width: 3,
        background: sig.color,
        borderRadius: '12px 0 0 12px',
      }} />

      <div style={{ padding: '12px 14px 10px 16px' }}>
        {/* Header row */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
          <div style={{ minWidth: 0 }}>
            <div style={{
              fontFamily: 'var(--sans-deck)',
              fontSize: 13,
              fontWeight: 600,
              color: 'var(--ink)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
            }}>
              {vm.name}
            </div>
            <div style={{
              fontFamily: 'var(--mono-deck)',
              fontSize: 10,
              color: 'var(--ink-3)',
              letterSpacing: '0.04em',
            }}>
              {vm.namespace} · {vm.node}
            </div>
          </div>
          <span style={{
            fontFamily: 'var(--mono-deck)',
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: '0.05em',
            color: sig.color,
            background: sig.bg,
            padding: '2px 7px',
            borderRadius: 4,
            flexShrink: 0,
          }}>
            {sig.label}
          </span>
        </div>

        {/* Metric bars */}
        <div style={{ display: 'grid', gap: 5 }}>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontFamily: 'var(--mono-deck)', fontSize: 10, color: 'var(--ink-3)' }}>CPU</span>
              <span style={{ fontFamily: 'var(--mono-deck)', fontSize: 10, color: SIG[cpuSig].color, fontFeatureSettings: '"tnum"' }}>
                {vm.cpuPct.toFixed(0)}%
              </span>
            </div>
            <Bar pct={vm.cpuPct} color={SIG[cpuSig].color} />
          </div>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontFamily: 'var(--mono-deck)', fontSize: 10, color: 'var(--ink-3)' }}>MEM</span>
              <span style={{ fontFamily: 'var(--mono-deck)', fontSize: 10, color: SIG[memSig].color, fontFeatureSettings: '"tnum"' }}>
                {vm.memPct.toFixed(0)}%
              </span>
            </div>
            <Bar pct={vm.memPct} color={SIG[memSig].color} />
          </div>
        </div>

        {/* Footer */}
        {vm.netMbps > 0 && (
          <div style={{
            marginTop: 8,
            fontFamily: 'var(--mono-deck)',
            fontSize: 10,
            color: 'var(--ink-3)',
            letterSpacing: '0.04em',
          }}>
            net {vm.netMbps.toFixed(1)} Mb/s
          </div>
        )}
      </div>
    </div>
  );
}
