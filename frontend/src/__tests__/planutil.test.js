import { describe, it, expect } from 'vitest';
import { needsSecondApprover, planByline, waitingForApproval } from '../planutil.js';

const row = (o) => ({ id: 'p-1', status: 'pending', needs_approval: true, approved_by: null, created_by: 'alice', ...o });

describe('waitingForApproval', () => {
  it('lists only pending plans that need a second person and are not the user\'s own', () => {
    const plans = [
      row({ id: 'a' }),
      row({ id: 'b', created_by: 'bob' }),
      row({ id: 'c', approved_by: 'carol' }),
      row({ id: 'd', needs_approval: false }),
      row({ id: 'e', status: 'executed' }),
      null,
    ];
    expect(waitingForApproval(plans, 'bob').map((p) => p.id)).toEqual(['a']);
    expect(waitingForApproval(plans, 'dave').map((p) => p.id)).toEqual(['a', 'b']);
  });
  it('tolerates missing input', () => {
    expect(waitingForApproval(undefined, 'x')).toEqual([]);
  });
});

describe('needsSecondApprover', () => {
  it('is true only until someone approves', () => {
    expect(needsSecondApprover({ needs_approval: true })).toBe(true);
    expect(needsSecondApprover({ needs_approval: true, approved_by: 'bob' })).toBe(false);
    expect(needsSecondApprover({ needs_approval: false })).toBe(false);
    expect(needsSecondApprover(undefined)).toBe(false);
  });
});

describe('planByline', () => {
  it('reads from a full plan or a listing row', () => {
    expect(planByline({ created_by: 'alice', min_role: 'admin', steps: [1, 2, 3] })).toBe('alice · admin role · 3 steps');
    expect(planByline({ created_by: 'alice', min_role: 'write', steps: 1 })).toBe('alice · write role · 1 step');
    expect(planByline({})).toBe('someone · write role · 0 steps');
  });
});
