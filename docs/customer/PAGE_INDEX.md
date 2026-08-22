# Veyron — Complete page index

Every primary navigable dashboard route.

_Generated: 2026-08-22 · 65 routes_

Regenerate: `node scripts/customer-docs/generate-page-index.mjs`

## Favorites

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Mission Control | `dashboard` | Mission Control home — fleet health, shortcuts, and live KubeVirt signals. | [Open](pages/favorites/dashboard.md) |
| Fleet Constellation | `fleet-constellation` | Visual constellation of the VM fleet across namespaces and nodes. | [Open](pages/favorites/fleet-constellation.md) |
| Reactor Core | `reactor` | Reactor Core — high-density operational wall for hot workloads. | [Open](pages/favorites/reactor.md) |
| Gallery Wall | `gallery-wall` | Gallery of VM cards and status tiles for at-a-glance browsing. | [Open](pages/favorites/gallery-wall.md) |
| vCentre | `vcentre` | vCentre-style inventory and ops surface for KubeVirt VMs. | [Open](pages/favorites/vcentre.md) |
| ConsoleHub | `console-hub` | Console hub for VNC, serial, and remote access sessions. | [Open](pages/favorites/console-hub.md) |
| VM Capsule | `vm-capsule` | Focused single-VM capsule view for deep workbench ops. | [Open](pages/favorites/vm-capsule.md) |

## Compute

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| VMs | `vms` | VirtualMachine inventory — create, power, migrate, and inspect KubeVirt VMs. | [Open](pages/compute/vms.md) |
| Nodes | `nodes` | Kubernetes / virt node capacity and placement view. | [Open](pages/compute/nodes.md) |
| Pods | `pods` | Pod inventory including virt-launcher pods. | [Open](pages/compute/pods.md) |
| Workloads | `workloads` | Cross-workload view of Deployments, StatefulSets, and VMs. | [Open](pages/compute/workloads.md) |
| Scheduling | `scheduling` | Scheduling constraints, node affinity, and placement hints. | [Open](pages/compute/scheduling.md) |
| HPA | `hpa` | Horizontal Pod Autoscaler objects affecting related workloads. | [Open](pages/compute/hpa.md) |
| Autoscaler | `autoscaler` | Cluster / VM autoscaler status and recommendations. | [Open](pages/compute/autoscaler.md) |

## Observe

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Monitoring | `monitoring` | Monitoring overview for the Veyron-managed estate. | [Open](pages/observe/monitoring.md) |
| Metrics | `metrics` | Live and historical metrics for VMs and nodes. | [Open](pages/observe/metrics.md) |
| Alerts | `alerts` | Active alerts from the observability stack. | [Open](pages/observe/alerts.md) |
| Events | `events` | Kubernetes and KubeVirt events. | [Open](pages/observe/events.md) |
| SLO | `slo` | Service level objectives and burn-rate views. | [Open](pages/observe/slo.md) |
| Performance | `performance` | Performance profiles and hotspots across the fleet. | [Open](pages/observe/performance.md) |
| Heatmap | `heatmap` | Density heatmap of resource pressure. | [Open](pages/observe/heatmap.md) |
| Custom Dashboards | `custom-dashboards` | User-defined observability dashboards. | [Open](pages/observe/custom-dashboards.md) |
| Observability Overview | `observability` | Observability overview landing page. | [Open](pages/observe/observability.md) |
| Insights | `insights` | AI / analytics insights over fleet telemetry. | [Open](pages/observe/insights.md) |
| Traces | `traces` | Distributed traces when tracing backends are configured. | [Open](pages/observe/traces.md) |
| Logs | `logs` | Log explorer for VM and platform workloads. | [Open](pages/observe/logs.md) |
| Incidents | `incidents` | Incident timeline and response hooks. | [Open](pages/observe/incidents.md) |
| Mission Control Wall | `mission-control` | Full-wall Mission Control display mode. | [Open](pages/observe/mission-control.md) |

## Storage

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Snapshots | `snapshots` | VolumeSnapshot / VM snapshot inventory and actions. | [Open](pages/storage/snapshots.md) |
| Storage | `storage` | PVC, DataVolume, and storage class inventory for VMs. | [Open](pages/storage/storage.md) |
| Images | `images` | ContainerDisk / golden image catalog. | [Open](pages/storage/images.md) |
| Backups | `backups` | Backup jobs and restore entry points for VMs. | [Open](pages/storage/backups.md) |

