# VMRogue Feature Matrix

**See also:** [Documentation index](README.md) for the full set of guides.

This matrix maps the current dashboard surface to its backend routes and implementation status.

Status legend:
- `Working`: page exists in the navbar and is backed by implemented routes
- `Partial`: page exists but some fields are heuristic, incomplete, or only partially populated
- `Missing UI`: backend route exists but no first-class dashboard page exposes it
- `Missing Backend`: UI references behavior that is not fully implemented server-side

## Dashboard Pages

| Page | Frontend Page ID | Primary Routes | Backend Source | Status | Notes |
|---|---|---|---|---|---|
| Dashboard | `page-dashboard` | `/api/v1/vms`, `/api/v1/events`, `/api/v1/dashboard/overview` | `http_server.rs` | Working | Overview cards and event feed are live |
| VMs | `page-vms` | `/api/v1/vms`, `/api/v1/vms/:ns/:name`, VM actions, `/rdp-expose`, `/guest-agent/*-rdp` | `http_server.rs` | Working | Main lifecycle page; RDP panel in embedded SPA and dashboard-next `VMBrowser` |
| Snapshots | `page-snapshots` | `/api/v1/snapshots`, create/delete/restore routes | `http_server.rs` | Working | Uses direct handlers in `http_server.rs` |
| Nodes | `page-nodes` | `/api/v1/nodes` | `http_server.rs`, `handlers/nodes.rs` | Working | Real allocatable/capacity data |
| Pods | `page-pods` | `/api/v1/pods` | `http_server.rs`, `handlers/pods.rs` | Working | Real pod listing |
| Storage | `page-storage` | `/api/v1/storage/pvcs`, `/api/v1/storage/classes` | `http_server.rs` | Working | Separate richer storage routes also exist |
| Events | `page-events` | `/api/v1/events` | `http_server.rs`, `handlers/events.rs` | Working | Event feed is live |
| CRDs | `page-crds` | `/api/v1/crds/vmroguevms` | `handlers/crds.rs` | Working | Operator CRD listing and create flow |
| Policies | `page-policies` | `/api/v1/crds/policies` | `handlers/crds.rs` | Working | Uses operator CRDs |
| Insights | `page-insights` | `/api/v1/crds/insights` | `handlers/crds.rs` | Working | Uses operator CRDs |
| Actions | `page-actions` | `/api/v1/crds/actions` | `handlers/crds.rs` | Working | Uses operator CRDs |
| Costs | `page-costs` | `/api/v1/costs`, `/summary`, `/forecast` | `handlers/costs.rs` | Partial | Summary JSON includes `pricing_model`, `disclaimer`; not billing-integrated |
| Security | `page-security` | `/api/v1/security/posture`, `/findings` | `handlers/security.rs` | Partial | Rule set is basic VM config analysis |
| Monitoring | `page-monitoring` | `/api/v1/monitoring/status`, `/api/v1/recommendations` | `handlers/monitoring.rs`, `handlers/recommendations.rs` | Partial | Stack detection is heuristic, alert count now real |
| Workloads | `page-workloads` | `/api/v1/workloads` | `handlers/workloads.rs` | Working | CPU/memory request aggregation now populated |
| Alerts | `page-alerts` | `/api/v1/alerts` | `handlers/alerts.rs` | Working | Namespace-aware |
| Audit | `page-audit` | `/api/v1/audit/trail`, `/api/v1/audit/stats` | `handlers/audit.rs` | Working | Namespace-aware |
| Notifications | `page-notifications` | `/api/v1/notifications` | `handlers/notifications.rs` | Working | Namespace-aware |
| Helm | `page-helm` | `/api/v1/helm/releases` | `handlers/helm.rs` | Working | Client-side namespace filtering |
| Operators | `page-operators` | `/api/v1/operators` | `handlers/operators.rs` | Working | Client-side namespace filtering |
| Custom Resources | `page-custom-resources` | `/api/v1/custom-resources` | `handlers/custom_resources.rs` | Working | Cluster-scoped CRD inventory |
| SLO | `page-slo` | `/api/v1/slo/objectives` | `handlers/slo.rs` | Working | Namespace-aware |
| Chaos | `page-chaos` | `/api/v1/chaos/experiments` | `handlers/chaos_engineering.rs` | Working | Client-side namespace filtering |
| RBAC | `page-rbac` | `/api/v1/rbac/roles`, `/api/v1/rbac/bindings` | `handlers/rbac.rs` | Working | Cluster-scoped |
| Quotas | `page-quotas` | `/api/v1/quotas` | `handlers/quotas.rs` | Working | Namespace-aware |
| Ingress | `page-ingress` | `/api/v1/ingress` | `handlers/ingress.rs` | Working | Namespace-aware |
| HPA | `page-hpa` | `/api/v1/hpa` | `handlers/hpa.rs` | Working | Namespace-aware |
| Backups | `page-backups` | `/api/v1/backups` | `handlers/backups.rs` | Working | Namespace-aware listing |
| Metrics | `page-metrics` | `/api/v1/metrics` | `handlers/metrics.rs` | Working | Cluster summary view |
| Topology | `page-topology` | `/api/v1/topology/map` | `handlers/topology.rs` | Working | Namespace-aware VM graph |
| Dependencies | `page-dependencies` | `/api/v1/dependencies/graph` | `handlers/dependencies.rs` | Working | Client-side namespace filtering |
| Autoscaler | `page-autoscaler` | `/api/v1/autoscaler/policies` | `handlers/autoscaler.rs` | Working | Namespace-aware |
| Forecasting | `page-forecasting` | `/api/v1/forecasting/predictions` | `handlers/forecasting.rs` | Partial | Forecasts are heuristic projections |
| GitOps | `page-gitops` | `/api/v1/gitops/status`, `/api/v1/gitops/sync` | `handlers/gitops.rs` | Partial | Real config detection, sync action is advisory |
| Scheduling | `page-scheduling` | `/api/v1/scheduling/status` | `handlers/scheduling.rs` | Partial | Latency/preemption fields still basic |
| Cilium | `page-cilium` | `/api/v1/cilium/status`, `/api/v1/cilium/policies` | `handlers/cilium.rs` | Partial | Falls back to NetworkPolicies |
| Observability | `page-observability` | `/api/v1/observability/overview` | `handlers/observability.rs` | Partial | Stack detection and counts are heuristic |
| Performance | `page-performance` | `/api/v1/performance/profiles` | `handlers/performance.rs` | Partial | Percentiles are single-sample proxies |
| Webhooks | `page-webhooks` | `/api/v1/webhooks` | `handlers/webhooks.rs` | Working | Namespace-aware |

