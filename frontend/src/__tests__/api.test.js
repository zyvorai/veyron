import { describe, expect, it } from 'vitest';
import { asArray, loginErrorMessage, mapNode, mapVm, parseCount, parseMemGi, relTime, tidyEventMessage, unwrap } from '../api.js';

describe('unwrap / asArray', () => {
  it('unwraps the success envelope', () => {
    expect(unwrap({ success: true, data: [1] })).toEqual([1]);
    expect(unwrap({ success: false, data: [1] })).toEqual({ success: false, data: [1] });
  });

  it('finds the list under any known key', () => {
    expect(asArray([1, 2])).toEqual([1, 2]);
    expect(asArray({ items: [1] })).toEqual([1]);
    expect(asArray({ success: true, data: { vms: [{ name: 'a' }] } })).toEqual([{ name: 'a' }]);
    expect(asArray({ nothing: true })).toEqual([]);
    expect(asArray(null)).toEqual([]);
  });
});

describe('parseCount', () => {
  it('reads numbers and "2 cores" labels', () => {
    expect(parseCount(4)).toBe(4);
    expect(parseCount('2 cores')).toBe(2);
    expect(parseCount(undefined)).toBe('—');
  });
});

describe('parseMemGi', () => {
  it('normalizes units to GiB', () => {
    expect(parseMemGi('8Gi')).toBe(8);
    expect(parseMemGi('2048Mi')).toBe(2);
    expect(parseMemGi(4096)).toBe(4);
    expect(parseMemGi(16)).toBe(16);
    expect(parseMemGi(null)).toBe(0);
  });
});

describe('mapVm', () => {
  it('builds a stable namespaced id and strips phase prefixes', () => {
    const vm = mapVm({ name: 'web', namespace: 'prod', status: 'VMIPhaseRunning', cpu: '2 cores', memory: '4Gi' }, 0);
    expect(vm).toMatchObject({ id: 'prod/web', status: 'Running', cpu: 2, ram: 4, host: '—' });
  });

  it('falls back to defaults for sparse payloads', () => {
    expect(mapVm({}, 3)).toMatchObject({ id: 'default/vm-3', status: 'Unknown' });
  });
});

describe('audit mapping fixes', () => {
  it('does not invent a disk size from a PVC reference or N/A', () => {
    const vm = mapVm({ name: 'a', disk: 'pvc:e2e-root-2', host: 'N/A', ip: 'none' }, 0);
    expect(vm).toMatchObject({ disk: '—', diskSrc: 'pvc:e2e-root-2', host: '—', ip: '—' });
    expect(mapVm({ name: 'b', disk_gb: '20Gi' }, 0).disk).toBe(20);
  });

  it('reports host usage as null when metrics-server has no data', () => {
    expect(mapNode({ name: 'n1' }, 0).cpu).toBeNull();
    expect(mapNode({ name: 'n1', cpu_percent: 98.7 }, 0).cpu).toBe(99);
  });

  it('rewrites name_ns(uid) event subjects to ns/name', () => {
    expect(tidyEventMessage('Back-off restarting web_prod(0f8e2c1a-1b2c-4d5e-8f90-123456789abc) container')).toContain('prod/web');
  });

  it('formats relative times', () => {
    expect(relTime(new Date(Date.now() - 5 * 60_000).toISOString())).toBe('5m ago');
  });
});

describe('loginErrorMessage', () => {
  it('maps auth failures and unreachable servers to Netra-style copy', () => {
    expect(loginErrorMessage(Object.assign(new Error('Invalid credentials'), { status: 401 }))).toBe('Wrong username or password.');
    expect(loginErrorMessage(Object.assign(new TypeError('Failed to fetch'), { network: true }))).toMatch(/Could not reach Veyron/);
    expect(loginErrorMessage(Object.assign(new Error('Bad gateway'), { status: 502 }))).toMatch(/Could not reach Veyron/);
  });
});
