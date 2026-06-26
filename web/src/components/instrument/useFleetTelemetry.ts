/* ============================================================
   web/src/components/instrument/useFleetTelemetry.ts
   Live fleet feed. Prefers SSE/WS LIVE stream, falls back to
   polling. Both ReactorCore and Constellation read its output.
   ============================================================ */
import { useEffect, useRef, useState } from 'react';
import { FleetVM, VmApiRow, adaptRow } from './types';

interface Options {
  /** SSE endpoint emitting VM rows (same feed as manifest ticker). */
  streamUrl?: string;
  /** Polling fallback endpoint. */
  pollUrl?: string;
  pollMs?: number;
}

export function useFleetTelemetry({
  streamUrl = '/api/v1/vms/stream',
  pollUrl = '/api/v1/vms?namespace=all',
  pollMs = 4000,
}: Options = {}) {
  const [vms, setVms] = useState<FleetVM[]>([]);
  const [connected, setConnected] = useState(false);
  const byId = useRef(new Map<string, FleetVM>());

  const merge = (rows: VmApiRow[]) => {
    for (const row of rows) {
      const vm = adaptRow(row);
      byId.current.set(vm.id, vm);
    }
    setVms([...byId.current.values()]);
  };

  useEffect(() => {
    let es: EventSource | null = null;
    let timer: ReturnType<typeof setInterval> | null = null;

    const startPolling = () => {
      const tick = async () => {
        try {
          const res = await fetch(pollUrl, {
            headers: { 'X-API-Key': (window as any).__VEYRON_API_KEY || '' },
          });
          const data = await res.json();
          const rows: VmApiRow[] = Array.isArray(data)
            ? data
            : (data.vms || data.data || data.items || []);
          byId.current.clear();
          merge(rows);
          setConnected(true);
        } catch {
          setConnected(false);
        }
      };
      tick();
      timer = setInterval(tick, pollMs);
    };

    if (typeof EventSource !== 'undefined' && streamUrl) {
      es = new EventSource(streamUrl);
      es.onopen = () => setConnected(true);
      es.onmessage = (e) => {
        try {
          const payload = JSON.parse(e.data);
          merge(Array.isArray(payload) ? payload : [payload]);
        } catch {
          /* ignore malformed frame */
        }
      };
      es.onerror = () => {
        es?.close();
        es = null;
        setConnected(false);
        startPolling();
      };
    } else {
      startPolling();
    }

    return () => {
      es?.close();
      if (timer) clearInterval(timer);
    };
  }, [streamUrl, pollUrl, pollMs]);

  return { vms, connected };
}
