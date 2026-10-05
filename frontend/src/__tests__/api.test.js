import { describe, expect, it } from 'vitest';
import { asArray, mapVm, parseCount, parseMemGi, unwrap } from '../api.js';

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
