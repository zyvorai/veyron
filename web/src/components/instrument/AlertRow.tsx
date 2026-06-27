/* ============================================================
   AlertRow — single alert item with signal icon.
   Usage: <AlertRow level="crit" title="CPU spike" meta="node-01 · 2m ago" />
   ============================================================ */

type Level = 'warn' | 'crit' | 'info';

const LEVEL: Record<Level, { color: string; bg: string; icon: string; label: string }> = {
  crit: { color: '#E24B4A', bg: 'rgba(226,75,74,0.10)',  icon: '●', label: 'Critical' },
  warn: { color: '#EF9F27', bg: 'rgba(239,159,39,0.10)', icon: '◆', label: 'Warning'  },
  info: { color: '#378ADD', bg: 'rgba(55,138,221,0.10)', icon: '◉', label: 'Info'     },
};

interface AlertRowProps {
  level: Level;
  title: string;
  meta?: string;
  timestamp?: string;
  onDismiss?: () => void;
  onClick?: () => void;
  className?: string;
}

export function AlertRow({ level, title, meta, timestamp, onDismiss, onClick, className }: AlertRowProps) {
  const l = LEVEL[level];
  return (
    <div
      className={className}
      onClick={onClick}
      role={onClick ? 'button' : 'listitem'}
      tabIndex={onClick ? 0 : undefined}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '10px 12px',
        background: l.bg,
        border: `1px solid ${l.color}33`,
        borderLeft: `3px solid ${l.color}`,
        borderRadius: 8,
        cursor: onClick ? 'pointer' : undefined,
        transition: 'background .15s',
      }}
    >
      {/* Icon */}
      <span style={{
        fontFamily: 'var(--mono-deck)',
        fontSize: 12,
        color: l.color,
        flexShrink: 0,
        lineHeight: 1,
      }} aria-hidden="true">
        {l.icon}
      </span>

      {/* Body */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
          fontFamily: 'var(--sans-deck)',
          fontSize: 13,
          fontWeight: 500,
          color: 'var(--ink)',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}>
          {title}
        </div>
        {(meta || timestamp) && (
          <div style={{
            fontFamily: 'var(--mono-deck)',
            fontSize: 10,
            color: 'var(--ink-3)',
            marginTop: 2,
            letterSpacing: '0.04em',
          }}>
            {[meta, timestamp].filter(Boolean).join(' · ')}
          </div>
        )}
      </div>

      {/* Level chip */}
      <span style={{
        fontFamily: 'var(--mono-deck)',
        fontSize: 9,
        fontWeight: 700,
        letterSpacing: '0.08em',
        textTransform: 'uppercase',
        color: l.color,
        flexShrink: 0,
      }}>
        {l.label}
      </span>

      {/* Dismiss */}
      {onDismiss && (
        <button
          onClick={e => { e.stopPropagation(); onDismiss(); }}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--ink-3)',
            fontSize: 14,
            lineHeight: 1,
            padding: '0 2px',
            flexShrink: 0,
          }}
          aria-label="Dismiss alert"
        >
          ×
        </button>
      )}
    </div>
  );
}