## Network

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Ingress | `ingress` | Ingress / route inventory for exposed services. | [Open](pages/network/ingress.md) |
| Network Policies | `network-policies` | Kubernetes NetworkPolicy objects affecting VM namespaces. | [Open](pages/network/network-policies.md) |
| Cilium | `cilium` | Cilium network status when Cilium is the CNI. | [Open](pages/network/cilium.md) |
| Topology | `topology` | Network topology across VMs, services, and nodes. | [Open](pages/network/topology.md) |
| Dependencies | `dependencies` | Service dependency graph for selected workloads. | [Open](pages/network/dependencies.md) |
| Network Intelligence | `network-intel` | Network intelligence summary for the virtualization plane. | [Open](pages/network/network-intel.md) |

## Security

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Security Posture | `security` | Security posture across VMs and namespaces. | [Open](pages/security/security.md) |
| Compliance | `compliance` | Compliance checks and evidence for the fleet. | [Open](pages/security/compliance.md) |
| RBAC | `rbac` | Kubernetes RBAC review for Veyron operators and tenants. | [Open](pages/security/rbac.md) |
| Policies | `policies` | Admission / VM policy enforcement surfaces. | [Open](pages/security/policies.md) |
| Quotas | `quotas` | ResourceQuota and LimitRange views. | [Open](pages/security/quotas.md) |
| Audit | `audit` | Audit trail of mutating Veyron and cluster actions. | [Open](pages/security/audit.md) |
| SOC | `soc` | SOC-style security operations board. | [Open](pages/security/soc.md) |

## Platform

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Helm | `helm` | Helm release inventory for the cluster. | [Open](pages/platform/helm.md) |
| Operators | `operators` | Operator / OLM status relevant to KubeVirt and companions. | [Open](pages/platform/operators.md) |
| CRDs | `crds` | CustomResourceDefinition catalog. | [Open](pages/platform/crds.md) |
| Blueprint Studio | `blueprint-studio` | Multi-VM blueprint authoring and apply. | [Open](pages/platform/blueprint-studio.md) |
| Custom Resources | `custom-resources` | Browse and edit custom resources. | [Open](pages/platform/custom-resources.md) |
| Catalog | `catalog` | Workload and template catalog. | [Open](pages/platform/catalog.md) |
| Template Foundry | `app-store` | Template Foundry — OS templates and app blueprints. | [Open](pages/platform/app-store.md) |
| GitOps | `gitops` | GitOps export and sync status for VeyronVM declarations. | [Open](pages/platform/gitops.md) |
| Integrations | `integrations` | External integrations (ticketing, chat, IdP). | [Open](pages/platform/integrations.md) |
| Webhooks | `webhooks` | Webhook endpoints for automation. | [Open](pages/platform/webhooks.md) |
| Tenants | `tenants` | Multi-tenant boundaries and namespace mapping. | [Open](pages/platform/tenants.md) |
| Stack Health | `stack-health` | End-to-end stack health for Veyron + KubeVirt + CDI. | [Open](pages/platform/stack-health.md) |
| Settings | `settings` | Dashboard, auth, and AI provider settings. | [Open](pages/platform/settings.md) |

## FinOps

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Costs | `costs` | FinOps cost view for VM and cluster spend. | [Open](pages/finops/costs.md) |
| Forecasting | `forecasting` | Cost and capacity forecasting. | [Open](pages/finops/forecasting.md) |

## Ops

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Chaos | `chaos` | Chaos experiments against the virtualization stack. | [Open](pages/ops/chaos.md) |
| Disaster Recovery | `dr` | Disaster recovery plans for VMs and namespaces. | [Open](pages/ops/dr.md) |
| Actions | `actions` | Queued / recent operator actions. | [Open](pages/ops/actions.md) |
| Notifications | `notifications` | Notification channels and delivery preferences. | [Open](pages/ops/notifications.md) |

## Ask Zeus

| Page | Route | Purpose | Guide |
|------|-------|---------|-------|
| Ask Zeus | `ask-zeus` | Ask Zeus — AI assistant grounded on this cluster. | [Open](pages/ask-zeus/ask-zeus.md) |

## Related

- [Customer docs home](README.md)
- [Page-by-page guides](pages/README.md)
