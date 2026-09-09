import {
  Monitor, Server, Disc3, Hexagon, Package, HardDrive, Camera, Archive, Network,
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
    acts: ['start', 'stop', 'restart', 'pause', 'migrate', 'console', 'snapshot', 'clone', 'delete'],
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
    acts: ['delete'],
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
    ],
    acts: ['clone', 'delete'],
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
  },
};

export function emptyData() {
  const out = {};
  for (const [k, meta] of Object.entries(RES_META)) {
    out[k] = { ...meta, rows: [], extra: meta.extraTitle ? { title: meta.extraTitle, cols: meta.extraCols, rows: [] } : undefined };
  }
  return out;
}

export const NAV = [
  { g: 'Overview', items: [['mission', 'Mission Control'], ['console', 'ConsoleHub']] },
  { g: 'Compute', items: [['vms'], ['hosts'], ['images'], ['pods'], ['templates']] },
  { g: 'Storage & network', items: [['pvcs'], ['snapshots'], ['backups'], ['networks']] },
  { g: 'System', items: [['settings', 'Settings']] },
];

/** Flat page order for 1:1 swipe navigation (matches sidebar top→bottom). */
export const PAGE_ORDER = NAV.flatMap((g) => g.items.map(([id]) => id));
