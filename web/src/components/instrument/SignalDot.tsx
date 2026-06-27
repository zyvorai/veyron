/* ============================================================
   SignalDot — status indicator with optional pulse animation.
   Usage: <SignalDot signal="ok" pulse label="Running" />
   ============================================================ */
import { Signal } from './types';

const TOKEN: Record<Signal, { color: string; bg: string }> = {
  ok:   { color: 'var(--nominal)',  bg: 'var(--nominal-bg)'  },
  warn: { color: 'var(--caution)', bg: 'var(--caution-bg)'  },
  crit: { color: 'var(--critical)',bg: 'var(--critical-bg)'  },
  off:  { color: 'var(--inert)',   bg: 'var(--inert-bg)'    },
};

interface SignalDotProps {
  signal: Signal;
  pulse?: boolean;
  size?: number;
  label?: string;
  className?: string;
}

export function SignalDot({ signal, pulse = false, size = 8, label, className }: SignalDotProps) {
  const { color } = TOKEN[signal];
  return (
    <span
      className={className}
      style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}
      aria-label={label ?? signal}
    >
      <span style={{ position: 'relative', width: size, height: size, flexShrink: 0 }}>
        {pulse && signal !== 'off' && (
          <span style={{
            position: 'absolute', inset: -3,
            borderRadius: '50%',
            background: color,
            opacity: 0.25,
            animation: 'id-pulse 2s ease-in-out infinite',
          }} />
        )}
        <span style={{
          display: 'block', width: size, height: size,
          borderRadius: '50%',
          background: color,
        }} />
      </span>
      {label && (
        <span style={{
          fontFamily: 'var(--mono-deck)',
          fontSize: 11,
          color: color,
          fontWeight: 500,
          letterSpacing: '0.04em',
        }}>
          {label}
        </span>
      )}
    </span>
  );
}
