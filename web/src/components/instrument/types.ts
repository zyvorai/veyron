/* ============================================================
   web/src/components/instrument/types.ts
   Shared fleet model for all Instrument Deck components.
   ============================================================ */

export type Signal = 'ok' | 'warn' | 'crit' | 'off';

export interface FleetVM {
  id: string;        // namespace/name
  name: string;
  namespace: string;
  node: string;
  status: string;    // KubeVirt status string
  signal: Signal;
  cpuPct: number;    // 0-100
  memPct: number;    // 0-100
  netMbps: number;
  os?: string;
}

/** Shape returned by /api/v1/vms?namespace=all */
export interface VmApiRow {
  name: string;
  namespace?: string;
  node?: string;
  status?: string;
  cpu_utilization?: number;
  memory_utilization?: number;
  network_utilization?: number;
  os?: string;
}

export function deriveSignal(row: VmApiRow): Signal {
  const s = (row.status || '').toLowerCase();
  if (s === 'failed' || s === 'error' || s.includes('error')) return 'crit';
  if (s === 'stopped' || s === 'paused') return 'off';
  const cpu = row.cpu_utilization ?? 0;
  const mem = row.memory_utilization ?? 0;
  if (cpu > 90 || mem > 90) return 'crit';
  if (cpu > 75 || mem > 75) return 'warn';
  return 'ok';
}

export function adaptRow(row: VmApiRow): FleetVM {
  const ns = row.namespace || 'default';
  return {
    id: `${ns}/${row.name}`,
    name: row.name,
    namespace: ns,
    node: row.node || 'unknown',
    status: row.status || 'Unknown',
    signal: deriveSignal(row),
    cpuPct: row.cpu_utilization ?? 0,
    memPct: row.memory_utilization ?? 0,
    netMbps: (row.network_utilization ?? 0) * 10,
    os: row.os,
  };
}

export function fleetHealth(vms: FleetVM[]): number {
  if (!vms.length) return 1;
  const running = vms.filter(v => v.signal === 'ok' || v.signal === 'warn').length;
  return running / vms.length;
}

export interface NodeGroup {
  node: string;
  vms: FleetVM[];
}

export function groupByNode(vms: FleetVM[]): NodeGroup[] {
  const map = new Map<string, FleetVM[]>();
  for (const vm of vms) {
    const arr = map.get(vm.node) ?? [];
    arr.push(vm);
    map.set(vm.node, arr);
  }
  return [...map.entries()].map(([node, vms]) => ({ node, vms }));
}
