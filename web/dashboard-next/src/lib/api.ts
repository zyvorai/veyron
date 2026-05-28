// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** VMRogue REST client (browser → same-origin /api/v1). */

import { getApiKey, getBearerToken } from './auth';

const API = '/api/v1';

export type VmrogueFeatureContext = {
  data_source: string;
  scope: string;
  limitations: string;
};

type ApiBody = Record<string, unknown> | unknown[] | null;

function authHeaders(): HeadersInit {
  const apiKey = getApiKey();
  if (apiKey) {
    return { 'X-API-Key': apiKey };
  }
  const token = getBearerToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/** Normalize unknown thrown values to a user-facing message. */
export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return 'Request failed';
}

/** Extract an error message from a parsed API body (HTTP error or success:false). */
export function extractApiError(
  body: ApiBody,
  statusText: string,
  fallbackText?: string,
): string {
  if (body && typeof body === 'object' && !Array.isArray(body)) {
    const obj = body as Record<string, unknown>;
    const errField = obj.error;
    if (errField && typeof errField === 'object' && errField !== null) {
      const msg = (errField as { message?: string }).message;
      if (msg) return String(msg);
    }
    if (typeof errField === 'string' && errField) return errField;
    if (typeof obj.message === 'string' && obj.message) return obj.message;
  }
  if (fallbackText?.trim()) return fallbackText.trim().slice(0, 200);
  return statusText || 'Request failed';
}

/** Unwrap ApiResponse { success, data }; leave bare payloads as-is. */
export function unwrapApiData<T>(body: ApiBody): T {
  if (body && typeof body === 'object' && !Array.isArray(body)) {
    const obj = body as Record<string, unknown>;
    if (obj.success === true && Object.prototype.hasOwnProperty.call(obj, 'data')) {
      return obj.data as T;
    }
  }
  return body as T;
}

/** Parse fetch response body and throw on failure. Exported for unit tests. */
export async function parseApiResponse<T>(res: Response): Promise<T> {
  const text = await res.text();
  let body: ApiBody = null;
  if (text) {
    try {
      body = JSON.parse(text) as ApiBody;
    } catch {
      if (!res.ok) {
        throw new Error(text.trim().slice(0, 200) || `HTTP ${res.status}`);
      }
      throw new Error('Invalid JSON from API');
    }
  }

  if (!res.ok) {
    throw new Error(extractApiError(body, res.statusText, text));
  }

  if (body && typeof body === 'object' && !Array.isArray(body)) {
    const obj = body as Record<string, unknown>;
    if (obj.success === false) {
      throw new Error(extractApiError(body, res.statusText));
    }
  }

  return unwrapApiData<T>(body);
}

async function apiJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
      ...(init?.headers ?? {}),
    },
  });
  return parseApiResponse<T>(res);
}

function nsQuery(namespace?: string, defaultAll = false): string {
  if (namespace && namespace !== 'all') {
    return `?namespace=${encodeURIComponent(namespace)}`;
  }
  return defaultAll ? '?namespace=all' : '';
}

export interface VmRecord {
  namespace: string;
  name: string;
  status: string;
  ip?: string;
  node?: string;
  cpu?: string;
  memory?: string;
}

export interface AlertRecord {
  id?: string;
  name: string;
  source: string;
  status: string;
  message: string;
  severity?: string;
}

export async function fetchVmInventory(namespace: string): Promise<VmRecord[]> {
  const q = namespace && namespace !== 'all' ? `?namespace=${encodeURIComponent(namespace)}` : '';
  return apiJson<VmRecord[]>(`/vms${q}`);
}

export async function fetchAlerts(namespace?: string): Promise<AlertRecord[]> {
  const q = nsQuery(namespace);
  return apiJson<AlertRecord[]>(`/alerts${q}`);
}

/** Alerts fetch that returns empty list on failure (metrics poll fallback). */
export async function fetchAlertsSafe(namespace?: string): Promise<AlertRecord[]> {
  try {
    return await fetchAlerts(namespace);
  } catch {
    return [];
  }
}

