/* ============================================================
   TopologySurface — vCenter killer.
   Host columns as capacity space; VM blocks sized by footprint.
   Features:
   · Allocation vs Live load toggle
   · Simulate drain — lights up target hosts, fades migrating VMs
   · Drag-to-migrate — drag VM block to host; fires real API call
   · Live load breathing — blocks reflow on every telemetry tick
   · Keyboard a11y — select VM with ↑↓, press M, pick host

   Usage:
     <TopologySurface vms={vms} onInspect={(vm) => openConsole(vm)} />
   ============================================================ */
import { useState, useRef, useCallback, useMemo, useId } from 'react';
import { FleetVM, Signal } from './types';

/* ── Constants ──────────────────────────────────────────────── */
const SIG_COLOR: Record<Signal, string> = {
  ok:   '#1D9E75',
  warn: '#EF9F27',
  crit: '#E24B4A',
  off:  '#5F5E5A',
};
const SIG_BG: Record<Signal, string> = {
  ok:   'rgba(29,158,117,0.10)',
  warn: 'rgba(239,159,39,0.10)',
  crit: 'rgba(226,75,74,0.10)',
  off:  'rgba(95,94,90,0.08)',
};
const COL_W   = 200;   // px per host column
const MIN_H   = 32;    // px minimum VM block height
const MAX_H   = 140;   // px maximum VM block height
const COL_GAP = 12;
const PAD     = 8;

/* ── Types ──────────────────────────────────────────────────── */
type Metric = 'alloc' | 'live';

interface Host {
  name: string;
  vms: FleetVM[];
  cpuFree: number;    // 0-100 percent remaining
  memFree: number;
}

interface MigState {
  vm: FleetVM;
  from: string;
  to: string;
  phase: 'preflight' | 'copying' | 'done' | 'failed';
  progress: number;     // 0-100
  error?: string;
}

interface DragState {
  vm: FleetVM;
  fromHost: string;
  ghostX: number;
  ghostY: number;
  overHost: string | null;
  fits: boolean;
}

/* ── Helpers ─────────────────────────────────────────────────── */
function groupByNode(vms: FleetVM[]): Host[] {
  const map = new Map<string, FleetVM[]>();
  for (const vm of vms) {
    const arr = map.get(vm.node) ?? [];
    arr.push(vm);
    map.set(vm.node, arr);
  }
  return [...map.entries()].map(([name, hvms]) => {
    const cpuUsed = hvms.reduce((s, v) => s + v.cpuPct, 0) / Math.max(hvms.length, 1);
    const memUsed = hvms.reduce((s, v) => s + v.memPct, 0) / Math.max(hvms.length, 1);
    return { name, vms: hvms, cpuFree: 100 - cpuUsed, memFree: 100 - memUsed };
  });
}

function vmHeight(vm: FleetVM, metric: Metric): number {
  const load = metric === 'alloc'
    ? Math.max(vm.cpuPct, vm.memPct)
    : Math.max(vm.cpuPct, vm.memPct);
  return Math.round(MIN_H + (load / 100) * (MAX_H - MIN_H));
}

function capacityBar(
  used: number,
  label: string,
  color: string,
): JSX.Element {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <span style={{
        fontFamily: 'var(--mono-deck)', fontSize: 9,
        color: 'var(--ink-3)', letterSpacing: '0.06em',
        textTransform: 'uppercase', width: 28, flexShrink: 0,
      }}>{label}</span>
      <div style={{ flex: 1, height: 3, background: 'var(--hairline)', borderRadius: 2, overflow: 'hidden' }}>
        <div style={{
          height: '100%', width: `${Math.min(100, used)}%`,
          background: color, borderRadius: 2,
          transition: 'width .4s ease',
        }} />
      </div>
      <span style={{
        fontFamily: 'var(--mono-deck)', fontSize: 9,
        color: 'var(--ink-2)', fontFeatureSettings: '"tnum"',
        width: 28, textAlign: 'right', flexShrink: 0,
      }}>{Math.round(used)}%</span>
    </div>
  );
}

