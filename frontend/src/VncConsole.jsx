import { useEffect, useRef, useState } from 'react';
import RFB from '@novnc/novnc/lib/rfb.js';
import { api, wsUrl } from './api.js';

/** Live noVNC viewer for a Kairon machine (ticket → /vnc WebSocket). */
export function VncConsole({ vm }) {
  const hostRef = useRef(null);
  const rfbRef = useRef(null);
  const [status, setStatus] = useState('Connecting…');
  const [err, setErr] = useState('');

  useEffect(() => {
    let cancelled = false;
    let rfb = null;

    (async () => {
      try {
        setErr('');
        setStatus('Requesting console ticket…');
        const { ticket } = await api.wsTicket();
        if (cancelled || !hostRef.current) return;
        const ns = encodeURIComponent(vm.ns || 'default');
        const name = encodeURIComponent(vm.name);
        const url = wsUrl(`/api/v1/vms/${ns}/${name}/vnc?ticket=${encodeURIComponent(ticket)}`);
        setStatus('Opening VNC…');
        rfb = new RFB(hostRef.current, url);
        rfb.scaleViewport = true;
        rfb.resizeSession = true;
        rfb.background = '#0b0b0d';
        rfb.addEventListener('connect', () => {
          if (!cancelled) setStatus('');
        });
        rfb.addEventListener('disconnect', (e) => {
          if (cancelled) return;
          const clean = e?.detail?.clean;
          setStatus(clean ? 'Disconnected' : 'Connection lost');
          if (!clean) setErr('VNC session ended unexpectedly. Is the VM running?');
        });
        rfb.addEventListener('credentialsrequired', () => {
          try {
            rfb.sendCredentials({ password: '' });
          } catch {
            /* ignore */
          }
        });
        rfbRef.current = rfb;
      } catch (e) {
        if (!cancelled) {
          setStatus('');
          setErr(e.message || String(e));
        }
      }
    })();

    return () => {
      cancelled = true;
      try {
        rfb?.disconnect();
      } catch {
        /* ignore */
      }
      rfbRef.current = null;
    };
  }, [vm.ns, vm.name]);

  return (
    <div className="console">
      <div className="console-canvas" ref={hostRef} />
      {(status || err) && (
        <div className="console-status">
          {err ? (
            <>
              <div style={{ color: '#ff7b72', marginBottom: 8 }}>{err}</div>
              <div>
                Guest: {vm.os || '—'} · IP: {vm.ip || '—'} · Status: {vm.status || '—'}
              </div>
            </>
          ) : (
            status
          )}
        </div>
      )}
    </div>
  );
}
