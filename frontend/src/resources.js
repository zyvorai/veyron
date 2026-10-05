import {
  Monitor, Server, Disc3, Hexagon, Package, HardDrive, Camera, Archive, Network,
  Cpu, Bell, Shield, Database, ArrowLeftRight, Activity, Map, Cloud, Radar,
} from 'lucide-react';

/** Schema only — rows come from the API. */
export const RES_META = {
  vms: {
    l: 'Virtual machines',
    I: Monitor,
    kind: 'VirtualMachine',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['cpu', 'vCPU', 'n'],
      ['ram', 'Memory', 'n', ' GiB'],
      ['disk', 'Disk', 'n', ' GB'],
      ['host', 'Host', 'm'],
      ['ip', 'IP', 'm'],
      ['age', 'Uptime', 'n'],
    ],
    insp: [
      ['Guest OS', 'os'],
      ['Host', 'host'],
      ['IP', 'ip'],
      ['Uptime', 'age'],
      ['Namespace', 'ns'],
    ],
    spark: true,
    acts: ['start', 'stop', 'restart', 'pause', 'unpause', 'migrate', 'console', 'snapshot', 'clone', 'delete'],
    canCreate: true,
  },
  hosts: {
    l: 'Hosts',
    I: Server,
    kind: 'Host',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['cpu', 'CPU', 'meter'],
      ['mem', 'Memory', 'meter'],
      ['vms', 'VMs', 'n'],
      ['kernel', 'Kernel', 'm'],
      ['age', 'Uptime', 'n'],
    ],
    insp: [
      ['Kernel', 'kernel'],
      ['Cores', 'cores'],
      ['Memory', 'memTotal'],
      ['VMs', 'vms'],
      ['Uptime', 'age'],
    ],
    acts: ['cordon', 'uncordon', 'reboot'],
  },
  gpus: {
    l: 'GPUs',
    I: Cpu,
    kind: 'GPU',
    cols: [
      ['name', 'Node'],
      ['status', 'Status', 'st'],
      ['product', 'Product', 'm'],
      ['count', 'Alloc', 'n'],
      ['kind', 'Type', 'm'],
      ['memory', 'Memory', 'm'],
    ],
    insp: [
      ['Product', 'product'],
      ['Type', 'kind'],
      ['Resources', 'resources'],
      ['Memory', 'memory'],
      ['Allocatable', 'count'],
    ],
    acts: [],
  },
  images: {
    l: 'Images & ISOs',
    I: Disc3,
    kind: 'Image',
    cols: [
      ['name', 'Name'],
      ['type', 'Type'],
      ['size', 'Size', 'n'],
      ['os', 'OS'],
      ['age', 'Added', 'n'],
    ],
    insp: [
      ['Type', 'type'],
      ['Size', 'size'],
      ['OS', 'os'],
      ['Source', 'src'],
      ['Namespace', 'ns'],
    ],
    acts: ['publish', 'delete'],
    canCreate: true,
    extraTitle: 'DataSources',
    extraCols: [
      ['name', 'Name'],
      ['cls', 'Namespace', 'm'],
      ['prov', 'Source', 'm'],
      ['total', 'Size', 'n'],
    ],
  },
  pods: {
    l: 'Pods',
    I: Hexagon,
    kind: 'Pod',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['ready', 'Ready', 'n'],
      ['restarts', 'Restarts', 'n'],
      ['node', 'Node', 'm'],
      ['age', 'Age', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Node', 'node'],
      ['Image', 'image'],
      ['Restarts', 'restarts'],
    ],
    acts: ['logs', 'delete'],
  },
  templates: {
    l: 'Template Foundry',
    I: Package,
    kind: 'Template',
    cols: [
      ['name', 'Name'],
      ['os', 'OS'],
      ['cpu', 'vCPU', 'n'],
      ['ram', 'Memory', 'n', ' GiB'],
      ['disk', 'Disk', 'n', ' GB'],
      ['uses', 'Used by', 'n'],
    ],
    insp: [
      ['OS', 'os'],
      ['Cloud-init', 'ci'],
      ['Used by', 'uses'],
      ['Tags', 'tags'],
    ],
    acts: ['clone', 'delete'],
    extraTitle: 'VMProfiles',
    extraCols: [
      ['name', 'Name'],
      ['os', 'Family', 'm'],
      ['cpu', 'vCPU', 'n'],
      ['ram', 'Memory', 'n'],
    ],
  },
  migrations: {
    l: 'Migrations',
    I: ArrowLeftRight,
    kind: 'Migration',
    cols: [
      ['name', 'ID'],
      ['status', 'Status', 'st'],
      ['vm', 'VM', 'm'],
      ['source', 'Source', 'm'],
      ['target', 'Target', 'm'],
      ['progress', 'Progress', 'n'],
      ['age', 'Started', 'n'],
    ],
    insp: [
      ['VM', 'vm'],
      ['Source', 'source'],
      ['Target', 'target'],
      ['Type', 'type'],
      ['Progress', 'progress'],
    ],
    acts: ['delete'],
  },
  pvcs: {
    l: 'Storage',
    I: HardDrive,
    kind: 'PersistentVolumeClaim',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['cls', 'Class', 'm'],
      ['size', 'Size', 'n'],
      ['used', 'Used', 'meter'],
      ['access', 'Access', 'm'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Class', 'cls'],
      ['Volume', 'vol'],
      ['Access', 'access'],
      ['Size', 'size'],
    ],
    acts: ['resize', 'delete'],
    canCreate: true,
    extraTitle: 'Storage pools',
    extraCols: [
      ['name', 'Name'],
      ['cls', 'Class', 'm'],
      ['prov', 'Provisioner', 'm'],
      ['total', 'Total', 'n'],
      ['used', 'Used', 'meter'],
      ['vols', 'Volumes', 'n'],
    ],
  },
  snapshots: {
    l: 'Snapshots',
    I: Camera,
    kind: 'Snapshot',
    cols: [
      ['name', 'Name'],
      ['vm', 'Machine', 'm'],
      ['size', 'Size', 'n'],
      ['age', 'Taken', 'n'],
      ['status', 'Status', 'st'],
    ],
    insp: [
      ['Machine', 'vm'],
      ['Size', 'size'],
      ['Taken', 'age'],
    ],
    acts: ['restore', 'delete'],
  },
  backups: {
    l: 'Backups',
    I: Archive,
    kind: 'Backup',
    cols: [
      ['name', 'Name'],
      ['target', 'Target', 'm'],
      ['items', 'Items', 'n'],
      ['size', 'Size', 'n'],
      ['age', 'Last run', 'n'],
      ['status', 'Status', 'st'],
    ],
    insp: [
      ['Target', 'target'],
      ['Schedule', 'sched'],
      ['Retention', 'ret'],
    ],
    acts: ['run', 'restore', 'delete'],
  },
  atlas: {
    l: 'Atlas',
    I: Database,
    kind: 'AtlasVolume',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['size', 'Size', 'n'],
      ['backend', 'Backend', 'm'],
      ['pvc', 'PVC', 'm'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Backend', 'backend'],
      ['PVC', 'pvc'],
      ['Size', 'size'],
    ],
    acts: [],
    extraTitle: 'Atlas snapshots',
    extraCols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['volume', 'Volume', 'm'],
      ['size', 'Size', 'n'],
      ['age', 'Taken', 'n'],
    ],
  },
  networks: {
    l: 'Networks',
    I: Network,
    kind: 'Network',
    cols: [
      ['name', 'Name'],
      ['type', 'Type'],
      ['cidr', 'CIDR', 'm'],
      ['vlan', 'VLAN', 'n'],
      ['attached', 'Attached', 'n'],
      ['status', 'Status', 'st'],
    ],
    insp: [
      ['Type', 'type'],
      ['CIDR', 'cidr'],
      ['VLAN', 'vlan'],
      ['Gateway', 'gw'],
      ['Namespace', 'ns'],
    ],
    acts: ['delete'],
    canCreate: true,
  },
  alerts: {
    l: 'Alerts',
    I: Bell,
    kind: 'Alert',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['severity', 'Severity', 'm'],
      ['message', 'Message', 'm'],
      ['source', 'Source', 'm'],
      ['age', 'Fired', 'n'],
    ],
    insp: [
      ['Severity', 'severity'],
      ['Message', 'message'],
      ['Source', 'source'],
      ['Fired', 'age'],
    ],
    acts: ['resolve'],
  },
  soc: {
    l: 'Security',
    I: Shield,
    kind: 'Detection',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['severity', 'Severity', 'm'],
      ['message', 'Message', 'm'],
      ['source', 'Source', 'm'],
      ['age', 'Seen', 'n'],
    ],
    insp: [
      ['Severity', 'severity'],
      ['Message', 'message'],
      ['Source', 'source'],
      ['Seen', 'age'],
    ],
    acts: ['ack'],
  },
};

