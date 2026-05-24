/** VMRogue REST client (browser → same-origin /api/v1). */

import { getApiKey } from './auth';

const API = '/api/v1';

function authHeaders(): HeadersInit {
  const apiKey = getApiKey();
  if (apiKey) {
    return { 'X-API-Key': apiKey };
  }
  const token = typeof localStorage !== 'undefined' ? localStorage.getItem('vmrogue_token') : null;
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function apiJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(),
      ...(init?.headers ?? {}),
    },
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg =
      (body as { error?: { message?: string } })?.error?.message ||
      (body as { message?: string })?.message ||
      res.statusText;
    throw new Error(msg);
  }
  const wrapped = body as { data?: T };
  return (wrapped.data !== undefined ? wrapped.data : body) as T;
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
  const q = namespace && namespace !== "all" ? `?namespace=${encodeURIComponent(namespace)}` : "";
  return apiJson<VmRecord[]>(`/vms${q}`);
}

export async function fetchAlerts(namespace?: string): Promise<AlertRecord[]> {
  const q = namespace && namespace !== 'all' ? `?namespace=${encodeURIComponent(namespace)}` : '';
  return apiJson<AlertRecord[]>(`/alerts${q}`).catch(() => []);
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

export async function stopVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'stop');
}

export async function startVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'start');
}

export async function restartVirtualMachine(namespace: string, name: string): Promise<void> {
  await vmLifecycleAction(namespace, name, 'restart');
}

async function vmLifecycleAction(namespace: string, name: string, action: 'start' | 'stop' | 'restart'): Promise<void> {
  const res = await fetch(
    `${API}/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/${action}`,
    {
      method: 'POST',
      headers: authHeaders(),
    },
  );
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg =
      (body as { error?: { message?: string } })?.error?.message ||
      (body as { message?: string })?.message ||
      res.statusText;
    throw new Error(msg);
  }
}

export type RdpGuestAgentResult = {
  success: boolean
  guest_agent_connected: boolean
  is_windows_vm: boolean
  message: string
  guest_exec: unknown
  exit_code?: number | null
  stdout?: string | null
  stderr?: string | null
}

export type RdpExposeStatus = {
  guest_ip?: string | null
  is_windows_vm: boolean
  exposed: boolean
  node_port?: number | null
  cluster_ip?: string | null
  service_name: string
  service_type?: string | null
  rdp_via_nodeport_example?: string | null
  vm_spec_has_rdp_port: boolean
  suggested_node_port?: number | null
}

export type VmDetail = VmRecord & {
  vmi_status?: {
    phase?: string
    conditions?: Array<{ type?: string; type_?: string; status?: string }>
  }
}

export function fetchVmDetail(namespace: string, name: string): Promise<VmDetail> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}`)
}

export function fetchRdpExpose(namespace: string, name: string): Promise<RdpExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/rdp-expose`)
}

export function putRdpExpose(
  namespace: string,
  name: string,
  body: { enabled: boolean; service_type?: string; node_port?: number }
): Promise<RdpExposeStatus> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/rdp-expose`, {
    method: "PUT",
    body: JSON.stringify(body),
  })
}

/** Enable Windows RDP inside the guest via QEMU guest-agent (guest-exec). */
export async function enableRdpViaGuestAgent(
  namespace: string,
  name: string
): Promise<RdpGuestAgentResult> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/guest-agent/enable-rdp`, {
    method: "POST",
    body: "{}",
  })
}

/** Disable Windows RDP inside the guest via QEMU guest-agent (guest-exec). */
export async function disableRdpViaGuestAgent(
  namespace: string,
  name: string
): Promise<RdpGuestAgentResult> {
  return apiJson(`/vms/${encodeURIComponent(namespace)}/${encodeURIComponent(name)}/guest-agent/disable-rdp`, {
    method: "POST",
    body: "{}",
  })
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
  }
): Promise<AddDataDiskResponse> {
  return apiJson(`/vms/${encodeURIComponent(ns)}/${encodeURIComponent(name)}/storage/data-disk`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}
