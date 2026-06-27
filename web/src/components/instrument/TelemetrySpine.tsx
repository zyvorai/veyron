/* ============================================================
   TelemetrySpine — animated status bar with live fleet metrics.
   Mirrors the id-spine bar in dashboard.html as a React component.
   Usage:
     <TelemetrySpine vms={vms} connected={connected} alerts={3} />
   ============================================================ */
import { useEffect, useRef } from 'react';
import { FleetVM } from './types';

interface TelemetrySpineProps {
  vms: FleetVM[];
  connected: boolean;
  alerts?: number;
  className?: string;
}

export function TelemetrySpine({ vms, connected, alerts = 0, className }: TelemetrySpineProps) {
  const scanRef = useRef<HTMLDivElement>(null);

  const running = vms.filter(v => v.signal === 'ok' || v.signal === 'warn').length;
  const avgCpu = vms.length
    ? vms.reduce((s, v) => s + v.cpuPct, 0) / vms.length
    : 0;
  const avgMem = vms.length
    ? vms.reduce((s, v) => s + v.memPct, 0) / vms.length
    : 0;
  const avgNet = vms.length
    ? vms.reduce((s, v) => s + v.netMbps, 0) / vms.length
    : 0;

  const hasAlerts = alerts > 0;

  // Blink the scan line every 1.4s to mirror dashboard behaviour
  useEffect(() => {
    const el = scanRef.current;
    if (!el) return;
    let on = true;
    const t = setInterval(() => {
      on = !on;
      el.style.opacity = on ? '1' : '0.3';
    }, 700);
    return () => clearInterval(t);
  }, []);

  return (
    <div
      className={className}
      role="status"
      aria-live="polite"
      aria-label="Fleet telemetry spine"
      style={{
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        gap: 0,
        height: 36,
        background: 'var(--hull)',
        borderBottom: '1px solid var(--hairline)',
        overflow: 'hidden',
        fontFamily: 'var(--mono-deck)',
        fontSize: 11,
        letterSpacing: '0.05em',
      }}
    >
      {/* Plasma scan sweep */}
      <div
        ref={scanRef}
        aria-hidden="true"
        style={{
          position: 'absolute',
          inset: 0,
          background: 'linear-gradient(90deg, transparent 0%, var(--plasma-glow) 50%, transparent 100%)',
          backgroundSize: '200% 100%',
          animation: 'id-scan-sweep 4s linear infinite',
          pointerEvents: 'none',
        }}
      />

      {/* LIVE badge */}
      <div style={{
        padding: '0 14px',
        borderRight: '1px solid var(--hairline)',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        flexShrink: 0,
      }}>
        <span style={{
          width: 6, height: 6, borderRadius: '50%',
          background: connected ? 'var(--nominal)' : 'var(--inert)',
          boxShadow: connected ? '0 0 6px var(--nominal)' : undefined,
          display: 'inline-block',
          flexShrink: 0,
        }} />
        <span style={{ color: connected ? 'var(--nominal)' : 'var(--ink-3)', fontWeight: 600 }}>
          {connected ? 'LIVE' : 'OFFLINE'}
        </span>
      </div>

      {/* VM count */}
      <Seg label="VMs">
        <b style={{ color: 'var(--ink)', fontWeight: 600 }}>{running}</b>&nbsp;running
      </Seg>

      {/* CPU */}
      <Seg label="CPU">
        cpu&nbsp;<b style={{ color: cpuColor(avgCpu), fontFeatureSettings: '"tnum"' }}>
          {avgCpu.toFixed(0)}%
        </b>
      </Seg>

      {/* MEM */}
      <Seg label="MEM">
        mem&nbsp;<b style={{ color: memColor(avgMem), fontFeatureSettings: '"tnum"' }}>
          {avgMem.toFixed(0)}%
        </b>
      </Seg>

      {/* NET */}
      <Seg label="NET">
        net&nbsp;<b style={{ color: 'var(--ink)', fontFeatureSettings: '"tnum"' }}>
          {avgNet.toFixed(1)}
        </b>&nbsp;Mb/s
      </Seg>

      {/* Alerts */}
      {hasAlerts && (
        <Seg label="ALERTS">
          <span style={{
            width: 6, height: 6, borderRadius: '50%',
            background: 'var(--caution)',
            display: 'inline-block',
            marginRight: 4,
          }} />
          <b style={{ color: 'var(--caution)', fontFeatureSettings: '"tnum"' }}>{alerts}</b>&nbsp;alerts
        </Seg>
      )}
    </div>
  );
}

function Seg({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      aria-label={label}
      style={{
        padding: '0 16px',
        borderRight: '1px solid var(--hairline)',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        color: 'var(--ink-2)',
        gap: 2,
        flexShrink: 0,
      }}
    >
      {children}
    </div>
  );
}

function cpuColor(pct: number) {
  if (pct > 90) return 'var(--critical)';
  if (pct > 75) return 'var(--caution)';
  return 'var(--nominal)';
}

function memColor(pct: number) {
  if (pct > 90) return 'var(--critical)';
  if (pct > 75) return 'var(--caution)';
  return 'var(--ink)';
}