export async function fetchHealthSummary(): Promise<{
  status: string;
  message: string;
  uptime_seconds?: number;
}> {
  type HealthPayload = { status: string; uptime_seconds?: number; service?: string };
  const data = await apiJson<HealthPayload>(`/health`).catch(
    (): HealthPayload => ({ status: 'unknown' }),
  );
  const message =
    data.uptime_seconds != null
      ? `uptime ${data.uptime_seconds}s`
      : data.service ?? data.status;
  return { status: data.status, message, uptime_seconds: data.uptime_seconds };
}

export interface NamespaceRecord {
  name: string;
  status: string;
  vm_count: number;
}

export async function fetchNamespaces(): Promise<
  Array<{ name: string; vmCount: number; status: string }>
> {
  const rows = await apiJson<NamespaceRecord[]>('/namespaces');
  return rows.map((ns) => ({
    name: ns.name,
    vmCount: ns.vm_count,
    status: ns.status,
  }));
}

export type ClusterNodeRecord = {
  name: string;
  status: string;
  roles: string[];
  cpu_capacity: string;
  memory_capacity: string;
  cpu_allocatable: string;
  memory_allocatable: string;
  kubelet_version: string;
  os_image: string;
  kernel_version: string;
  age: string;
};

export async function fetchClusterNodes(): Promise<ClusterNodeRecord[]> {
  return apiJson<ClusterNodeRecord[]>('/nodes');
}

export type PvcRecord = {
  name: string;
  namespace: string;
  status: string;
  capacity: string;
  storage_class: string;
  access_modes: string[];
};

export type StorageClassRecord = {
  name: string;
  provisioner: string;
  reclaim_policy: string;
  volume_binding_mode: string;
  is_default: boolean;
};

export async function fetchPvcs(namespace = 'all'): Promise<PvcRecord[]> {
  const q = `?namespace=${encodeURIComponent(namespace)}`;
  return apiJson<PvcRecord[]>(`/storage/pvcs${q}`);
}

export async function fetchStorageClasses(): Promise<StorageClassRecord[]> {
  return apiJson<StorageClassRecord[]>('/storage/classes');
}

export type CustomResourceRecord = {
  name: string;
  group: string;
  version: string;
  kind: string;
  namespace?: string | null;
  scope: string;
  instance_count: number;
};

export type GitOpsStatusRecord = {
  vmrogue_context?: VmrogueFeatureContext;
  repo_url: string;
  branch: string;
  last_commit: string;
  sync_status: string;
  last_synced?: string | null;
  drift_detected: boolean;
  note: string;
  argo_applications?: string[];
  flux_kustomizations?: string[];
};

export async function fetchCustomResources(): Promise<CustomResourceRecord[]> {
  return apiJson<CustomResourceRecord[]>('/custom-resources');
}

export async function fetchGitOpsStatus(namespace = 'all'): Promise<GitOpsStatusRecord> {
  const q = nsQuery(namespace);
  return apiJson<GitOpsStatusRecord>(`/gitops/status${q}`);
}

export type GitOpsSyncRequest = {
  force?: boolean;
  dry_run?: boolean;
  argo_app?: string;
  flux_kustomization?: string;
};

export type GitOpsSyncResult = {
  status?: string;
  message?: string;
  sync_status?: string;
  argo_sync_triggered?: boolean;
  flux_reconcile_triggered?: boolean;
  vmrogue_context?: VmrogueFeatureContext;
};

export async function triggerGitOpsSync(
  body: GitOpsSyncRequest = {},
  namespace = 'all',
): Promise<GitOpsSyncResult> {
  const q = nsQuery(namespace, true);
  return apiJson<GitOpsSyncResult>(`/gitops/sync${q}`, {
    method: 'POST',
    body: JSON.stringify({ force: false, dry_run: false, ...body }),
  });
}

export type PodRecord = {
  name: string;
  namespace: string;
  phase: string;
  node_name: string;
  ip: string;
  containers: string[];
  restarts: number;
  age: string;
};

