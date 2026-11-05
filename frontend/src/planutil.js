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
