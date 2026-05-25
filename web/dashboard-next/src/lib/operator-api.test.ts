import { describe, expect, it } from 'vitest';

/** Keep in sync with `OPERATOR_API_PREFIXES` in src/api/dashboard_paths.rs */
export const OPERATOR_API_PREFIXES = [
  '/api/v1/vms',
  '/api/v1/ws',
  '/api/v1/events',
  '/api/v1/nodes',
  '/api/v1/pods',
  '/api/v1/snapshots',
  '/api/v1/snapshot-schedules',
  '/api/v1/dashboard',
  '/api/v1/templates',
  '/api/v1/profiles',
  '/api/v1/namespaces',
  '/api/v1/storage',
  '/api/v1/activity',
  '/api/v1/alerts',
  '/api/v1/custom-resources',
  '/api/v1/gitops',
  '/api/v1/monitoring',
  '/api/v1/security',
  '/api/v1/costs',
  '/api/v1/logs',
  '/api/v1/incidents',
] as const;

describe('operator API path smoke list', () => {
  it('covers dashboard-next read routes used by the React client', () => {
    const clientPaths = [
      '/api/v1/health',
      '/api/v1/namespaces',
      '/api/v1/nodes',
      '/api/v1/storage/classes',
      '/api/v1/storage/pvcs',
      '/api/v1/vms',
      '/api/v1/templates',
      '/api/v1/alerts',
      '/api/v1/snapshots',
      '/api/v1/snapshot-schedules',
      '/api/v1/custom-resources',
      '/api/v1/gitops/status',
      '/api/v1/gitops/sync',
      '/api/v1/pods',
      '/api/v1/events/recent',
      '/api/v1/monitoring/status',
      '/api/v1/security/posture',
      '/api/v1/security/findings',
      '/api/v1/costs/summary',
      '/api/v1/costs/budgets',
      '/api/v1/logs',
      '/api/v1/incidents/timeline',
    ];

    for (const path of clientPaths) {
      if (path === '/api/v1/health') continue;
      expect(
        OPERATOR_API_PREFIXES.some((prefix) => path.startsWith(prefix)),
        path,
      ).toBe(true);
    }
  });
});