export async function fetchPods(namespace = 'all'): Promise<PodRecord[]> {
  const q = `?namespace=${encodeURIComponent(namespace)}`;
  return apiJson<PodRecord[]>(`/pods${q}`);
}

export type ClusterEventRecord = {
  id: string;
  event_type: string;
  reason: string;
  message: string;
  namespace: string;
  involved_object: string;
  timestamp: string;
  count: number;
};

export async function fetchRecentEvents(namespace = 'all'): Promise<ClusterEventRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<ClusterEventRecord[]>(`/events/recent${q}`);
}

export type MonitoringStatusRecord = {
  prometheus_available: boolean;
  grafana_available: boolean;
  alertmanager_available: boolean;
  metrics_collection_interval: string;
  retention_period: string;
  active_alerts: number;
  total_targets: number;
  healthy_targets: number;
};

export async function fetchMonitoringStatus(namespace = 'all'): Promise<MonitoringStatusRecord> {
  const q = nsQuery(namespace);
  return apiJson<MonitoringStatusRecord>(`/monitoring/status${q}`);
}

export type SecurityPostureRecord = {
  overall_score: number;
  risk_level: string;
  total_findings: number;
  critical_findings: number;
  high_findings: number;
  medium_findings: number;
  low_findings: number;
};

export async function fetchSecurityPosture(namespace = 'all'): Promise<SecurityPostureRecord> {
  const q = nsQuery(namespace);
  return apiJson<SecurityPostureRecord>(`/security/posture${q}`);
}

export type SecurityFindingRecord = {
  id: string;
  severity: string;
  category: string;
  title: string;
  description: string;
  resource: string;
  recommendation: string;
  detected_at: string;
};

export async function fetchSecurityFindings(namespace = 'all'): Promise<SecurityFindingRecord[]> {
  const q = nsQuery(namespace);
  return apiJson<SecurityFindingRecord[]>(`/security/findings${q}`);
}

export type CostSummaryRecord = {
  vmrogue_context?: VmrogueFeatureContext;
  total_cost: number;
  currency: string;
  period: string;
  pricing_model?: string;
  disclaimer?: string;
};

export async function fetchCostSummary(namespace = 'all'): Promise<CostSummaryRecord> {
  const q = nsQuery(namespace);
  return apiJson<CostSummaryRecord>(`/costs/summary${q}`);
}

export type CostBudgetRecord = {
  name: string;
  namespace: string;
  monthly_limit: number;
  current_spend: number;
  alert_threshold_percent: number;
  status: string;
};

export type CreateBudgetRequest = {
  name: string;
  namespace: string;
  monthly_limit: number;
  alert_threshold_percent?: number;
};

export async function fetchCostBudgets(): Promise<CostBudgetRecord[]> {
  return apiJson<CostBudgetRecord[]>('/costs/budgets');
}