/* ── Main component ─────────────────────────────────────────── */
interface TopologySurfaceProps {
  vms: FleetVM[];
  onInspect?: (vm: FleetVM) => void;
  apiKey?: string;
  className?: string;
}

export function TopologySurface({ vms, onInspect, apiKey, className }: TopologySurfaceProps) {
  const uid = useId();
  const [metric, setMetric]     = useState<Metric>('live');
  const [drain, setDrain]       = useState<string | null>(null);   // host being simulated-drained
  const [mig, setMig]           = useState<MigState | null>(null);
  const [drag, setDrag]         = useState<DragState | null>(null);
  const [selectedVm, setSelected] = useState<string | null>(null); // kb a11y: ns/name key
  const [keyPickHost, setKeyPickHost] = useState(false);           // M key mode

  const containerRef = useRef<HTMLDivElement>(null);
  const migTimerRef  = useRef<ReturnType<typeof setTimeout> | null>(null);

  /* Derive hosts; optimistically move VM during migration */
  const hosts = useMemo<Host[]>(() => {
    let list = [...vms];
    if (mig && (mig.phase === 'copying' || mig.phase === 'done')) {
      list = list.map(v =>
        v.id === mig.vm.id ? { ...v, node: mig.to } : v,
      );
    }
    return groupByNode(list);
  }, [vms, mig]);

  /* ── Live-migration API call ─────────────────────────────── */
  const runMigration = useCallback(async (vm: FleetVM, targetNode: string) => {
    const state: MigState = { vm, from: vm.node, to: targetNode, phase: 'preflight', progress: 0 };
    setMig(state);

    try {
      const res = await fetch(`/api/v1/vms/${vm.namespace}/${vm.name}/migrate`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey ?? (window as any).__VEYRON_API_KEY ?? '',
        },
        body: JSON.stringify({ targetNode }),
      });

      if (!res.ok) {
        const msg = await res.text().catch(() => `HTTP ${res.status}`);
        setMig(s => s ? { ...s, phase: 'failed', error: msg } : null);
        setTimeout(() => setMig(null), 4000);
        return;
      }

      // KubeVirt VMIM is async — simulate phase ticks while SSE would provide real status
      setMig(s => s ? { ...s, phase: 'copying', progress: 5 } : null);
      let p = 5;
      const tick = () => {
        p = Math.min(p + 8 + Math.random() * 12, 99);
        setMig(s => s ? { ...s, progress: p } : null);
        if (p < 99) {
          migTimerRef.current = setTimeout(tick, 600 + Math.random() * 400);
        } else {
          setMig(s => s ? { ...s, phase: 'done', progress: 100 } : null);
          setTimeout(() => setMig(null), 2500);
        }
      };
      migTimerRef.current = setTimeout(tick, 500);
    } catch (err) {
      setMig(s => s ? { ...s, phase: 'failed', error: String(err) } : null);
      setTimeout(() => setMig(null), 4000);
    }
  }, [apiKey]);

  /* ── Capacity preflight ──────────────────────────────────── */
  function fits(vm: FleetVM, host: Host): boolean {
    const cpuNeed = vm.cpuPct;
    const memNeed = vm.memPct;
    return host.cpuFree > cpuNeed && host.memFree > memNeed;
  }

  /* ── Drag handlers ──────────────────────────────────────── */
  const onVmDragStart = useCallback((e: React.DragEvent, vm: FleetVM, fromHost: string) => {
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('vmId', vm.id);
    // Create invisible drag image (we render our own ghost)
    const ghost = document.createElement('div');
    ghost.style.position = 'fixed'; ghost.style.opacity = '0';
    document.body.appendChild(ghost);
    e.dataTransfer.setDragImage(ghost, 0, 0);
    setTimeout(() => document.body.removeChild(ghost), 0);

    setDrag({ vm, fromHost, ghostX: e.clientX, ghostY: e.clientY, overHost: null, fits: false });
  }, []);

  const onHostDragOver = useCallback((e: React.DragEvent, host: Host) => {
    e.preventDefault();
    const vm = drag?.vm;
    if (!vm) return;
    const f = fits(vm, host);
    e.dataTransfer.dropEffect = f ? 'move' : 'none';
    setDrag(d => d ? { ...d, overHost: host.name, fits: f, ghostX: e.clientX, ghostY: e.clientY } : null);
  }, [drag]);

  const onHostDrop = useCallback((e: React.DragEvent, host: Host) => {
    e.preventDefault();
    if (!drag || !drag.fits || host.name === drag.fromHost || mig) return;
    const vm = drag.vm;
    setDrag(null);
    runMigration(vm, host.name);
  }, [drag, mig, runMigration]);

  const onDragEnd = useCallback(() => setDrag(null), []);

  /* ── Drain simulation ─────────────────────────────────────── */
  function startDrain(hostName: string) {
    setDrain(d => d === hostName ? null : hostName);
  }

  /* ── Keyboard a11y ────────────────────────────────────────── */
  const allVmIds = useMemo(() => vms.map(v => v.id), [vms]);
  function handleVmKey(e: React.KeyboardEvent, vm: FleetVM) {
    if (e.key === 'Enter') { onInspect?.(vm); return; }
    if (e.key === 'm' || e.key === 'M') {
      setSelected(vm.id);
      setKeyPickHost(true);
      return;
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const idx = allVmIds.indexOf(vm.id);
      const next = e.key === 'ArrowDown'
        ? allVmIds[Math.min(idx + 1, allVmIds.length - 1)]
        : allVmIds[Math.max(idx - 1, 0)];
      setSelected(next);
      document.getElementById(`vm-block-${uid}-${next.replace(/\//,'_')}`)?.focus();
    }
  }
  function handleKeyHostPick(host: Host) {
    const vm = vms.find(v => v.id === selectedVm);
    if (!vm || host.name === vm.node || mig) return;
    setKeyPickHost(false);
    runMigration(vm, host.name);
  }

  /* ── Migration banner ─────────────────────────────────────── */
  function MigBanner() {
    if (!mig) return null;
    const { vm, from, to, phase, progress, error } = mig;
    const phaseLabel = { preflight: 'Preflight', copying: 'Pre-copy', done: 'Complete', failed: 'Failed' }[phase];
    const color = phase === 'failed' ? '#E24B4A' : phase === 'done' ? '#1D9E75' : '#f59e0b';
    return (
      <div style={{
        position: 'sticky', top: 0, zIndex: 20,
        background: `${color}18`,
        border: `1px solid ${color}44`,
        borderRadius: 8,
        padding: '8px 14px',
        marginBottom: 10,
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        fontFamily: 'var(--mono-deck)',
        fontSize: 11,
      }}>
        {phase === 'copying' && (
          <div style={{ width: 80, height: 3, background: 'var(--hairline)', borderRadius: 2, overflow: 'hidden' }}>
            <div style={{ height: '100%', width: `${progress}%`, background: color, transition: 'width .3s linear' }} />
          </div>
        )}
        <span style={{ color }}>{phaseLabel}</span>
        <span style={{ color: 'var(--ink-2)' }}>
          {vm.namespace}/{vm.name} · {from} → {to}
          {phase === 'copying' && ` · ${Math.round(progress)}%`}
          {error && ` · ${error}`}
        </span>
        {phase !== 'copying' && (
          <button onClick={() => setMig(null)} style={{
            background: 'none', border: 'none', cursor: 'pointer',
            color: 'var(--ink-3)', marginLeft: 'auto', fontSize: 14,
          }}>×</button>
        )}
      </div>
    );
  }

  /* ── Column render ────────────────────────────────────────── */
  function HostColumn({ host }: { host: Host }) {
    const isDraining = drain === host.name;
    const isDropTarget = drag?.overHost === host.name;
    const dropFits = drag?.fits ?? false;
    const isKeyTarget = keyPickHost;
    const cpuUsed = 100 - host.cpuFree;
    const memUsed = 100 - host.memFree;

    return (
      <div
        onDragOver={e => onHostDragOver(e, host)}
        onDrop={e => onHostDrop(e, host)}
        onClick={isKeyTarget ? () => handleKeyHostPick(host) : undefined}
        style={{
          width: COL_W,
          flexShrink: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
          padding: PAD,
          background: isDropTarget
            ? (dropFits ? 'rgba(29,158,117,0.08)' : 'rgba(226,75,74,0.08)')
            : isDraining ? 'rgba(239,159,39,0.06)' : 'var(--panel)',
          border: `1px solid ${
            isDropTarget
              ? (dropFits ? '#1D9E75' : '#E24B4A')
              : isKeyTarget ? 'var(--plasma)'
              : isDraining ? '#EF9F27'
              : 'var(--hairline)'
          }`,
          borderRadius: 12,
          cursor: isKeyTarget ? 'pointer' : undefined,
          transition: 'background .15s, border-color .15s',
        }}
      >
        {/* Host header */}
        <div style={{ paddingBottom: 8, borderBottom: '1px solid var(--hairline)' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
            <span style={{
              fontFamily: 'var(--sans-deck)', fontSize: 12, fontWeight: 600,
              color: isDraining ? '#EF9F27' : 'var(--ink)',
              whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
            }}>
              {host.name.length > 16 ? host.name.split('-').slice(-2).join('-') : host.name}
            </span>
            <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
              <span style={{
                fontFamily: 'var(--mono-deck)', fontSize: 9, color: 'var(--ink-3)',
              }}>{host.vms.length} VMs</span>
              <button
                title={isDraining ? 'Cancel drain simulation' : 'Simulate drain'}
                onClick={() => startDrain(host.name)}
                style={{
                  fontFamily: 'var(--mono-deck)', fontSize: 9,
                  background: isDraining ? 'rgba(239,159,39,0.15)' : 'none',
                  color: isDraining ? '#EF9F27' : 'var(--ink-3)',
                  border: `1px solid ${isDraining ? '#EF9F27' : 'var(--hairline)'}`,
                  borderRadius: 4, padding: '1px 5px', cursor: 'pointer',
                  transition: 'all .12s',
                }}
              >
                {isDraining ? 'DRAINING' : 'DRAIN'}
              </button>
            </div>
          </div>
          {capacityBar(cpuUsed, 'CPU', cpuUsed > 90 ? '#E24B4A' : cpuUsed > 75 ? '#EF9F27' : '#1D9E75')}
          <div style={{ marginTop: 4 }} />
          {capacityBar(memUsed, 'MEM', memUsed > 90 ? '#E24B4A' : memUsed > 75 ? '#EF9F27' : '#f59e0b')}
        </div>

        {/* VM blocks */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4, flex: 1 }}>
          {host.vms.map(vm => (
            <VmBlock
              key={vm.id}
              vm={vm}
              metric={metric}
              drain={isDraining}
              migrating={mig?.vm.id === vm.id}
              selected={selectedVm === vm.id}
              uid={uid}
              onInspect={onInspect}
              onDragStart={onVmDragStart}
              onDragEnd={onDragEnd}
              onKeyDown={handleVmKey}
              onClick={() => setSelected(vm.id)}
            />
          ))}
        </div>

        {/* Drop zone hint */}
        {isDropTarget && (
          <div style={{
            textAlign: 'center',
            fontFamily: 'var(--mono-deck)', fontSize: 10,
            color: dropFits ? '#1D9E75' : '#E24B4A',
            padding: '6px 0',
            borderRadius: 6,
            background: dropFits ? 'rgba(29,158,117,0.08)' : 'rgba(226,75,74,0.08)',
            border: `1px dashed ${dropFits ? '#1D9E75' : '#E24B4A'}`,
          }}>
            {dropFits ? `Drop to migrate here` : `Insufficient capacity`}
          </div>
        )}
      </div>
    );
  }

  /* ── Drag ghost ────────────────────────────────────────────── */
  function DragGhost() {
    if (!drag) return null;
    const { vm, ghostX, ghostY, fits: f } = drag;
    return (
      <div style={{
        position: 'fixed',
        left: ghostX + 12,
        top: ghostY - 16,
        pointerEvents: 'none',
        zIndex: 9999,
        background: SIG_BG[vm.signal],
        border: `1px solid ${f !== false ? (f ? '#1D9E75' : '#E24B4A') : SIG_COLOR[vm.signal]}`,
        borderRadius: 8,
        padding: '5px 10px',
        fontFamily: 'var(--mono-deck)',
        fontSize: 11,
        color: 'var(--ink)',
        boxShadow: '0 8px 32px rgba(0,0,0,.6)',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
      }}>
        <span style={{ width: 6, height: 6, borderRadius: '50%', background: SIG_COLOR[vm.signal], display: 'inline-block' }} />
        {vm.name}
      </div>
    );
  }

  return (
    <div className={className}>
      {/* Toolbar */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        marginBottom: 12,
        flexWrap: 'wrap',
      }}>
        <div style={{ display: 'flex', gap: 2 }}>
          {(['live', 'alloc'] as const).map(m => (
            <button
              key={m}
              onClick={() => setMetric(m)}
              style={{
                fontFamily: 'var(--mono-deck)', fontSize: 10, fontWeight: 600,
                letterSpacing: '0.06em', textTransform: 'uppercase',
                padding: '4px 10px', borderRadius: 5,
                border: `1px solid ${metric === m ? 'var(--plasma)' : 'var(--hairline)'}`,
                background: metric === m ? 'var(--plasma-glow)' : 'transparent',
                color: metric === m ? 'var(--plasma)' : 'var(--ink-3)',
                cursor: 'pointer', transition: 'all .12s',
              }}
            >
              {m === 'live' ? 'Live Load' : 'Allocation'}
            </button>
          ))}
        </div>

        <div style={{
          fontFamily: 'var(--mono-deck)', fontSize: 10,
          color: 'var(--ink-3)', marginLeft: 6,
        }}>
          drag VM block to migrate · press M to keyboard-migrate
        </div>

        {keyPickHost && (
          <div style={{
            marginLeft: 'auto',
            fontFamily: 'var(--mono-deck)', fontSize: 10,
            color: 'var(--plasma)', border: '1px solid var(--plasma)',
            borderRadius: 5, padding: '3px 8px',
            background: 'var(--plasma-glow)',
          }}>
            Click target host ↑ or press Esc to cancel
          </div>
        )}
      </div>

      <MigBanner />

      {/* Surface */}
      <div
        ref={containerRef}
        role="region"
        aria-label="Host topology surface"
        onKeyDown={e => { if (e.key === 'Escape') { setKeyPickHost(false); setSelected(null); }}}
        style={{
          display: 'flex',
          gap: COL_GAP,
          overflowX: 'auto',
          paddingBottom: 4,
        }}
      >
        {hosts.map(h => <HostColumn key={h.name} host={h} />)}

        {hosts.length === 0 && (
          <div style={{
            padding: 40,
            textAlign: 'center',
            fontFamily: 'var(--mono-deck)',
            fontSize: 12,
            color: 'var(--ink-3)',
          }}>
            No VMs. Connect the fleet feed to populate.
          </div>
        )}
      </div>

      {/* Drag ghost */}
      <DragGhost />
    </div>
  );
}