export function emptyData() {
  const out = {};
  for (const [k, meta] of Object.entries(RES_META)) {
    out[k] = {
      ...meta,
      rows: [],
      extra: meta.extraTitle
        ? { title: meta.extraTitle, cols: meta.extraCols, rows: [] }
        : undefined,
    };
  }
  return out;
}

/** Global navigation (apple.com-style mega-menus). Each item: [pageId, label?, blurb]. */
export const NAV = [
  {
    g: 'Overview',
    tone: 'sky',
    items: [
      ['mission', 'Mission Control', 'The whole fleet at a glance, live.'],
      ['console', 'Consoles', 'Open a live screen on any running guest.'],
      ['monitoring', 'Monitoring', 'Capacity headroom and platform versions.'],
      ['topology', 'Topology', 'How hosts, machines and networks connect.'],
    ],
  },
  {
    g: 'Compute',
    tone: 'violet',
    items: [
      ['vms', null, 'Create, size, start and migrate machines.'],
      ['hosts', null, 'Nodes, load and maintenance.'],
      ['gpus', null, 'Passthrough and vGPU inventory per node.'],
      ['templates', null, 'Current OS releases, ready to boot.'],
      ['images', null, 'Golden images and import jobs.'],
      ['pods', null, 'Workloads running next to your VMs.'],
      ['migrations', null, 'Live migrations in flight.'],
    ],
  },
  {
    g: 'Storage & Network',
    tone: 'amber',
    items: [
      ['pvcs', null, 'Volumes, classes and resize.'],
      ['snapshots', null, 'Point-in-time copies and restore.'],
      ['backups', null, 'Scheduled and on-demand backups.'],
      ['atlas', null, 'Ceph snapshots and S3 backups via Atlas.'],
      ['dr', 'DR & Velero', 'Failover, failback and cluster backups.'],
      ['networks', null, 'Secondary networks and CIDRs.'],
      ['network-brain', 'PacketWolf', 'Flows and network intelligence.'],
    ],
  },
  {
    g: 'Security',
    tone: 'rose',
    items: [
      ['alerts', null, 'Warnings across the cluster.'],
      ['soc', null, 'Detections, hunts and SIEM export.'],
    ],
  },
  { g: 'Settings', tone: 'graphite', page: 'settings', items: [['settings', 'Settings', 'Accounts, policy and integrations.']] },
];

