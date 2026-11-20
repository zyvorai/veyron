// Mirrors the server's route-level minimum roles (src/api/auth_context.rs): readonly can only read,
// write can mutate VMs/snapshots/etc, and a few cluster-wide actions need admin. The server is the
// source of truth and still enforces everything (403); this only hides controls that would fail.
const ADMIN_ONLY_ACTS = new Set(['cordon', 'uncordon', 'reboot', 'restore', 'publish']);

export function normRole(role) {
  const r = String(role || 'readonly').toLowerCase();
  return r === 'admin' || r === 'write' ? r : 'readonly';
}

export function canMutate(role) {
  return normRole(role) !== 'readonly';
}

export function canAct(role, action) {
  const r = normRole(role);
  if (r === 'admin') return true;
  // Opening a console / reading logs is not a mutation as far as the UI is concerned.
  if (action === 'console' || action === 'logs') return true;
  if (r === 'readonly') return false;
  return !ADMIN_ONLY_ACTS.has(action);
}
