/* Copyright 2026 Zyvor AI Labs · https://zyvor.dev
 * SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 */
export const ASSESSMENT_KINDS = ['blueprint', 'placement', 'maintenance', 'migration', 'gpu', 'recovery', 'fleet'];
export function assessmentPath(kind) {
  if (!ASSESSMENT_KINDS.includes(kind)) throw new Error('Unknown assessment');
  if (kind === 'blueprint') return '/api/v1/enterprise/blueprints/validate';
  if (kind === 'placement') return '/api/v1/enterprise/placement';
  return `/api/v1/enterprise/assess/${kind}`;
}
export function operationQuery(namespace) {
  if (!namespace || namespace === 'all' || !/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(namespace) || namespace.length > 63) throw new Error('Enter one namespace');
  return `namespace=${encodeURIComponent(namespace)}`;
}
export function parseAssessment(text) {
  const value = JSON.parse(text);
  if (!value || typeof value !== 'object') throw new Error('Assessment must be an object or array');
  return value;
}
export const SAMPLES = {
  blueprint: { name: 'web-stack', namespace: 'default', machines: [
    {name:'db',resources:{cpu_millis:2000,memory_bytes:4294967296,gpu:0},image_ref:'registry.example/db:v1',depends_on:[]},
    {name:'web',resources:{cpu_millis:1000,memory_bytes:2147483648,gpu:0},image_ref:'registry.example/web:v1',depends_on:['db']},
  ] },
  placement: {resources:{cpu_millis:2000,memory_bytes:4294967296,gpu:0},node_selector:{},excluded_nodes:[],headroom_percent:10},
  maintenance: {source_node:'node-a',headroom_percent:10,vms:[{name:'web',resources:{cpu_millis:2000,memory_bytes:4294967296,gpu:0},migration_ready:false,local_storage:false,whole_gpu:false}],nodes:[{name:'node-b',available:{cpu_millis:8000,memory_bytes:17179869184,gpu:0},ready:true,schedulable:true,tainted:false,labels:{}}]},
  migration:{conversion_complete:false,virtio_drivers:false,boot_verified:false,network_mapped:false,storage_mapped:false,source_fenced:false,application_verified:false},
  gpu:{device_advertised:false,iommu:false,guest_driver:false,license_valid:false,isolation_verified:false,whole_gpu:true,live_migration_requested:false},
  recovery:{snapshot_at:0,incident_at:0,recovered_at:0,now:0,target_rpo_seconds:300,target_rto_seconds:900,snapshot_ready:false,isolated:false,source_fenced:false,boot_verified:false,application_verified:false},
  fleet:[{context:'primary',control_plane_id:'primary',ready:false,storage_replication_verified:false,isolated_recovery_verified:false},{context:'recovery',control_plane_id:'recovery',ready:false,storage_replication_verified:false,isolated_recovery_verified:false}],
};