export async function createCostBudget(body: CreateBudgetRequest): Promise<CostBudgetRecord> {
  return apiJson<CostBudgetRecord>('/costs/budgets', {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export type LogLineRecord = {
  ts: string;
  level: string;
  source: string;
  msg: string;
};

export type LogsDashboardRecord = {
  vmrogue_context: VmrogueFeatureContext;
  error_count: number;
  warn_count: number;
  info_count: number;
  total_1h: number;
  lines: LogLineRecord[];
};

export async function fetchLogs(namespace = 'all', limit = 50): Promise<LogsDashboardRecord> {
  const params = new URLSearchParams();
  if (namespace && namespace !== 'all') params.set('namespace', namespace);
  params.set('limit', String(limit));
  const q = params.toString() ? `?${params.toString()}` : '';
  return apiJson<LogsDashboardRecord>(`/logs${q}`);
}

export type IncidentEventRecord = {
  id: string;
  timestamp: string;
  kind: string;
  severity: string;
  title: string;
  description: string;
  vm_name?: string | null;
  namespace: string;
  resolved: boolean;
};

export type IncidentsTimelineRecord = {
  vmrogue_context: VmrogueFeatureContext;
  total_incidents: number;
  open_incidents: number;
  resolved_last_24h: number;
  critical: number;
  warning: number;
  events: IncidentEventRecord[];
};

export async function fetchIncidents(namespace = 'all'): Promise<IncidentsTimelineRecord> {
  const q = nsQuery(namespace);
  return apiJson<IncidentsTimelineRecord>(`/incidents/timeline${q}`);
}

export type SnapshotScheduleRecord = {
  name: string;
  namespace: string;
  vm_name: string;
  cron: string;
  enabled: boolean;
  snapshot_prefix: string;
  max_snapshots: number;
  last_run?: string | null;
};

export type CreateSnapshotScheduleRequest = {
  namespace?: string;
  vm_name: string;
  cron: string;
  snapshot_prefix?: string;
  enabled?: boolean;
  max_snapshots?: number;
};

export async function fetchSnapshotSchedules(namespace = 'all'): Promise<SnapshotScheduleRecord[]> {
  const q = nsQuery(namespace);
  return apiJson<SnapshotScheduleRecord[]>(`/snapshot-schedules${q}`);
}

export async function createSnapshotSchedule(
  body: CreateSnapshotScheduleRequest,
): Promise<{ message?: string; name?: string; namespace?: string }> {
  return apiJson('/snapshot-schedules', {
    method: 'POST',
    body: JSON.stringify({ enabled: true, max_snapshots: 0, ...body }),
  });
}

export async function deleteSnapshotSchedule(namespace: string, cmName: string): Promise<void> {
  await apiJson(
    `/snapshot-schedules/${encodeURIComponent(namespace)}/${encodeURIComponent(cmName)}`,
    { method: 'DELETE' },
  );
}

export async function stopVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'stop');
}

export async function startVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'start');
}

export async function restartVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'restart');
}

async function vmLifecycleAction(
  namespace: string,
  name: string,
  action: 'start' | 'stop' | 'restart',
): Promise<void> {
  await apiJson(
    `/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/${action}`,
    { method: 'POST', headers: { 'Content-Type': 'application/json' } },
  );
}

export async function pauseVirtualMachine(namespace: string, name: string): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/pause`, {
    method: 'POST',
  });
}

export async function unpauseVirtualMachine(namespace: string, name: string): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/unpause`, {
    method: 'POST',
  });
}

export async function migrateVirtualMachine(namespace: string, name: string): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/migrate`, {
    method: 'POST',
  });
}

export async function cloneVirtualMachine(
  namespace: string,
  name: string,
  newName: string,
): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/clone`, {
    method: 'POST',
    body: JSON.stringify({ new_name: newName }),
  });
}

export async function resizeVirtualMachine(
  namespace: string,
  name: string,
  body: { cpus: number; memory: string },
): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

export type RdpGuestAgentResult = {
  success: boolean;
  guest_agent_connected: boolean;
  is_windows_vm: boolean;
  message: string;
  guest_exec: unknown;
  exit_code?: number | null;
  stdout?: string | null;
  stderr?: string | null;
};

export type RdpExposeStatus = {
  guest_ip?: string | null;
  is_windows_vm: boolean;
  exposed: boolean;
  node_port?: number | null;
  cluster_ip?: string | null;
  service_name: string;
  service_type?: string | null;
  rdp_via_nodeport_example?: string | null;
  vm_spec_has_rdp_port: boolean;
  suggested_node_port?: number | null;
};

export type VmDetail = VmRecord & {
  vmi_status?: {
    phase?: string;
    conditions?: Array<{ type?: string; type_?: string; status?: string }>;
  };
};

export function fetchVmDetail(namespace: string, name: string): Promise<VmDetail> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}`);
}

export function fetchRdpExpose(namespace: string, name: string): Promise<RdpExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/rdp-expose`);
}

