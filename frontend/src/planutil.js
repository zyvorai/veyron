/** Plans from `GET .../plans` that are waiting for this user to approve them as the second person. */
export function waitingForApproval(plans, username) {
  return (plans || []).filter(
    (p) => p && p.status === 'pending' && p.needs_approval && !p.approved_by && p.created_by !== username,
  );
}

/** One line for a plan card or inbox row: who drafted it and what it needs. */
export function planByline(p) {
  const count = Array.isArray(p.steps) ? p.steps.length : (p.steps ?? 0);
  return `${p.created_by || 'someone'} · ${p.min_role || 'write'} role · ${count} step${count === 1 ? '' : 's'}`;
}

/** Whether a plan still has to be approved by a second person before it can run. */
export function needsSecondApprover(plan) {
  return Boolean(plan?.needs_approval) && !plan?.approved_by;
}

const INVERSE = { start: 'stop', stop: 'start', pause: 'unpause', unpause: 'pause', cordon: 'uncordon', uncordon: 'cordon' };

/** Steps of a finished plan that succeeded and have a true inverse (mirrors the server's undo). */
export function undoableSteps(plan) {
  if (!plan || (plan.status !== 'executed' && plan.status !== 'failed')) return [];
  return (plan.steps || []).filter((s) => s.result?.ok && INVERSE[s.action]);
}

/** Whether to offer "Undo" on a plan card. */
export function canUndo(plan) {
  return undoableSteps(plan).length > 0;
}