## Implemented Backend Routes Surfaced In Dashboard

These route groups now have first-class dashboard pages:

| Route Group | Example Routes | Status |
|---|---|---|
| Alerts | `/api/v1/alerts` | Surfaced |
| Audit | `/api/v1/audit/trail`, `/api/v1/audit/stats` | Surfaced |
| Notifications | `/api/v1/notifications` | Surfaced |
| Helm | `/api/v1/helm/releases` | Surfaced |
| Operators | `/api/v1/operators` | Surfaced |
| Custom Resources | `/api/v1/custom-resources` | Surfaced |
| SLO | `/api/v1/slo/objectives` | Surfaced |
| Chaos | `/api/v1/chaos/experiments` | Surfaced |
| RBAC | `/api/v1/rbac/roles`, `/api/v1/rbac/bindings` | Surfaced |
| Quotas | `/api/v1/quotas` | Surfaced |
| Ingress | `/api/v1/ingress` | Surfaced |
| HPA | `/api/v1/hpa` | Surfaced |
| Backups | `/api/v1/backups` | Surfaced |
| Metrics | `/api/v1/metrics`, `/api/v1/metrics/:vm` | Surfaced |
| Topology | `/api/v1/topology/map` | Surfaced |
| Dependencies | `/api/v1/dependencies/graph` | Surfaced |
| Autoscaler | `/api/v1/autoscaler/policies` | Surfaced |
| Forecasting | `/api/v1/forecasting/predictions` | Surfaced |
| GitOps | `/api/v1/gitops/status`, `/api/v1/gitops/sync` | Surfaced |
| Scheduling | `/api/v1/scheduling/status` | Surfaced |
| Cilium | `/api/v1/cilium/status`, `/api/v1/cilium/policies` | Surfaced |
| Observability | `/api/v1/observability/overview` | Surfaced |
| Performance | `/api/v1/performance/profiles` | Surfaced |
| Webhooks | `/api/v1/webhooks` | Surfaced |