export function putRdpExpose(
  namespace: string,
  name: string,
  body: { enabled: boolean; service_type?: string; node_port?: number },
): Promise<RdpExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/rdp-expose`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

export async function enableRdpViaGuestAgent(
  namespace: string,
  name: string,
): Promise<RdpGuestAgentResult> {
  return apiJson(
    `/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/guest-agent/enable-rdp`,
    { method: 'POST', body: '{}' },
  );
}

export async function disableRdpViaGuestAgent(
  namespace: string,
  name: string,
): Promise<RdpGuestAgentResult> {
  return apiJson(
    `/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/guest-agent/disable-rdp`,
    { method: 'POST', body: '{}' },
  );
}

export interface DataDiskDefaults {
  suggested_disk_name: string;
  suggested_pvc_name: string;
  suggested_bus: string;
  guest_os: string;
  is_windows: boolean;
  is_linux: boolean;
  root_disk_bus?: string | null;
  storage_class?: string | null;
  storage_classes: string[];
  suggested_mount_path?: string | null;
  suggested_filesystem?: string | null;
  default_size_gi: number;
}

export interface AddDataDiskResponse {
  success: boolean;
  message: string;
  pvc_name: string;
  disk_name: string;
  bus: string;
  size: string;
  guest_os: string;
  is_windows: boolean;
  is_linux: boolean;
  guest_init: {
    summary: string;
    steps: string[];
    powershell?: string;
    shell_script?: string;
    drive_letter?: string;
    mount_path?: string;
    filesystem?: string;
  };
}

export function fetchDataDiskDefaults(ns: string, name: string): Promise<DataDiskDefaults> {
  return apiJson(`/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/storage/data-disk/defaults`);
}

export function addDataDisk(
  ns: string,
  name: string,
  body: {
    disk_name?: string;
    size_gi: number;
    storage_class?: string;
    bus?: string;
    drive_letter?: string;
    mount_path?: string;
    filesystem?: string;
    wait_bound?: boolean;
  },
): Promise<AddDataDiskResponse> {
  return apiJson(`/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/storage/data-disk`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export async function deleteVirtualMachine(namespace: string, name: string): Promise<void> {
  await apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

export type VmTemplate = {
  name: string;
  description: string;
  os_type: string;
  default_cpus: number;
  default_memory: string;
  default_disk_size: string;
  tags: string[];
};

export function fetchTemplates(): Promise<VmTemplate[]> {
  return apiJson<VmTemplate[]>('/templates');
}

export type CreateVmRequest = {
  name: string;
  namespace?: string;
  template?: string;
  cpus?: number;
  memory?: string;
  disk_size?: string;
  start?: boolean;
  allow_internet?: boolean;
};

export function createVirtualMachine(body: CreateVmRequest): Promise<{ name?: string }> {
  return apiJson('/vms', {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export type VmExposePort = {
  name?: string;
  port: number;
  target_port?: number;
  protocol?: string;
  node_port?: number | null;
};

export type VmExposeStatus = {
  enabled: boolean;
  service_name?: string;
  service_type?: string;
  cluster_ip?: string;
  ports?: VmExposePort[];
};

export function fetchVmExpose(namespace: string, name: string): Promise<VmExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/expose`);
}

export function putVmExpose(
  namespace: string,
  name: string,
  body: {
    enabled: boolean;
    service_type?: string;
    ports?: VmExposePort[];
  },
): Promise<VmExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/expose`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

export type VmInternetStatus = {
  enabled: boolean;
  backend?: string | null;
  policy_name?: string | null;
};

export function fetchVmInternet(namespace: string, name: string): Promise<VmInternetStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/network/internet`);
}

export function enableVmInternet(namespace: string, name: string): Promise<VmInternetStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/network/internet`, {
    method: 'PUT',
  });
}

export function disableVmInternet(namespace: string, name: string): Promise<VmInternetStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/network/internet`, {
    method: 'DELETE',
  });
}

export type VmSnapshot = {
  name: string;
  vm_name: string;
  namespace: string;
  status: string;
  ready: boolean;
  age: string;
};

