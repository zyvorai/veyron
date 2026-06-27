/* ============================================================
   Chip — filter/tag chip with active/idle states.
   Usage:
     <Chip label="Running" active={filter === 'ok'} onClick={() => setFilter('ok')} />
     <Chip label="3" dot="warn" active />
   ============================================================ */
import { Signal } from './types';

const DOT_COLOR: Record<Signal, string> = {
  ok:   'var(--nominal)',
  warn: 'var(--caution)',
  crit: 'var(--critical)',
  off:  'var(--inert)',
};

interface ChipProps {
  label: string;
  active?: boolean;
  dot?: Signal;
  count?: number;
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
}

export function Chip({ label, active = false, dot, count, onClick, disabled = false, className }: ChipProps) {
  return (
    <button
      className={className}
      onClick={onClick}
      disabled={disabled}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5,
        fontFamily: 'var(--mono-deck)',
        fontSize: 11,
        fontWeight: active ? 600 : 400,
        letterSpacing: '0.04em',
        color: active ? 'var(--ink)' : 'var(--ink-3)',
        background: active ? 'var(--panel-hi)' : 'transparent',
        border: `1px solid ${active ? 'var(--hairline-hi)' : 'var(--hairline)'}`,
        borderRadius: 6,
        padding: '4px 10px',
        cursor: disabled ? 'default' : 'pointer',
        opacity: disabled ? 0.4 : 1,
        transition: 'color .12s, background .12s, border-color .12s',
        outline: 'none',
        userSelect: 'none',
      }}
      onMouseEnter={e => {
        if (!disabled) {
          const el = e.currentTarget as HTMLButtonElement;
          if (!active) {
            el.style.color = 'var(--ink-2)';
            el.style.borderColor = 'var(--hairline-hi)';
          }
        }
      }}
      onMouseLeave={e => {
        if (!disabled) {
          const el = e.currentTarget as HTMLButtonElement;
          el.style.color = active ? 'var(--ink)' : 'var(--ink-3)';
          el.style.borderColor = active ? 'var(--hairline-hi)' : 'var(--hairline)';
        }
      }}
      aria-pressed={active}
    >
      {dot && (
        <span style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: DOT_COLOR[dot],
          flexShrink: 0,
          display: 'inline-block',
        }} aria-hidden="true" />
      )}
      {label}
      {count !== undefined && (
        <span style={{
          fontSize: 10,
          color: 'var(--ink-3)',
          fontFeatureSettings: '"tnum"',
        }}>
          {count}
        </span>
      )}
    </button>
  );
}
