/* ============================================================
   web/src/components/instrument/ReactorCore.tsx
   The fleet as one living instrument. Canvas-rendered so it
   scales past ~60 VMs without dropping frames.

   Usage:
     const { vms } = useFleetTelemetry();
     <ReactorCore vms={vms} onInspect={(vm) => openConsole(vm)} />
   ============================================================ */
import { useEffect, useRef } from 'react';
import { FleetVM, Signal, fleetHealth, groupByNode } from './types';

// Instrument Deck signal palette — matches CSS vars exactly.
// Canvas cannot read CSS vars, so we keep a JS-side copy.
const PAL: Record<Signal | 'plasma' | 'deep' | 'ink', string> = {
  ok:     '#1D9E75',  // --nominal
  warn:   '#EF9F27',  // --caution
  crit:   '#E24B4A',  // --critical
  off:    '#5F5E5A',  // --inert
  plasma: '#f59e0b',
  deep:   '#b45309',
  ink:    '#5f6884',
};

interface Particle {
  vm: FleetVM;
  ring: number;
  ang: number;
  phase: number;
  _sx?: number;
  _sy?: number;
  _r?: number;
}

interface RingSpec {
  node: string;
  r: number;
  speed: number;
  tilt: number;
  dead: boolean;
}

export interface ReactorCoreProps {
  vms: FleetVM[];
  ringRadii?: number[];
  onInspect?: (vm: FleetVM) => void;
  failedNode?: string | null;
  className?: string;
}

