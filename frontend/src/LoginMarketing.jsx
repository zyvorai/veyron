import {
  Server, HardDrive, Camera, ShieldCheck, Cpu, Network, Gauge, KeyRound, Boxes, Activity, Layers, Cloud,
} from 'lucide-react';

/*
 * Marketing chapter for the sign-in page. Every claim below maps to a feature that ships in this
 * repo (templates, snapshots, DR, GPU passthrough, golden images, RBAC/OIDC, Cilium, Atlas) — no
 * invented customers, benchmarks or SLAs. Keep it that way.
 */
const FEATURES = [
  { icon: Server, title: 'Create VMs from templates', body: '40+ OS templates — Linux and Windows — with sane CPU, memory and firmware defaults. Pick one, press create.' },
  { icon: Gauge, title: 'Live resize', body: 'Hot-add CPU and memory to a running VM and hot-plug disks, without a reboot.' },
  { icon: Camera, title: 'Snapshots and restore', body: 'Point-in-time VM snapshots, schedules and retention, with app-consistent snapshots on demand.' },
  { icon: Cloud, title: 'Backup and DR', body: 'Velero off-cluster backups and a dry-run-first failback flow, so recovery is rehearsed, not improvised.' },
  { icon: HardDrive, title: 'Golden images', body: 'Upload an ISO or QCOW2, publish it once, and clone a private disk per VM — Windows included.' },
  { icon: Cpu, title: 'GPU passthrough', body: 'Per-node GPU inventory, passthrough and vGPU awareness, with migration guards that stop unsafe moves.' },
  { icon: Network, title: 'Network-aware', body: 'Per-VM egress control through Cilium, SSH and RDP exposure with secure defaults, flow visibility.' },
  { icon: ShieldCheck, title: 'Role-based access', body: 'Read-only, write and admin roles enforced on every route, with OIDC single sign-on for your IdP.' },
  { icon: Boxes, title: 'Ceph-backed storage', body: 'Shared block and file storage through Atlas and Rook-Ceph, with snapshot and clone on real volumes.' },
];

const STATS = [
  { value: '40+', label: 'OS templates' },
  { value: '3', label: 'access roles' },
  { value: '1', label: 'console for the fleet' },
  { value: 'REST', label: 'API for everything you click' },
];

const STEPS = [
  { n: '1', icon: Layers, title: 'Choose a template', body: 'Start from a tested OS template or your own golden image.' },
  { n: '2', icon: Activity, title: 'Create and watch it boot', body: 'Live status, VNC and serial console in the browser.' },
  { n: '3', icon: KeyRound, title: 'Operate day-2', body: 'Resize, snapshot, migrate, back up and expose — from one place.' },
];

export function LoginMarketing() {
  return (
    <section className="login-chapter login-chapter-marketing" aria-label="What Veyron does">
      <div className="lm-wrap">
        <p className="lm-kicker">Why Veyron</p>
        <h2 className="lm-title">Everything you do to a VM, in one console.</h2>
        <p className="lm-lede">
          Veyron turns KubeVirt into a private cloud your whole team can run: create, resize, protect and
          recover virtual machines without stitching together kubectl, YAML and dashboards.
        </p>

        <div className="lm-stats" role="list">
          {STATS.map((s) => (
            <div key={s.label} className="lm-stat" role="listitem">
              <b>{s.value}</b>
              <span>{s.label}</span>
            </div>
          ))}
        </div>

        <div className="lm-grid">
          {FEATURES.map(({ icon: Icon, title, body }) => (
            <article key={title} className="lm-card">
              <Icon size={22} aria-hidden />
              <h3>{title}</h3>
              <p>{body}</p>
            </article>
          ))}
        </div>

        <h3 className="lm-subtitle">From zero to running in three steps</h3>
        <ol className="lm-steps">
          {STEPS.map(({ n, icon: Icon, title, body }) => (
            <li key={n} className="lm-step">
              <span className="lm-step-n">{n}</span>
              <Icon size={20} aria-hidden />
              <div>
                <b>{title}</b>
                <p>{body}</p>
              </div>
            </li>
          ))}
        </ol>

        <p className="lm-foot">Built on KubeVirt and CDI · Cilium networking · Ceph storage · by ZyvorAI</p>
      </div>
    </section>
  );
}
