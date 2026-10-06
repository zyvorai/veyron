import { describe, it, expect } from 'vitest';
import { applyFilter, looksLikeQuestion, matches } from '../aisearch.js';

const vms = [
  { id: 'a/web-1', name: 'web-1', ns: 'a', status: 'Running', cpu: 2, ram: 4, os: 'ubuntu-24.04' },
  { id: 'a/db-1', name: 'db-1', ns: 'a', status: 'Stopped', cpu: 8, ram: 32, os: 'windows-server-2022' },
  { id: 'b/ci', name: 'ci', ns: 'b', status: 'Running', cpu: '—', ram: 16, os: 'debian-12' },
];

describe('aisearch', () => {
  it('matches strings case-insensitively and numbers numerically', () => {
    expect(matches(vms[0], { field: 'status', op: 'eq', value: 'running' })).toBe(true);
    expect(matches(vms[1], { field: 'os', op: 'contains', value: 'Windows' })).toBe(true);
    expect(matches(vms[1], { field: 'ram', op: 'gt', value: 8 })).toBe(true);
    expect(matches(vms[2], { field: 'cpu', op: 'gt', value: 1 })).toBe(false);
  });

  it('filters, sorts and limits', () => {
    const f = { resource: 'vms', where: [{ field: 'status', op: 'eq', value: 'Running' }], sort: { field: 'ram', dir: 'desc' }, limit: 1 };
    expect(applyFilter(vms, f).map((r) => r.name)).toEqual(['ci']);
    expect(applyFilter(vms, { where: [] })).toHaveLength(3);
  });

  it('puts unmeasured values last when sorting', () => {
    const out = applyFilter(vms, { where: [], sort: { field: 'cpu', dir: 'desc' } });
    expect(out.map((r) => r.name)).toEqual(['db-1', 'web-1', 'ci']);
  });

  it('tells questions from names', () => {
    expect(looksLikeQuestion('web-1')).toBe(false);
    expect(looksLikeQuestion('stopped windows vms')).toBe(true);
    expect(looksLikeQuestion('which vms?')).toBe(true);
    expect(looksLikeQuestion('? db')).toBe(true);
  });
});
