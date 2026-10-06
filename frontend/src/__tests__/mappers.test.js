import { describe, expect, it } from 'vitest';
import {
  listOf, money, mapRecommendation, mapIncident, mapAudit, mapRole, mapBinding, mapOperator,
  mapHelmRelease, mapQuota, mapCustomResource, mapWorkload, mapOrphan, mapSchedule,
  mapCiliumPolicy, mapNetworkPolicy, mapIngress, mapUser,
} from '../api.js';

describe('listOf', () => {
  it('accepts bare arrays, keyed objects and the success envelope', () => {
    expect(listOf([1], 'x')).toEqual([1]);
    expect(listOf({ orphans: [1, 2], count: 2 }, 'orphans')).toEqual([1, 2]);
    expect(listOf({ success: true, data: [3] }, 'x')).toEqual([3]);
    expect(listOf({ other: [1] }, 'x')).toEqual([]);
    expect(listOf(null, 'x')).toEqual([]);
  });
});

describe('money', () => {
  it('formats amounts and dashes non-numbers', () => {
    expect(money(122.18)).toMatch(/122/);
    expect(money(9.46)).toMatch(/9\.46/);
    expect(money('n/a')).toBe('—');
  });
});

describe('operations mappers', () => {
  it('maps a recommendation with savings', () => {
    const r = mapRecommendation(
      { id: 'REC-1', category: 'ResourceOptimization', priority: 'High', title: 'Set limits', resource: 'default/vm', estimated_savings: 9.46, effort: 'Low' },
      0,
    );
    expect(r).toMatchObject({ id: 'REC-1', name: 'Set limits', status: 'High', resource: 'default/vm', effort: 'Low' });
    expect(r.savings).toMatch(/9\.46/);
  });

  it('marks resolved incidents and keeps ids unique', () => {
    const a = mapIncident({ id: 'slo', severity: 'critical', resolved: false, kind: 'slo-breach', title: 'Breach' }, 0);
    const b = mapIncident({ id: 'slo', resolved: true }, 1);
    expect(a.status).toBe('critical');
    expect(a.kind).toBe('Slo Breach');
    expect(b.status).toBe('Resolved');
    expect(a.id).not.toBe(b.id);
  });

  it('maps audit entries', () => {
    const r = mapAudit({ id: 'x', user: 'default-scheduler', action: 'Scheduled', resource_type: 'Pod', resource_name: 'p1', outcome: 'Normal' }, 0);
    expect(r).toMatchObject({ name: 'p1', status: 'Normal', user: 'default-scheduler', type: 'Pod' });
  });
});

describe('security mappers', () => {
  it('summarises role rules and wildcard verbs', () => {
    const r = mapRole({ name: 'admin', namespace: null, rules: [{ api_groups: [''], resources: ['pods'], verbs: ['*'] }] }, 0);
    expect(r).toMatchObject({ ns: 'cluster-wide', rules: 1, verbs: 'all (*)', groups: 'core' });
  });

  it('lists binding subjects', () => {
    const b = mapBinding({ name: 'b', role_name: 'r', subjects: [{ kind: 'ServiceAccount', name: 'sa', namespace: 'ns' }] }, 0);
    expect(b.subjects).toBe('ServiceAccount:ns/sa');
  });

  it('maps a console user', () => {
    expect(mapUser({ username: 'admin', role: 'admin', display_name: 'Administrator' })).toMatchObject({
      id: 'admin',
      role: 'admin',
      display: 'Administrator',
    });
  });
});

describe('platform mappers', () => {
  it('counts operator-managed resources and dashes empty versions', () => {
    const o = mapOperator({ name: 'virt-controller', namespace: 'kubevirt', version: '', status: 'Running', managed_resources: ['A', 'B'] }, 0);
    expect(o).toMatchObject({ managed: 2, version: '—', status: 'Running' });
  });

  it('joins helm chart and version, and dashes empty ones', () => {
    expect(mapHelmRelease({ name: 'x', chart: 'cilium', chart_version: '1.19.5' }, 0).chart).toBe('cilium-1.19.5');
    expect(mapHelmRelease({ name: 'x', chart: '', chart_version: '' }, 0).chart).toBe('—');
  });

  it('shows quota usage only when a limit is set', () => {
    const q = mapQuota({ name: 'q', namespace: 'n', cpu_limit: '', cpu_used: '', vm_limit: 3, vm_count: 0 }, 0);
    expect(q).toMatchObject({ cpu: '—', vms: '0 / 3' });
  });

  it('maps CRDs and workloads', () => {
    expect(mapCustomResource({ name: 'addons.k3s.cattle.io', kind: 'Addon', instance_count: 0 }, 0)).toMatchObject({ name: 'Addon', count: 0 });
    expect(mapWorkload({ name: 'w', workload_type: 'Deployment', replicas: 2, ready_replicas: 1, status: 'Ready' }, 0).ready).toBe('1/2');
  });
});

describe('storage and network mappers', () => {
  it('maps orphans and schedules', () => {
    expect(mapOrphan({ namespace: 'hermes-system', name: 'hermes-data', storage_class: 'local-path', capacity: '1Gi' }, 0)).toMatchObject({
      id: 'hermes-system/hermes-data',
      size: '1Gi',
    });
    const s = mapSchedule({ name: 'c', namespace: 'default', vm_name: 'vm', cron: '0 2 * * *', enabled: true, max_snapshots: 0 }, 0);
    expect(s).toMatchObject({ status: 'Enabled', keep: 'all', lastRun: '—' });
  });

  it('renders selectors and ingress backends', () => {
    expect(mapCiliumPolicy({ name: 'p', endpoint_selector: { app: 'aether' } }, 0).selector).toBe('app=aether');
    expect(mapNetworkPolicy({ name: 'p', pod_selector: {} }, 0).selector).toBe('all pods');
    const i = mapIngress({ name: 'i', hosts: ['a.example'], tls: true, rules: [{ path: '/api', service_name: 'api', service_port: 9191 }] }, 0);
    expect(i).toMatchObject({ hosts: 'a.example', tls: 'Yes', backends: '/api → api:9191' });
  });
});