export function fetchVmSnapshots(namespace: string, vmName: string): Promise<VmSnapshot[]> {
  return apiJson(`/snapshots/${encodeURIComponent(namespace)}/${encodeURIComponent(vmName)}`);
}

export function createVmSnapshot(
  namespace: string,
  vmName: string,
  snapshotName?: string,
): Promise<{ message?: string }> {
  return apiJson(`/snapshots/${encodeURIComponent(namespace)}/${encodeURIComponent(vmName)}/create`, {
    method: 'POST',
    body: JSON.stringify(snapshotName ? { snapshot_name: snapshotName } : {}),
  });
}

export function deleteVmSnapshot(namespace: string, snapshotName: string): Promise<void> {
  return apiJson(`/snapshots/${encodeURIComponent(namespace)}/${encodeURIComponent(snapshotName)}/delete`, {
    method: 'POST',
  });
}

export function restoreVmSnapshot(namespace: string, snapshotName: string): Promise<void> {
  return apiJson(`/snapshots/${encodeURIComponent(namespace)}/${encodeURIComponent(snapshotName)}/restore`, {
    method: 'POST',
  });
}

export type TenantRecord = {
  id: string;
  display_name: string;
  owner_email: string;
  status: string;
  namespaces: string[];
  billing_tags: Record<string, string>;
  cpu_quota: string;
  memory_quota: string;
  max_vms: number;
  created_at: string;
};

export function fetchTenants(): Promise<TenantRecord[]> {
  return apiJson<TenantRecord[]>('/tenants');
}

export function createTenant(body: {
  id: string;
  display_name: string;
  owner_email: string;
  cpu_quota?: string;
  memory_quota?: string;
  max_vms?: number;
  bootstrap_namespace?: boolean;
}): Promise<TenantRecord> {
  return apiJson('/tenants', { method: 'POST', body: JSON.stringify(body) });
}

export type NadRecord = {
  name: string;
  namespace: string;
  config: string;
  scope: string;
};

export function fetchNetworkAttachmentDefinitions(namespace?: string): Promise<NadRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<NadRecord[]>(`/network/nads${q}`);
}

export type AttachMultusResponse = {
  interface_name: string;
  multus_network_name: string;
  message: string;
};

export function attachMultusToVm(
  namespace: string,
  vmName: string,
  body: { nad_namespace: string; nad_name: string; interface_name?: string },
): Promise<AttachMultusResponse> {
  return apiJson(
    `/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(vmName)}/network/multus`,
    { method: 'POST', body: JSON.stringify(body) },
  );
}

export type ImageCatalogEntry = {
  name: string;
  namespace: string;
  kind: string;
  source_type: string;
  status: string;
  capacity: string;
  storage_class: string;
  age: string;
  tags: string[];
};

export type ImageCatalogResponse = {
  vmrogue_context: VmrogueFeatureContext;
  images: ImageCatalogEntry[];
};

export function fetchImageCatalog(namespace?: string): Promise<ImageCatalogResponse> {
  const q = nsQuery(namespace, true);
  return apiJson(`/images/catalog${q}`);
}

export type VeleroBackupRecord = {
  name: string;
  namespace: string;
  phase: string;
  storage_location: string;
  completion_timestamp?: string | null;
  items_backed_up: number;
};

export type VeleroRestoreRecord = {
  name: string;
  namespace: string;
  phase: string;
  backup_name: string;
  completion_timestamp?: string | null;
};

export type VeleroStatusResponse = {
  vmrogue_context: VmrogueFeatureContext;
  velero_available: boolean;
  backups: VeleroBackupRecord[];
  restores: VeleroRestoreRecord[];
};

export function fetchVeleroStatus(): Promise<VeleroStatusResponse> {
  return apiJson('/velero/status');
}

export type TraceRow = {
  trace_id: string;
  service: string;
  operation: string;
  duration_ms: number;
  span_count: number;
  status: string;
  started_at: string;
};

export type TracesResponse = {
  vmrogue_context: VmrogueFeatureContext;
  total_traces: number;
  success_rate: number;
  p99_ms?: number | null;
  errors_1h: number;
  traces: TraceRow[];
};