## Known Partial Areas

| Area | Current Limitation |
|---|---|
| Costs | Uses estimated formulas, not provider billing data |
| Security | VM posture is configuration-derived, not vulnerability-scanner-backed |
| Monitoring | Prometheus/Grafana detection is service-name based |
| Operators | Managed resources are inferred heuristically from CRDs |
| Helm | Chart metadata depends on labels/annotations present in release secrets |
| Chaos | Experiment parsing is generic because chaos CRDs vary by platform |
| Forecasting | Predictions are derived from simple growth assumptions |
| GitOps | Sync status is config-derived; sync action does not drive ArgoCD/Flux directly |
| Scheduling | Scheduling latency and preemptions are not yet measured from scheduler metrics |
| Cilium | Policy view falls back to standard NetworkPolicy rather than native Cilium CRDs |
| Observability | Counts and ingestion/storage values are approximations |
| Performance | Percentiles are based on the latest metric sample, not historical windows |

## Operator UI (`/dashboard-next/`)

React operator console (HyperShell layout). Use the classic SPA at `/dashboard` for full platform pages.

| Capability | Classic `/dashboard` | Operator UI `/dashboard-next/` |
|---|---|---|
| Login + API key session | Partial (localStorage) | Working (sessionStorage) |
| Fleet metrics dashboard | Working | Working (REST/WS feed) |
| VM list + lifecycle | Working | Working |
| VNC console | Working | Working |
| Serial console | Working | Working |
| Create / delete VM | Working | Working |
| Pause / unpause / migrate | Working | Working |
| Clone / resize VM | Working | Working |
| Bulk start / stop / delete | Working | Working |
| Snapshots (per VM) | Working | Working |
| SSH expose + internet egress | Working | Working |
| RDP + data disk | Working | Working |
| Cluster nodes (read-only) | Working | Working |
| Storage PVCs + classes (read-only) | Working | Working |
| GitOps, CRDs, … | Working | Link to classic SPA |
| Help → About (zyvor.dev) | Working | Working (login + shell) |

**Strategy:** Operator UI targets day-one KubeVirt fleet ops; classic dashboard remains the full platform surface until pages are ported incrementally.

### Operator UI — remaining (optional)

| Item | Priority | Notes |
|------|----------|-------|
| Port platform pages (GitOps, CRDs) | Low | Nodes + storage in operator UI; classic for the rest |
| Cross-product Zyvor login kit sync | Low | Run `scripts/zyvor-branding/apply-zyvor-premium-login.sh` after shell/CSS changes |
| Default landing on `/dashboard-next/` | Done | `GET /` permanent redirect to operator UI |
| Vite bundle splitting / size budget | Low | Lazy routes already split Dashboard vs inventory |
| E2E against live cluster | Medium | Vitest covers auth/nav; integration needs KubeVirt |

## Immediate Next Steps

1. Deploy embedded `/dashboard-next/` bundle (nodes, storage, root redirect).
2. Add smoke coverage for remaining dashboard-facing API endpoints (backend/integration).
3. Tighten remaining heuristic pages such as `costs`, `security`, and `monitoring` in the classic SPA.
