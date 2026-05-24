import { useState } from 'react';
import {
  restartVirtualMachine,
  startVirtualMachine,
  stopVirtualMachine,
} from '../../lib/api';

type VmLifecycleBarProps = {
  namespace: string;
  vmName: string;
  status: string;
  onChanged?: () => void;
};

export function VmLifecycleBar({ namespace, vmName, status, onChanged }: VmLifecycleBarProps) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const normalized = status.toLowerCase();
  const running = normalized === 'running';
  const stopped = normalized === 'stopped' || normalized === 'paused';

  const run = async (action: 'start' | 'stop' | 'restart', label: string) => {
    setBusy(label);
    setError(null);
    try {
      if (action === 'start') await startVirtualMachine(namespace, vmName);
      else if (action === 'stop') await stopVirtualMachine(namespace, vmName);
      else await restartVirtualMachine(namespace, vmName);
      onChanged?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'VM action failed');
    } finally {
      setBusy(null);
    }
  };

  const btn = (label: string, action: 'start' | 'stop' | 'restart', disabled: boolean) => (
    <button
      type="button"
      disabled={disabled || busy !== null}
      onClick={() => void run(action, label)}
      style={{
        padding: '8px 14px',
        borderRadius: '6px',
        border: '1px solid #ddd',
        background: disabled ? '#f3f4f6' : '#fff',
        color: disabled ? '#9ca3af' : '#222324',
        fontWeight: 600,
        fontSize: '13px',
        cursor: disabled || busy ? 'not-allowed' : 'pointer',
      }}
    >
      {busy === label ? `${label}…` : label}
    </button>
  );

  return (
    <div style={{ marginTop: '16px', marginBottom: '8px' }}>
      <div style={{ fontSize: '13px', fontWeight: 600, color: '#374151', marginBottom: '8px' }}>
        Lifecycle
      </div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
        {btn('Start', 'start', running)}
        {btn('Stop', 'stop', stopped)}
        {btn('Restart', 'restart', !running)}
      </div>
      {error ? (
        <p style={{ marginTop: '8px', fontSize: '13px', color: '#dc2626' }} role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