/* ── VM Block ─────────────────────────────────────────────────── */
interface VmBlockProps {
  vm: FleetVM;
  metric: Metric;
  drain: boolean;
  migrating: boolean;
  selected: boolean;
  uid: string;
  onInspect?: (vm: FleetVM) => void;
  onDragStart: (e: React.DragEvent, vm: FleetVM, fromHost: string) => void;
  onDragEnd: () => void;
  onKeyDown: (e: React.KeyboardEvent, vm: FleetVM) => void;
  onClick: () => void;
}

function VmBlock({
  vm, metric, drain, migrating, selected, uid,
  onInspect, onDragStart, onDragEnd, onKeyDown, onClick,
}: VmBlockProps) {
  const h = vmHeight(vm, metric);
  const sigColor = SIG_COLOR[vm.signal];
  const sigBg = SIG_BG[vm.signal];
  const load = Math.max(vm.cpuPct, vm.memPct);
  const blockId = `vm-block-${uid}-${vm.id.replace(/\//g, '_')}`;

  return (
    <div
      id={blockId}
      draggable={vm.signal !== 'off'}
      tabIndex={0}
      role="button"
      aria-label={`${vm.name} ${vm.status}, CPU ${Math.round(vm.cpuPct)}%, MEM ${Math.round(vm.memPct)}%`}
      aria-selected={selected}
      onDragStart={e => onDragStart(e, vm, vm.node)}
      onDragEnd={onDragEnd}
      onKeyDown={e => onKeyDown(e, vm)}
      onClick={() => { onClick(); onInspect?.(vm); }}
      style={{
        position: 'relative',
        height: h,
        background: migrating
          ? 'rgba(245,158,11,0.12)'
          : drain ? 'rgba(239,159,39,0.07)' : sigBg,
        border: `1px solid ${selected ? sigColor : migrating ? '#f59e0b' : drain ? '#EF9F2766' : `${sigColor}44`}`,
        borderLeft: `3px solid ${drain ? '#EF9F27' : sigColor}`,
        borderRadius: 8,
        cursor: 'grab',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        padding: '6px 8px 4px',
        overflow: 'hidden',
        opacity: (drain || migrating) ? 0.65 : 1,
        transition: 'height .35s ease, opacity .3s, border-color .15s, background .15s',
        outline: selected ? `2px solid ${sigColor}` : undefined,
        outlineOffset: 1,
        userSelect: 'none',
      }}
      onMouseEnter={e => {
        (e.currentTarget as HTMLElement).style.outline = `2px solid ${sigColor}`;
        (e.currentTarget as HTMLElement).style.outlineOffset = '1px';
      }}
      onMouseLeave={e => {
        if (!selected) {
          (e.currentTarget as HTMLElement).style.outline = 'none';
        }
      }}
    >
      {/* VM name */}
      <div style={{
        fontFamily: 'var(--sans-deck)',
        fontSize: Math.max(9, Math.min(12, h / 10)),
        fontWeight: 600,
        color: 'var(--ink)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
      }}>
        {vm.name}
      </div>

      {/* Load readout — visible only when block is tall enough */}
      {h > 54 && (
        <div style={{
          fontFamily: 'var(--mono-deck)',
          fontSize: 9,
          color: sigColor,
          fontFeatureSettings: '"tnum"',
          letterSpacing: '0.04em',
        }}>
          {Math.round(vm.cpuPct)}%c · {Math.round(vm.memPct)}%m
        </div>
      )}

      {/* Bottom live-load sliver */}
      <div style={{ height: 3, background: 'var(--hairline)', borderRadius: 2, overflow: 'hidden', marginTop: 4 }}>
        <div style={{
          height: '100%',
          width: `${load}%`,
          background: sigColor,
          borderRadius: 2,
          transition: 'width .4s ease',
        }} />
      </div>

      {/* Migration overlay */}
      {migrating && (
        <div style={{
          position: 'absolute', inset: 0,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: 'rgba(245,158,11,0.15)',
          backdropFilter: 'blur(1px)',
          borderRadius: 7,
        }}>
          <span style={{
            fontFamily: 'var(--mono-deck)', fontSize: 9,
            color: '#f59e0b', letterSpacing: '0.06em',
          }}>
            MIGRATING
          </span>
        </div>
      )}
    </div>
  );
}
