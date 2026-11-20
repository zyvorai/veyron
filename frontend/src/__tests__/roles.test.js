import { describe, it, expect } from 'vitest';
import { normRole, canMutate, canAct } from '../roles.js';

describe('roles', () => {
  it('fails closed to readonly for unknown or missing roles', () => {
    expect(normRole(undefined)).toBe('readonly');
    expect(normRole('root')).toBe('readonly');
    expect(normRole('ADMIN')).toBe('admin');
  });

  it('readonly cannot mutate but can open consoles and logs', () => {
    expect(canMutate('readonly')).toBe(false);
    expect(canAct('readonly', 'stop')).toBe(false);
    expect(canAct('readonly', 'console')).toBe(true);
    expect(canAct('readonly', 'logs')).toBe(true);
  });

  it('write can mutate but not admin-only actions', () => {
    expect(canMutate('write')).toBe(true);
    expect(canAct('write', 'stop')).toBe(true);
    expect(canAct('write', 'cordon')).toBe(false);
    expect(canAct('write', 'restore')).toBe(false);
  });

  it('admin can do everything', () => {
    expect(canAct('admin', 'cordon')).toBe(true);
    expect(canAct('admin', 'restore')).toBe(true);
  });
});
