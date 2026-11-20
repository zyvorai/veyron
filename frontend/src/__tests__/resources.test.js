import { describe, expect, it } from 'vitest';
import { emptyData, pageGroup, pageLabel } from '../resources.js';
import { ALL_KEYS, CORE_KEYS } from '../useResources.js';

describe('resource registry', () => {
  it('has a loader for every core resource and an empty slot for every loader', () => {
    const data = emptyData();
    for (const key of CORE_KEYS) expect(ALL_KEYS).toContain(key);
    for (const key of ALL_KEYS) expect(data[key]?.rows).toEqual([]);
  });

  it('labels and groups pages from the nav', () => {
    expect(pageLabel('vms')).toBeTruthy();
    expect(pageGroup('vms')).toBe('Compute');
    expect(pageGroup('nope')).toBe('');
  });
});