/** Every routable page, in menu order. */
export const PAGE_ORDER = NAV.flatMap((g) => g.items.map(([id]) => id));

/** Section-identity tone per page id, derived from NAV group membership. */
export const TONE_BY_PAGE = Object.fromEntries(NAV.flatMap((g) => g.items.map(([id]) => [id, g.tone || null])));

/** Display label per page id (falls back to the resource label). */
export function pageLabel(id) {
  for (const g of NAV) {
    const hit = g.items.find(([pid]) => pid === id);
    if (hit) return hit[1] || RES_META[id]?.l || id;
  }
  return RES_META[id]?.l || id;
}

/** Menu blurb per page id. */
export function pageGroup(id) {
  return NAV.find((g) => g.items.some(([pid]) => pid === id))?.g || '';
}

export function pageBlurb(id) {
  for (const g of NAV) {
    const hit = g.items.find(([pid]) => pid === id);
    if (hit) return hit[2] || '';
  }
  return '';
}

export const CHAPTER_PAGES = new Set(['monitoring', 'topology', 'dr', 'network-brain']);

/** Story pages that render full-bleed, without the local toolbar. */
export const FULL_BLEED_PAGES = new Set(['mission', 'console', 'monitoring', 'topology', 'dr', 'network-brain']);

export const CHAPTER_ICONS = {
  monitoring: Activity,
  topology: Map,
  dr: Cloud,
  'network-brain': Radar,
};
