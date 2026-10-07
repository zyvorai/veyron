import {
  Monitor, Server, Disc3, Hexagon, Package, HardDrive, Camera, Archive, Network,
  Cpu, Bell, Shield, Database, ArrowLeftRight, Activity, Map, Cloud, Radar, Waypoints,
  Lightbulb, ListTree, Siren, ShieldAlert, ScrollText, Boxes, Ship, Gauge, Puzzle, Layers,
  Unlink, CalendarClock, Route, Wallet, Target, FileText, ClipboardCheck, Crosshair, Users,
  GitBranch, Globe, HeartPulse, Shuffle, Sparkles, ClipboardList, SearchCheck, Container, BrainCircuit,
} from 'lucide-react';

/** Schema only — rows come from the API. */
export const RES_META = {
  vms: {
    l: 'Virtual machines',
    empty: 'No machines yet.',
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
      ['age', 'Age', 'n'],
    ],
    insp: [
      ['Guest OS', 'os'],
      ['Host', 'host'],
      ['IP', 'ip'],
      ['Disk', 'diskSrc'],
      ['Age', 'age'],
      ['Namespace', 'ns'],
    ],
    spark: true,
    acts: ['start', 'stop', 'restart', 'pause', 'unpause', 'migrate', 'console', 'snapshot', 'clone', 'delete'],
    canCreate: true,
  },
  hosts: {
    l: 'Hosts',
    empty: 'No hosts reported.',
    I: Server,
    kind: 'Host',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['cpu', 'CPU', 'meter'],
      ['mem', 'Memory', 'meter'],
      ['vms', 'VMs', 'n'],
      ['kernel', 'Kernel', 'm'],
      ['age', 'Age', 'n'],
    ],
    insp: [
      ['Kernel', 'kernel'],
      ['Cores', 'cores'],
      ['Memory', 'memTotal'],
      ['VMs', 'vms'],
      ['Age', 'age'],
    ],
    acts: ['cordon', 'uncordon', 'reboot'],
  },
  gpus: {
    l: 'GPUs',
    empty: 'No GPUs advertised.',
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
      ['GPU Operator workload', 'workload'],
      ['Resources', 'resources'],
      ['Memory', 'memory'],
      ['Allocatable', 'count'],
    ],
    acts: [],
  },
  images: {
    l: 'Images & ISOs',
    empty: 'No images yet.',
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
    empty: 'No active pods.',
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
    empty: 'No templates found.',
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
    empty: 'No migrations in flight.',
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
    empty: 'No volumes yet.',
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
    empty: 'No snapshots yet.',
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
    empty: 'No backup policies yet.',
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
    empty: 'No Atlas volumes yet.',
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
    empty: 'No extra networks.',
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
    empty: 'Nothing is firing.',
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
      ['Fired', 'firedAt'],
    ],
    acts: ['resolve'],
  },
  soc: {
    l: 'Security',
    empty: 'No detections.',
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
  recommendations: {
    l: 'Recommendations',
    empty: 'Nothing to tune right now.',
    I: Lightbulb,
    kind: 'Recommendation',
    cols: [
      ['name', 'Recommendation'],
      ['status', 'Priority', 'st'],
      ['category', 'Category', 'm'],
      ['resource', 'Resource', 'm'],
      ['savings', 'Est. savings / mo', 'n'],
      ['effort', 'Effort', 'm'],
    ],
    insp: [
      ['Details', 'description'],
      ['Resource', 'resource'],
      ['Impact', 'impact'],
      ['Effort', 'effort'],
      ['Est. savings / mo', 'savings'],
    ],
    acts: [],
  },
  events: {
    l: 'Events',
    chrono: true,
    empty: 'No recent cluster events.',
    I: ListTree,
    kind: 'Event',
    cols: [
      ['name', 'Object'],
      ['status', 'Type', 'st'],
      ['reason', 'Reason', 'm'],
      ['ns', 'Namespace', 'm'],
      ['message', 'Message', 'm'],
      ['age', 'When', 'n'],
    ],
    insp: [
      ['Reason', 'reason'],
      ['Namespace', 'ns'],
      ['Message', 'message'],
      ['Time', 'at'],
    ],
    acts: [],
  },
  incidents: {
    l: 'Incidents',
    chrono: true,
    empty: 'No incidents. Everything is within target.',
    I: Siren,
    kind: 'Incident',
    cols: [
      ['name', 'Incident'],
      ['status', 'Severity', 'st'],
      ['kind', 'Kind', 'm'],
      ['vm', 'VM', 'm'],
      ['ns', 'Namespace', 'm'],
      ['age', 'When', 'n'],
    ],
    insp: [
      ['Details', 'description'],
      ['Kind', 'kind'],
      ['VM', 'vm'],
      ['Namespace', 'ns'],
      ['Time', 'at'],
    ],
    acts: [],
  },
  findings: {
    l: 'Security findings',
    empty: 'No findings. VM specs pass every configuration rule.',
    I: ShieldAlert,
    kind: 'Finding',
    cols: [
      ['name', 'Finding'],
      ['status', 'Severity', 'st'],
      ['category', 'Category', 'm'],
      ['resource', 'Resource', 'm'],
      ['age', 'Detected', 'n'],
    ],
    insp: [
      ['Details', 'description'],
      ['Fix', 'recommendation'],
      ['Resource', 'resource'],
      ['Category', 'category'],
    ],
    acts: [],
  },
  audit: {
    l: 'Audit trail',
    chrono: true,
    empty: 'No audit entries yet.',
    I: ScrollText,
    kind: 'Audit entry',
    cols: [
      ['name', 'Resource'],
      ['status', 'Outcome', 'st'],
      ['action', 'Action', 'm'],
      ['user', 'Actor', 'm'],
      ['type', 'Kind', 'm'],
      ['ns', 'Namespace', 'm'],
      ['age', 'When', 'n'],
    ],
    insp: [
      ['Details', 'details'],
      ['Actor', 'user'],
      ['Action', 'action'],
      ['Kind', 'type'],
      ['Namespace', 'ns'],
      ['Time', 'at'],
    ],
    acts: [],
  },
  operators: {
    l: 'Operators',
    empty: 'No operators found.',
    I: Boxes,
    kind: 'Operator',
    cols: [
      ['name', 'Operator'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['version', 'Version', 'm'],
      ['managed', 'Manages', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Version', 'version'],
      ['Manages', 'managedList'],
    ],
    acts: [],
  },
  helm: {
    l: 'Helm releases',
    empty: 'No Helm releases installed.',
    I: Ship,
    kind: 'Release',
    cols: [
      ['name', 'Release'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['chart', 'Chart', 'm'],
      ['app', 'App version', 'm'],
      ['revision', 'Revision', 'n'],
      ['age', 'Updated', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Chart', 'chart'],
      ['App version', 'app'],
      ['Revision', 'revision'],
    ],
    acts: [],
  },
  quotas: {
    l: 'Namespaces & quotas',
    empty: 'No resource quotas set.',
    I: Gauge,
    kind: 'Quota',
    cols: [
      ['name', 'Quota'],
      ['ns', 'Namespace', 'm'],
      ['cpu', 'CPU used / limit', 'm'],
      ['memory', 'Memory used / limit', 'm'],
      ['vms', 'VMs used / limit', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['CPU', 'cpu'],
      ['Memory', 'memory'],
      ['VMs', 'vms'],
    ],
    extraTitle: 'Namespaces',
    extraCols: [['name', 'Namespace']],
    acts: [],
  },
  crds: {
    l: 'Custom resources',
    empty: 'No custom resource definitions.',
    I: Puzzle,
    kind: 'CRD',
    cols: [
      ['name', 'Kind'],
      ['group', 'Group', 'm'],
      ['version', 'Version', 'm'],
      ['scope', 'Scope', 'm'],
      ['count', 'Instances', 'n'],
    ],
    insp: [
      ['CRD', 'crd'],
      ['Group', 'group'],
      ['Version', 'version'],
      ['Scope', 'scope'],
      ['Instances', 'count'],
    ],
    extraTitle: 'Veyron resources',
    extraCols: [
      ['name', 'Kind'],
      ['count', 'Instances', 'n'],
    ],
    acts: [],
  },
  workloads: {
    l: 'Workloads',
    empty: 'No workloads.',
    I: Layers,
    kind: 'Workload',
    cols: [
      ['name', 'Name'],
      ['status', 'Status', 'st'],
      ['type', 'Kind', 'm'],
      ['ns', 'Namespace', 'm'],
      ['ready', 'Ready', 'n'],
      ['cpu', 'CPU request', 'm'],
      ['memory', 'Memory request', 'm'],
    ],
    insp: [
      ['Kind', 'type'],
      ['Namespace', 'ns'],
      ['Ready', 'ready'],
      ['CPU request', 'cpu'],
      ['Memory request', 'memory'],
    ],
    acts: [],
  },
  orphans: {
    l: 'Orphan volumes',
    empty: 'No orphaned volumes. Every claim is in use.',
    I: Unlink,
    kind: 'Volume',
    cols: [
      ['name', 'Claim'],
      ['status', 'Status', 'st'],
      ['ns', 'Namespace', 'm'],
      ['sc', 'Class', 'm'],
      ['size', 'Size', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Class', 'sc'],
      ['Size', 'size'],
    ],
    acts: ['reclaim'],
  },
  schedules: {
    l: 'Snapshot schedules',
    empty: 'No snapshot schedules.',
    I: CalendarClock,
    kind: 'Schedule',
    cols: [
      ['name', 'Schedule'],
      ['status', 'Status', 'st'],
      ['vm', 'VM', 'm'],
      ['ns', 'Namespace', 'm'],
      ['cron', 'Cron', 'm'],
      ['keep', 'Keep', 'n'],
      ['lastRun', 'Last run', 'n'],
    ],
    insp: [
      ['VM', 'vm'],
      ['Cron', 'cron'],
      ['Prefix', 'prefix'],
      ['Keep', 'keep'],
      ['Last run', 'lastRun'],
    ],
    acts: ['delete'],
    canCreate: true,
  },
  netpols: {
    l: 'Policies & ingress',
    empty: 'No Kubernetes NetworkPolicies.',
    I: Route,
    kind: 'NetworkPolicy',
    cols: [
      ['name', 'Policy'],
      ['ns', 'Namespace', 'm'],
      ['types', 'Types', 'm'],
      ['selector', 'Pod selector', 'm'],
      ['ingress', 'Ingress rules', 'n'],
      ['egress', 'Egress rules', 'n'],
      ['age', 'Age', 'n'],
    ],
    insp: [
      ['Namespace', 'ns'],
      ['Types', 'types'],
      ['Pod selector', 'selector'],
      ['Ingress rules', 'ingress'],
      ['Egress rules', 'egress'],
    ],
    extraTitle: 'Ingresses',
    extraCols: [
      ['name', 'Ingress'],
      ['ns', 'Namespace', 'm'],
      ['cls', 'Class', 'm'],
      ['hosts', 'Hosts', 'm'],
      ['tls', 'TLS', 'm'],
      ['backends', 'Backends', 'm'],
    ],
    acts: [],
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
      ['gryvia', 'Gryvia', 'Container GPU nodes, tenants and GPU-hour usage.'],
      ['templates', null, 'Current OS releases, ready to boot.'],
      ['images', null, 'Golden images and import jobs.'],
      ['pods', null, 'Workloads running next to your VMs.'],
      ['workloads', null, 'Deployments, StatefulSets and DaemonSets.'],
      ['migrations', null, 'Live migrations in flight.'],
    ],
  },
  {
    g: 'Storage & Network',
    tone: 'amber',
    items: [
      ['pvcs', null, 'Volumes, classes and resize.'],
      ['snapshots', null, 'Point-in-time copies and restore.'],
      ['schedules', null, 'Recurring VM snapshots on a cron.'],
      ['backups', null, 'Scheduled and on-demand backups.'],
      ['storagehealth', 'Storage health', 'Usage per volume and per pool.'],
      ['orphans', null, 'Bound claims no VM references, and reclaim.'],
      ['atlas', null, 'Ceph snapshots and S3 backups via Atlas.'],
      ['dr', 'DR & Velero', 'Failover, failback and cluster backups.'],
      ['networks', null, 'Secondary networks and CIDRs.'],
      ['cilium', 'Cilium', 'Agents, policies and recent flows.'],
      ['netpols', null, 'NetworkPolicies and Ingress routes.'],
      ['netra', 'Netra', 'eBPF flows, drops and VM network view.'],
      ['paqtra', 'Paqtra', 'Cilium flow history, drops and policy posture.'],
    ],
  },
  {
    g: 'Security',
    tone: 'rose',
    items: [
      ['alerts', null, 'Warnings across the cluster.'],
      ['soc', null, 'Detections, hunts and SIEM export.'],
      ['hunting', 'Threat hunting', 'Hunts, playbooks, attack surface and SIEM export.'],
      ['findings', null, 'Configuration risks found in VM specs.'],
      ['compliance', 'Compliance', 'CIS and framework scores with reports.'],
      ['audit', null, 'Who changed what, and when.'],
      ['users', 'Users & roles', 'Console accounts and Kubernetes RBAC.'],
    ],
  },
  {
    g: 'Operations',
    tone: 'emerald',
    items: [
      ['enterprise', 'Workflow center', 'Readiness assessments and durable VM power requests.'],
      ['costs', 'Costs', 'Monthly estimate, forecast and budgets.'],
      ['recommendations', null, 'Right-sizing and savings ideas.'],
      ['slo', 'SLOs', 'Availability targets and error-budget burn.'],
      ['incidents', null, 'Breaches and failures on one timeline.'],
      ['events', null, 'Recent Kubernetes events, cluster-wide.'],
      ['logs', 'Logs', 'Search guest launcher logs.'],
    ],
  },
  {
    g: 'Platform',
    tone: 'graphite',
    items: [
      ['operators', null, 'Controllers and what they manage.'],
      ['helm', null, 'Installed charts and revisions.'],
      ['quotas', null, 'Namespaces and their resource limits.'],
      ['gitops', 'GitOps & catalog', 'Repository sync and the template catalog.'],
      ['crds', null, 'Every CRD and how many objects it has.'],
      ['clusters', 'Clusters', 'Kubeconfig contexts this API can switch to.'],
    ],
  },
  {
    g: 'AI',
    tone: 'violet',
    items: [
      ['ai', 'Veyron AI', 'Assistant, MCP, model and guardrails.'],
      ['proposals', 'Proposals', 'Changes the AI drafted, waiting for a human.'],
      ['investigations', 'Investigations', 'Root-cause reports for failures, written automatically.'],
      ['sandboxes', 'Sandboxes', 'Disposable VMs where agents run code.'],
      ['models', 'Models', 'Serve an open model inside your cluster.'],
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

/** Dashboards served by insights.jsx rather than the resource table. */
export const INSIGHT_PAGES = [
  'costs', 'slo', 'logs', 'compliance', 'hunting', 'users', 'gitops', 'clusters', 'storagehealth', 'cilium',
];

/** Veyron AI pages, served by aipages.jsx. */
export const AI_PAGES = ['ai', 'proposals', 'investigations', 'sandboxes', 'models'];

export const CHAPTER_PAGES = new Set(['monitoring', 'topology', 'dr', 'netra', 'paqtra', 'gryvia', 'enterprise', ...INSIGHT_PAGES, ...AI_PAGES]);

/** Story pages that render full-bleed, without the local toolbar. */
export const FULL_BLEED_PAGES = new Set([
  'mission', 'console', 'monitoring', 'topology', 'dr', 'netra', 'paqtra', 'gryvia', 'enterprise', ...INSIGHT_PAGES, ...AI_PAGES,
]);

export const CHAPTER_ICONS = {
  enterprise: ClipboardCheck,
  monitoring: Activity,
  topology: Map,
  dr: Cloud,
  netra: Radar,
  paqtra: Waypoints,
  gryvia: Cpu,
  costs: Wallet,
  slo: Target,
  logs: FileText,
  compliance: ClipboardCheck,
  hunting: Crosshair,
  users: Users,
  gitops: GitBranch,
  clusters: Globe,
  storagehealth: HeartPulse,
  cilium: Shuffle,
  ai: Sparkles,
  proposals: ClipboardList,
  investigations: SearchCheck,
  sandboxes: Container,
  models: BrainCircuit,
};
