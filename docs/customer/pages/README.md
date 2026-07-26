# Page-by-page guides

Each guide follows: Purpose → When to use it → How to get there → What you can do → Related pages.

Every route is also listed in the [complete page index](../PAGE_INDEX.md).

## Ask Zeus

| Page | What it covers |
|------|----------------|
| [Ask Zeus](ask-zeus/ask-zeus.md) | Ask Zeus — AI assistant grounded on this cluster. |

## Compute

| Page | What it covers |
|------|----------------|
| [Autoscaler](compute/autoscaler.md) | Cluster / VM autoscaler status and recommendations. |
| [HPA](compute/hpa.md) | Horizontal Pod Autoscaler objects affecting related workloads. |
| [Nodes](compute/nodes.md) | Kubernetes / virt node capacity and placement view. |
| [Pods](compute/pods.md) | Pod inventory including virt-launcher pods. |
| [Scheduling](compute/scheduling.md) | Scheduling constraints, node affinity, and placement hints. |
| [VMs](compute/vms.md) | VirtualMachine inventory — create, power, migrate, and inspect KubeVirt VMs. |
| [Workloads](compute/workloads.md) | Cross-workload view of Deployments, StatefulSets, and VMs. |

## Favorites

| Page | What it covers |
|------|----------------|
| [ConsoleHub](favorites/console-hub.md) | Console hub for VNC, serial, and remote access sessions. |
| [Mission Control](favorites/dashboard.md) | Mission Control home — fleet health, shortcuts, and live KubeVirt signals. |
| [Fleet Constellation](favorites/fleet-constellation.md) | Visual constellation of the VM fleet across namespaces and nodes. |
| [Gallery Wall](favorites/gallery-wall.md) | Gallery of VM cards and status tiles for at-a-glance browsing. |
| [Reactor Core](favorites/reactor.md) | Reactor Core — high-density operational wall for hot workloads. |
| [vCentre](favorites/vcentre.md) | vCentre-style inventory and ops surface for KubeVirt VMs. |
| [VM Capsule](favorites/vm-capsule.md) | Focused single-VM capsule view for deep workbench ops. |

## Finops

| Page | What it covers |
|------|----------------|
| [Costs](finops/costs.md) | FinOps cost view for VM and cluster spend. |
| [Forecasting](finops/forecasting.md) | Cost and capacity forecasting. |

## Network

| Page | What it covers |
|------|----------------|
| [Cilium](network/cilium.md) | Cilium network status when Cilium is the CNI. |
| [Dependencies](network/dependencies.md) | Service dependency graph for selected workloads. |
| [Ingress](network/ingress.md) | Ingress / route inventory for exposed services. |
| [Network Intelligence](network/network-intel.md) | Network intelligence summary for the virtualization plane. |
| [Network Policies](network/network-policies.md) | Kubernetes NetworkPolicy objects affecting VM namespaces. |
| [Topology](network/topology.md) | Network topology across VMs, services, and nodes. |

## Observe

| Page | What it covers |
|------|----------------|
| [Alerts](observe/alerts.md) | Active alerts from the observability stack. |
| [Custom Dashboards](observe/custom-dashboards.md) | User-defined observability dashboards. |
| [Events](observe/events.md) | Kubernetes and KubeVirt events. |
| [Heatmap](observe/heatmap.md) | Density heatmap of resource pressure. |
| [Incidents](observe/incidents.md) | Incident timeline and response hooks. |
| [Insights](observe/insights.md) | AI / analytics insights over fleet telemetry. |
| [Logs](observe/logs.md) | Log explorer for VM and platform workloads. |
| [Metrics](observe/metrics.md) | Live and historical metrics for VMs and nodes. |
| [Mission Control Wall](observe/mission-control.md) | Full-wall Mission Control display mode. |
| [Monitoring](observe/monitoring.md) | Monitoring overview for the Veyron-managed estate. |
| [Observability Overview](observe/observability.md) | Observability overview landing page. |
| [Performance](observe/performance.md) | Performance profiles and hotspots across the fleet. |
| [SLO](observe/slo.md) | Service level objectives and burn-rate views. |
| [Traces](observe/traces.md) | Distributed traces when tracing backends are configured. |

## Ops

| Page | What it covers |
|------|----------------|
| [Actions](ops/actions.md) | Queued / recent operator actions. |
| [Chaos](ops/chaos.md) | Chaos experiments against the virtualization stack. |
| [Disaster Recovery](ops/dr.md) | Disaster recovery plans for VMs and namespaces. |
| [Notifications](ops/notifications.md) | Notification channels and delivery preferences. |

## Platform

| Page | What it covers |
|------|----------------|
| [Template Foundry](platform/app-store.md) | Template Foundry — OS templates and app blueprints. |
| [Blueprint Studio](platform/blueprint-studio.md) | Multi-VM blueprint authoring and apply. |
| [Catalog](platform/catalog.md) | Workload and template catalog. |
| [CRDs](platform/crds.md) | CustomResourceDefinition catalog. |
| [Custom Resources](platform/custom-resources.md) | Browse and edit custom resources. |
| [GitOps](platform/gitops.md) | GitOps export and sync status for VeyronVM declarations. |
| [Helm](platform/helm.md) | Helm release inventory for the cluster. |
| [Integrations](platform/integrations.md) | External integrations (ticketing, chat, IdP). |
| [Operators](platform/operators.md) | Operator / OLM status relevant to KubeVirt and companions. |
| [Settings](platform/settings.md) | Dashboard, auth, and AI provider settings. |
| [Stack Health](platform/stack-health.md) | End-to-end stack health for Veyron + KubeVirt + CDI. |
| [Tenants](platform/tenants.md) | Multi-tenant boundaries and namespace mapping. |
| [Webhooks](platform/webhooks.md) | Webhook endpoints for automation. |

## Security

| Page | What it covers |
|------|----------------|
| [Audit](security/audit.md) | Audit trail of mutating Veyron and cluster actions. |
| [Compliance](security/compliance.md) | Compliance checks and evidence for the fleet. |
| [Policies](security/policies.md) | Admission / VM policy enforcement surfaces. |
| [Quotas](security/quotas.md) | ResourceQuota and LimitRange views. |
| [RBAC](security/rbac.md) | Kubernetes RBAC review for Veyron operators and tenants. |
| [Security Posture](security/security.md) | Security posture across VMs and namespaces. |
| [SOC](security/soc.md) | SOC-style security operations board. |

## Storage

| Page | What it covers |
|------|----------------|
| [Backups](storage/backups.md) | Backup jobs and restore entry points for VMs. |
| [Images](storage/images.md) | ContainerDisk / golden image catalog. |
| [Snapshots](storage/snapshots.md) | VolumeSnapshot / VM snapshot inventory and actions. |
| [Storage](storage/storage.md) | PVC, DataVolume, and storage class inventory for VMs. |

---

65 guides. Regenerate: `node scripts/customer-docs/generate-guide-index.mjs`.