export function ReactorCore({
  vms,
  ringRadii = [95, 140, 185, 225],
  onInspect,
  failedNode = null,
  className,
}: ReactorCoreProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const dataRef = useRef({ vms, failedNode });
  dataRef.current = { vms, failedNode };

  const partsRef = useRef<Particle[]>([]);
  const ringsRef = useRef<RingSpec[]>([]);
  const hoverRef = useRef<Particle | null>(null);

  // Rebuild orbital layout when node set changes; preserve particle angles.
  useEffect(() => {
    const groups = groupByNode(vms);
    ringsRef.current = groups.map((g, i) => ({
      node: g.node,
      r: ringRadii[Math.min(i, ringRadii.length - 1)],
      speed: (i % 2 ? -1 : 1) * (0.45 - i * 0.07),
      tilt: 0.34,
      dead: g.node === failedNode,
    }));

    const next: Particle[] = [];
    groups.forEach((g, ri) => {
      g.vms.forEach((vm, i) => {
        const prev = partsRef.current.find(p => p.vm.id === vm.id);
        next.push({
          vm,
          ring: ri,
          ang: prev?.ang ?? (i / Math.max(g.vms.length, 1)) * Math.PI * 2,
          phase: prev?.phase ?? Math.random() * 6.28,
        });
      });
    });
    partsRef.current = next;
  }, [vms, failedNode, ringRadii]);

  // Single RAF loop — reads dataRef so it never re-subscribes on telemetry ticks.
  useEffect(() => {
    const canvas = canvasRef.current!;
    const wrap = wrapRef.current!;
    const ctx = canvas.getContext('2d')!;
    const VW = 680, VH = 460, CX = 340, CY = 215;
    let raf = 0, t = 0;
    let dpr = Math.min(window.devicePixelRatio || 1, 2);

    const resize = () => {
      const w = wrap.clientWidth;
      const h = (w / VW) * VH;
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      canvas.style.width = w + 'px';
      canvas.style.height = h + 'px';
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    const draw = () => {
      t += 0.016;
      const { failedNode: fn } = dataRef.current;
      const parts = partsRef.current;
      const rings = ringsRef.current;
      const scale = (canvas.width / dpr) / VW;

      ctx.setTransform(dpr * scale, 0, 0, dpr * scale, 0, 0);
      ctx.clearRect(0, 0, VW, VH);

      const health = fleetHealth(dataRef.current.vms);
      const coreR = 46 * (1 + Math.sin(t * 2.2) * 0.04);

      // Orbital rings
      rings.forEach(ring => {
        ctx.beginPath();
        for (let a = 0; a <= 360; a += 6) {
          const rad = (a * Math.PI) / 180;
          const x = CX + Math.cos(rad) * ring.r;
          const y = CY + Math.sin(rad) * ring.r * ring.tilt * 1.6;
          a === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
        }
        ctx.strokeStyle = ring.dead ? 'rgba(226,75,74,.4)' : 'rgba(245,158,11,.22)';
        ctx.lineWidth = 1;
        ctx.setLineDash(ring.dead ? [4, 4] : []);
        ctx.stroke();
        ctx.setLineDash([]);
      });

      // Core halo
      const halo = ctx.createRadialGradient(CX, CY, 0, CX, CY, 150);
      halo.addColorStop(0, 'rgba(245,158,11,0.30)');
      halo.addColorStop(1, 'rgba(245,158,11,0)');
      ctx.fillStyle = halo;
      ctx.globalAlpha = 0.5 + Math.sin(t * 2.2) * 0.12;
      ctx.beginPath();
      ctx.arc(CX, CY, 150, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;

      // Particles — depth-sorted (painter's algorithm)
      const order: { p: Particle; x: number; y: number; depth: number; col: string; dead: boolean }[] = [];
      parts.forEach(p => {
        const ring = rings[p.ring];
        if (!ring) return;
        const dead = ring.dead || p.vm.signal === 'off';
        const load = Math.max(p.vm.cpuPct, p.vm.memPct) / 100;
        p.ang += ring.speed * 0.01 * (dead ? 0 : 1) * (1 + load * 0.6);
        const x = CX + Math.cos(p.ang) * ring.r;
        const y = CY + Math.sin(p.ang) * ring.r * ring.tilt * 1.6;
        const depth = (Math.sin(p.ang) + 1) / 2;
        order.push({ p, x, y, depth, col: dead ? PAL.off : PAL[p.vm.signal], dead });
      });
      order.sort((a, b) => a.depth - b.depth);

      order.forEach(({ p, x, y, depth, col, dead }) => {
        const load = Math.max(p.vm.cpuPct, p.vm.memPct) / 100;

        // Tether line to core
        if (!dead) {
          ctx.beginPath();
          ctx.moveTo(CX, CY);
          ctx.lineTo(x, y);
          ctx.strokeStyle = col;
          ctx.globalAlpha = 0.08 + load * 0.12;
          ctx.lineWidth = 0.6;
          ctx.stroke();
          ctx.globalAlpha = 1;
        }

        const baseR = (4 + load * 4) * (0.6 + depth * 0.6);

        // Glow halo
        if (!dead) {
          ctx.beginPath();
          ctx.arc(x, y, baseR + 4 + Math.sin(t * 3 + p.phase) * 2, 0, Math.PI * 2);
          ctx.fillStyle = col;
          ctx.globalAlpha = 0.15;
          ctx.fill();
          ctx.globalAlpha = 1;
        }

        // Particle body
        ctx.beginPath();
        ctx.arc(x, y, baseR, 0, Math.PI * 2);
        ctx.fillStyle = col;
        ctx.globalAlpha = dead ? 0.4 : 0.95;
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.lineWidth = 1.2;
        ctx.strokeStyle = '#0a0c14';
        ctx.stroke();

        p._sx = x; p._sy = y; p._r = baseR;
      });

      // Reactor core
      const coreCol = health > 0.7 ? null : health > 0.45 ? '#c98815' : '#a32d2d';
      ctx.beginPath();
      ctx.arc(CX, CY, coreR, 0, Math.PI * 2);
      if (coreCol) {
        ctx.fillStyle = coreCol;
      } else {
        const g = ctx.createRadialGradient(CX, CY - 10, 4, CX, CY, coreR);
        g.addColorStop(0, '#fcd9a0');
        g.addColorStop(0.4, PAL.plasma);
        g.addColorStop(1, PAL.deep);
        ctx.fillStyle = g;
      }
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = '#fcd9a0';
      ctx.stroke();

      // Internal flares
      for (let i = 0; i < 8; i++) {
        const sa = t * 1.3 + i * 0.785;
        ctx.beginPath();
        ctx.moveTo(CX + Math.cos(sa) * coreR * 0.4, CY + Math.sin(sa) * coreR * 0.4);
        ctx.lineTo(CX + Math.cos(sa) * coreR * 0.85, CY + Math.sin(sa) * coreR * 0.85);
        ctx.strokeStyle = '#fff';
        ctx.globalAlpha = 0.1 + Math.sin(t * 3 + i) * 0.08;
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.globalAlpha = 1;
      }

      // Core readout
      ctx.fillStyle = '#fff';
      ctx.font = "700 28px 'Space Grotesk', sans-serif";
      ctx.textAlign = 'center';
      ctx.fillText(Math.round(health * 100) + '%', CX, CY + 4);
      ctx.fillStyle = '#e0ddf8';
      ctx.font = "9px 'IBM Plex Mono', monospace";
      ctx.fillText('CORE', CX, CY + 18);

      // Node labels
      ctx.font = "9px 'IBM Plex Mono', monospace";
      ctx.textAlign = 'start';
      rings.forEach(ring => {
        ctx.fillStyle = ring.dead ? PAL.crit : PAL.ink;
        ctx.fillText(ring.node + (ring.dead ? ' ✕' : ''), CX + ring.r - 6, CY + 4);
      });

      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);

    // Hit-test for click/hover
    const hit = (mx: number, my: number): Particle | null => {
      const s = (canvas.width / dpr) / VW;
      const vx = mx / s, vy = my / s;
      for (const p of partsRef.current) {
        const dx = vx - (p._sx ?? -9999);
        const dy = vy - (p._sy ?? -9999);
        if (Math.hypot(dx, dy) < (p._r ?? 0) + 4) return p;
      }
      return null;
    };
    const onMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      hoverRef.current = hit(e.clientX - rect.left, e.clientY - rect.top);
      canvas.style.cursor = hoverRef.current ? 'pointer' : 'crosshair';
    };
    const onClick = () => {
      if (hoverRef.current && onInspect) onInspect(hoverRef.current.vm);
    };
    canvas.addEventListener('mousemove', onMove);
    canvas.addEventListener('click', onClick);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      canvas.removeEventListener('mousemove', onMove);
      canvas.removeEventListener('click', onClick);
    };
  }, [onInspect]);

  return (
    <div
      ref={wrapRef}
      className={className}
      style={{
        position: 'relative',
        background: 'radial-gradient(120% 100% at 50% 30%, rgba(245,158,11,0.06), #0a0c14)',
        border: '1px solid var(--hairline, #27304a)',
        borderRadius: 18,
        overflow: 'hidden',
      }}
    >
      <canvas ref={canvasRef} style={{ display: 'block', width: '100%' }} />
    </div>
  );
}
