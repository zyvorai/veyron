/* ============================================================
   ZeusPanel — Ask Zeus assistant panel with plasma orb.
   In the dashboard this wires to openCopilotWithVm().
   Usage:
     <ZeusPanel
       context={{ ns: 'default', vm: 'web-01' }}
       onSend={(msg) => window.openCopilotWithVm(msg, ns, vm)}
     />
   ============================================================ */
import { useState, useRef } from 'react';

interface ZeusPanelProps {
  context?: { ns?: string; vm?: string; node?: string };
  onSend?: (message: string, context?: { ns?: string; vm?: string }) => void;
  placeholder?: string;
  className?: string;
}

export function ZeusPanel({ context, onSend, placeholder, className }: ZeusPanelProps) {
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const hints = context?.vm
    ? [`What's wrong with ${context.vm}?`, `Show metrics for ${context.vm}`, `Restart ${context.vm}`]
    : ['Show unhealthy VMs', 'Which node is overloaded?', 'Summarize alerts'];

  const submit = () => {
    const msg = value.trim();
    if (!msg || busy) return;
    setBusy(true);
    onSend?.(msg, { ns: context?.ns, vm: context?.vm });
    setValue('');
    // Let caller resolve busy state; reset after short delay if not externally controlled
    setTimeout(() => setBusy(false), 800);
  };

  return (
    <div
      className={className}
      style={{
        background: 'var(--panel)',
        border: '1px solid var(--hairline)',
        borderRadius: 16,
        overflow: 'hidden',
        transition: 'border-color .2s',
      }}
      onFocusWithin={() => {/* border glow handled by CSS var */}}
    >
      {/* Header */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '12px 16px 10px',
        borderBottom: '1px solid var(--hairline)',
      }}>
        {/* Plasma orb */}
        <div style={{ position: 'relative', width: 28, height: 28, flexShrink: 0 }}>
          <div style={{
            position: 'absolute',
            inset: 0,
            borderRadius: '50%',
            background: 'radial-gradient(circle at 38% 38%, #fcd9a0, var(--plasma) 55%, var(--plasma-deep))',
            boxShadow: '0 0 12px var(--plasma-glow)',
            animation: 'id-orb-breathe 3s ease-in-out infinite',
          }} aria-hidden="true" />
          <div style={{
            position: 'absolute',
            top: 4, left: 5,
            width: 7, height: 4,
            borderRadius: '50%',
            background: 'rgba(255,255,255,0.35)',
            transform: 'rotate(-30deg)',
          }} />
        </div>
        <div>
          <div style={{
            fontFamily: 'var(--sans-deck)',
            fontSize: 13,
            fontWeight: 600,
            color: 'var(--ink)',
          }}>
            Ask Zeus
          </div>
          {context?.vm && (
            <div style={{
              fontFamily: 'var(--mono-deck)',
              fontSize: 10,
              color: 'var(--ink-3)',
              letterSpacing: '0.04em',
            }}>
              {context.ns}/{context.vm}
            </div>
          )}
        </div>
      </div>

      {/* Hint chips */}
      <div style={{
        display: 'flex',
        gap: 6,
        padding: '10px 14px 8px',
        flexWrap: 'wrap',
      }}>
        {hints.map(h => (
          <button
            key={h}
            onClick={() => { setValue(h); inputRef.current?.focus(); }}
            style={{
              fontFamily: 'var(--mono-deck)',
              fontSize: 10,
              color: 'var(--ink-2)',
              background: 'var(--panel-2)',
              border: '1px solid var(--hairline)',
              borderRadius: 6,
              padding: '3px 8px',
              cursor: 'pointer',
              letterSpacing: '0.02em',
              transition: 'border-color .12s, color .12s',
            }}
            onMouseEnter={e => {
              (e.currentTarget as HTMLElement).style.borderColor = 'var(--plasma)';
              (e.currentTarget as HTMLElement).style.color = 'var(--plasma)';
            }}
            onMouseLeave={e => {
              (e.currentTarget as HTMLElement).style.borderColor = 'var(--hairline)';
              (e.currentTarget as HTMLElement).style.color = 'var(--ink-2)';
            }}
          >
            {h}
          </button>
        ))}
      </div>

      {/* Input row */}
      <div style={{
        display: 'flex',
        gap: 8,
        padding: '0 14px 14px',
      }}>
        <input
          ref={inputRef}
          value={value}
          onChange={e => setValue(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && submit()}
          placeholder={placeholder ?? 'Ask anything about your fleet…'}
          style={{
            flex: 1,
            fontFamily: 'var(--sans-deck)',
            fontSize: 13,
            color: 'var(--ink)',
            background: 'var(--panel-2)',
            border: '1px solid var(--hairline)',
            borderRadius: 8,
            padding: '8px 12px',
            outline: 'none',
            transition: 'border-color .15s',
          }}
          onFocus={e => { (e.target as HTMLElement).style.borderColor = 'var(--plasma)'; }}
          onBlur={e => { (e.target as HTMLElement).style.borderColor = 'var(--hairline)'; }}
          aria-label="Message to Zeus assistant"
        />
        <button
          onClick={submit}
          disabled={!value.trim() || busy}
          style={{
            fontFamily: 'var(--sans-deck)',
            fontSize: 12,
            fontWeight: 600,
            color: busy ? 'var(--ink-3)' : 'var(--void)',
            background: busy ? 'var(--hairline)' : 'var(--plasma)',
            border: 'none',
            borderRadius: 8,
            padding: '8px 16px',
            cursor: busy || !value.trim() ? 'default' : 'pointer',
            transition: 'background .15s',
            flexShrink: 0,
          }}
          aria-label="Send message"
        >
          {busy ? '…' : '⌘↵'}
        </button>
      </div>
    </div>
  );
}
