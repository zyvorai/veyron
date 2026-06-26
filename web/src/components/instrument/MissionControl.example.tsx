/* ============================================================
   web/src/pages/MissionControl.tsx  (example wiring)
   Wires the live fleet feed to the Reactor as the page hero.

   To integrate:
     1. Point streamUrl / pollUrl at your actual API endpoints
     2. Update VmApiRow in types.ts to match your response shape
     3. Replace window.location.hash with your router / openCopilotWithVm()
   ============================================================ */
import { ReactorCore } from '../components/instrument/ReactorCore';
import { useFleetTelemetry } from '../components/instrument/useFleetTelemetry';
import { FleetVM } from '../components/instrument/types';

export function MissionControl() {
  const { vms, connected } = useFleetTelemetry({
    streamUrl: '/api/v1/vms/stream',
    pollUrl:   '/api/v1/vms?namespace=all',
    pollMs:    4000,
  });

  const openConsole = (vm: FleetVM) => {
    // In the SPA dashboard this wires to openCopilotWithVm()
    if (typeof (window as any).openCopilotWithVm === 'function') {
      (window as any).openCopilotWithVm(
        'Show health detail for VM ' + vm.name,
        vm.namespace,
        vm.name,
      );
    }
  };

  return (
    <div style={{ display: 'grid', gap: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <h1 style={{ fontFamily: "'Space Grotesk'", fontSize: 19, fontWeight: 600 }}>
          Mission Control
        </h1>
        <span className={`id-sig ${connected ? 'id-sig-ok' : 'id-sig-crit'}`}>
          <span className="id-pulse" />
          {connected ? 'live' : 'reconnecting'}
        </span>
      </div>

      <ReactorCore vms={vms} onInspect={openConsole} />

      {/* KPI tiles, alert rows, Ask Zeus panel go below */}
    </div>
  );
}