export function fetchTraces(namespace?: string): Promise<TracesResponse> {
  const q = nsQuery(namespace, true);
  return apiJson(`/traces${q}`);
}

export type MetricsTimelinePoint = {
  timestamp: number;
  value: number;
};

export type MetricsTimelineResponse = {
  vmrogue_context: VmrogueFeatureContext;
  namespace: string;
  vm_name: string;
  metric: string;
  unit: string;
  points: MetricsTimelinePoint[];
};

export function fetchMetricsTimeline(params: {
  namespace?: string;
  vm?: string;
  metric?: string;
  hours?: number;
}): Promise<MetricsTimelineResponse> {
  const search = new URLSearchParams();
  if (params.namespace && params.namespace !== 'all') {
    search.set('namespace', params.namespace);
  }
  if (params.vm) search.set('vm', params.vm);
  if (params.metric) search.set('metric', params.metric);
  if (params.hours != null) search.set('hours', String(params.hours));
  const q = search.toString();
  return apiJson(`/metrics/timeline${q ? `?${q}` : ''}`);
}

export type ComplianceStatusRecord = {
  framework: string;
  compliant: boolean;
  score: number;
  total_controls: number;
  passing_controls: number;
  failing_controls: number;
  last_checked: string;
};

export type ComplianceReportRecord = {
  id: string;
  framework: string;
  generated_at: string;
  summary: ComplianceStatusRecord;
  findings: Array<{
    control_id: string;
    title: string;
    status: string;
    severity: string;
    description: string;
  }>;
};

export function fetchComplianceStatus(namespace = 'all'): Promise<ComplianceStatusRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<ComplianceStatusRecord[]>(`/compliance/status${q}`);
}

export function fetchComplianceReports(namespace = 'all'): Promise<ComplianceReportRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<ComplianceReportRecord[]>(`/compliance/reports${q}`);
}

export type NodeHeatmapEntry = {
  node_name: string;
  cpu_utilization: number;
  memory_utilization: number;
  disk_utilization: number;
  network_utilization: number;
  vm_count: number;
  heat_score: number;
};

export type ResourceHeatmapResponse = {
  vmrogue_context: VmrogueFeatureContext;
  nodes: NodeHeatmapEntry[];
  timestamp: string;
};

export function fetchResourceHeatmap(): Promise<ResourceHeatmapResponse> {
  return apiJson<ResourceHeatmapResponse>('/heatmap/resources');
}

export type CustomDashboardRecord = {
  id: string;
  name: string;
  description: string;
  panels: unknown[];
  created_by: string;
  created_at: string;
  updated_at: string;
};

export function fetchCustomDashboards(namespace = 'all'): Promise<CustomDashboardRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<CustomDashboardRecord[]>(`/dashboards${q}`);
}

export type NetworkPolicyRecord = {
  name: string;
  namespace: string;
  policy_types: string[];
  ingress_rules: number;
  egress_rules: number;
  created_at: string;
};

export function fetchNetworkPolicies(namespace = 'all'): Promise<NetworkPolicyRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<NetworkPolicyRecord[]>(`/network-policies${q}`);
}

export type DrExportPayload = {
  namespace: string;
  vm_name: string;
  virtual_machine?: unknown;
  snapshots?: unknown[];
  note?: string;
};

export function drExportManifests(namespace: string, vmName: string): Promise<DrExportPayload> {
  const q = `?namespace=${encodeURIComponent(namespace)}&vm_name=${encodeURIComponent(vmName)}`;
  return apiJson<DrExportPayload>(`/dr/export${q}`);
}

export type DrFailoverRequest = {
  namespace: string;
  vm_name: string;
  dry_run?: boolean;
  target_kubeconfig_context?: string | null;
};

export type DrFailoverResponse = {
  status: string;
  snapshot_name?: string | null;
  message: string;
  target_kubeconfig_context?: string | null;
};

export function drFailover(body: DrFailoverRequest): Promise<DrFailoverResponse> {
  return apiJson<DrFailoverResponse>('/dr/failover', {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export type DrApplyRequest = {
  namespace: string;
  vm_name: string;
  target_namespace?: string;
  target_name?: string;
  virtual_machine?: unknown;
  restore_latest_snapshot?: boolean;
  dry_run?: boolean;
};

export type DrApplyResponse = {
  status: string;
  target_namespace: string;
  target_name: string;
  message: string;
  snapshot_name?: string | null;
};

export function drApply(body: DrApplyRequest): Promise<DrApplyResponse> {
  return apiJson<DrApplyResponse>('/dr/apply', {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export type ImportDataVolumeRequest = {
  name: string;
  namespace: string;
  url?: string;
  registry?: string;
  storage_class?: string;
  size?: string;
};

export function importDataVolume(body: ImportDataVolumeRequest): Promise<{ message?: string; name?: string }> {
  return apiJson('/images/import', {
    method: 'POST',
    body: JSON.stringify({ size: '20Gi', ...body }),
  });
}

export type ClusterSummary = {
  name: string;
  context: string;
  environment: string;
  is_primary: boolean;
  health: string;
  vm_count: number;
  node_count: number;
  region: string;
};

export type ClustersResponse = {
  vmrogue_context: VmrogueFeatureContext;
  current_context: string;
  clusters: ClusterSummary[];
};

export function fetchClusters(): Promise<ClustersResponse> {
  return apiJson<ClustersResponse>('/clusters');
}

export function syncCluster(name: string): Promise<ClusterSummary> {
  return apiJson<ClusterSummary>(`/clusters/${encodeURIComponent(name)}/sync`);
}

export type IntegrationStatusItem = {
  id: string;
  name: string;
  env_var: string;
  configured: boolean;
  endpoint: string | null;
  probe: string;
  feeds: string;
};

export type IntegrationsStatusResponse = {
  vmrogue_context: VmrogueFeatureContext;
  integrations: IntegrationStatusItem[];
  configured_count: number;
};

export function fetchIntegrationsStatus(): Promise<IntegrationsStatusResponse> {
  return apiJson<IntegrationsStatusResponse>('/integrations/status');
}

export type CiliumStatusRecord = {
  version: string;
  agent_count: number;
  healthy_agents: number;
  cluster_mesh_enabled: boolean;
  hubble_enabled: boolean;
  encryption_enabled: boolean;
  installation_namespace: string;
};

export type CiliumPolicyRecord = {
  name: string;
  namespace: string;
  enforcement: string;
  ingress_rules: number;
  egress_rules: number;
  policy_kind: string;
};

export function fetchCiliumStatus(): Promise<CiliumStatusRecord> {
  return apiJson<CiliumStatusRecord>('/cilium/status');
}

export function fetchCiliumPolicies(namespace = 'all'): Promise<CiliumPolicyRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<CiliumPolicyRecord[]>(`/cilium/policies${q}`);
}

export type AutoscalerPolicyRecord = {
  name: string;
  namespace: string;
  target_kind: string;
  target_name: string;
  min_replicas: number;
  max_replicas: number;
  current_replicas: number;
  cpu_threshold?: number | null;
  memory_threshold?: number | null;
  enabled: boolean;
};

export function fetchAutoscalerPolicies(namespace = 'all'): Promise<AutoscalerPolicyRecord[]> {
  const q = nsQuery(namespace, true);
  return apiJson<AutoscalerPolicyRecord[]>(`/autoscaler/policies${q}`);
}

export function createAutoscalerPolicy(
  namespace: string,
  body: {
    name: string;
    target_name: string;
    min_replicas: number;
    max_replicas: number;
    cpu_threshold?: number;
    memory_threshold?: number;
  },
): Promise<AutoscalerPolicyRecord> {
  const q =
    namespace && namespace !== 'all'
      ? `?namespace=${encodeURIComponent(namespace)}`
      : '';
  return apiJson<AutoscalerPolicyRecord>(`/autoscaler/policies${q}`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}
