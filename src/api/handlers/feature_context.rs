//! Metadata for API payloads that are derived, bounded, or backed by a narrow source.
//! Attached as `vmrogue_context` so clients and the dashboard can explain what users are seeing.
//!
//! **Production parity (optional follow-ups, not required for correctness of current data):**
//! - Traces: optional OpenTelemetry/Jaeger query backend when configured.
//! - Logs: optional Loki/Elasticsearch URL + auth for cluster-wide log search.
//! - Incidents: optional Alertmanager webhook or PagerDuty-style state instead of time-heuristic resolution.
//! - Performance: optional histogram-backed percentiles when a metrics store retains history.
//! - GitOps: optional Argo CD / Flux API clients when credentials and base URLs are supplied.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmrogueFeatureContext {
    /// Stable machine-readable producer id (e.g. `kubernetes_events`, `virt_launcher_logs`).
    pub data_source: String,
    /// Short human-readable scope line for subtitles or badges.
    pub scope: String,
    /// Limitations: what this is not, and how to interpret numbers or rows.
    pub limitations: String,
}

impl VmrogueFeatureContext {
    pub fn traces() -> Self {
        Self::traces_fallback()
    }

    pub fn traces_fallback() -> Self {
        Self {
            data_source: "kubernetes_events".to_string(),
            scope: "Kubernetes Events in the selected namespace scope, shown as trace-like rows."
                .to_string(),
            limitations: "Not OpenTelemetry or Jaeger: configure VMROGUE_JAEGER_QUERY_URL for real traces. duration_ms is nominal when using Events fallback.".to_string(),
        }
    }

    pub fn logs_loki() -> Self {
        Self {
            data_source: "loki_query_range".to_string(),
            scope: "Log lines from Loki for the selected namespace scope.".to_string(),
            limitations: "Requires VMROGUE_LOKI_URL. Query uses LogQL namespace selector.".to_string(),
        }
    }

    pub fn incidents_alertmanager() -> Self {
        Self {
            data_source: "alertmanager_v2_alerts".to_string(),
            scope: "Active Alertmanager alerts mapped to incident rows.".to_string(),
            limitations: "Requires VMROGUE_ALERTMANAGER_URL. Falls back to Warning Events when unset.".to_string(),
        }
    }

    pub fn gitops_controllers() -> Self {
        Self {
            data_source: "argocd_flux_crd_status".to_string(),
            scope: "Argo CD Application and Flux Kustomization CR status in namespace.".to_string(),
            limitations: "POST /gitops/sync triggers Argo CD API when VMROGUE_ARGOCD_URL+TOKEN are set; otherwise updates ConfigMap only.".to_string(),
        }
    }

    pub fn logs_dashboard() -> Self {
        Self {
            data_source: "virt_launcher_pod_logs".to_string(),
            scope: "Logs from pods labeled kubevirt.io/domain (virt-launcher) in the selected scope."
                .to_string(),
            limitations: "Not cluster-wide Loki/Elasticsearch: no aggregated platform or audit log pipeline. Lines are parsed heuristically from pod log text; other workloads are out of scope unless they share the label."
                .to_string(),
        }
    }

    pub fn incidents() -> Self {
        Self {
            data_source: "kubernetes_warning_events_plus_slo_hint".to_string(),
            scope: "Warning Events in scope, plus an optional synthetic row when VM availability is below target."
                .to_string(),
            limitations: "Not Alertmanager, PagerDuty, or a ticketing system. \"resolved\" means the Warning event is older than one hour (time heuristic), not acknowledged in an external tool. Counts are not deduplicated across controllers."
                .to_string(),
        }
    }

    pub fn performance_profiles() -> Self {
        Self {
            data_source: "vm_metrics_snapshot_estimates".to_string(),
            scope: "Live VM metrics from VMRogue collectors, with percentile and latency fields derived from the current sample."
                .to_string(),
            limitations: "cpu_p95/p99 and memory_p95/p99 are scaled estimates from the instantaneous usage, not historical histograms. latency_avg_ms is a rough model from I/O counters when present, not block-storage latency percentiles."
                .to_string(),
        }
    }

    pub fn performance_flamegraph() -> Self {
        Self {
            data_source: "synthetic_flamegraph_from_cpu".to_string(),
            scope: "Illustrative flame-style breakdown from current guest CPU usage percent."
                .to_string(),
            limitations: "Not a sampled CPU profile (no perf/parca). Frame proportions use fixed KVM guest ratios for visualization only."
                .to_string(),
        }
    }

    pub fn heatmap() -> Self {
        Self {
            data_source: "node_capacity_allocatable_vmi_density".to_string(),
            scope: "Per-node CPU/memory/disk from capacity vs allocatable; VM count from VMIs; network column is a density heuristic."
                .to_string(),
            limitations: "CPU/memory/disk are not kubelet-reported utilization. network_utilization is estimated from VM count, not NIC metrics. heat_score is a weighted blend of those signals."
                .to_string(),
        }
    }

    pub fn gitops_status() -> Self {
        Self {
            data_source: "vmrogue_gitops_configmap_plus_vm_list".to_string(),
            scope: "Reads labeled GitOps ConfigMaps in the namespace and compares stored vm_count to live VM list length."
                .to_string(),
            limitations: "Does not call Argo CD, Flux, or other GitOps controller APIs. POST /gitops/sync only updates timestamps in the ConfigMap when present — it does not trigger reconciliation."
                .to_string(),
        }
    }

    pub fn storage_pools() -> Self {
        Self {
            data_source: "storage_classes_and_pvc_capacity_sum".to_string(),
            scope: "One row per StorageClass with summed PVC capacities and volume counts."
                .to_string(),
            limitations: "available_capacity is N/A without a CSI/metrics integration that exposes pool free space. used_capacity mirrors allocated PVC capacity, not filesystem usage inside volumes."
                .to_string(),
        }
    }

    pub fn storage_usage() -> Self {
        Self {
            data_source: "pvc_spec_status".to_string(),
            scope: "Per-PVC capacity and phase in the namespace scope."
                .to_string(),
            limitations: "Kubernetes does not expose filesystem-level usage on PVCs without metrics (kubelet volume stats / CSI). Bound PVCs show full capacity as used for allocation reporting."
                .to_string(),
        }
    }

    pub fn costs_opencost() -> Self {
        Self {
            data_source: "opencost_allocation".to_string(),
            scope: "Monthly costs from OpenCost allocation API joined to VMs via virt-launcher pods.".to_string(),
            limitations: "Requires VMROGUE_OPENCOST_URL and OpenCost installed. Falls back to static rates when unavailable.".to_string(),
        }
    }

    pub fn costs_list() -> Self {
        Self {
            data_source: "vm_spec_static_rates".to_string(),
            scope: "Monthly USD estimate per VM from CPU, memory, disk request, and a flat network line item."
                .to_string(),
            limitations: "Internal reference rates only — not cloud billing, invoiced costs, or OpenCost. Network is a flat per-VM estimate; storage uses requests or a default when missing."
                .to_string(),
        }
    }

    pub fn costs_summary() -> Self {
        Self {
            data_source: "aggregated_vm_spec_static_rates".to_string(),
            scope: "Roll-up of the same static-rate model as GET /costs."
                .to_string(),
            limitations: "See costs list disclaimer: not provider billing; by_namespace and by_resource_type are sums of the internal model."
                .to_string(),
        }
    }

    pub fn costs_forecast() -> Self {
        Self {
            data_source: "current_monthly_times_scalar".to_string(),
            scope: "Simple projection from summed current monthly estimate."
                .to_string(),
            limitations: "projected_monthly applies a fixed uplift to current totals — not predictive ML or reservation-aware forecasting."
                .to_string(),
        }
    }

    pub fn forecasting_predictions() -> Self {
        Self {
            data_source: "cluster_density_heuristic".to_string(),
            scope: "Capacity-style rows derived from VM/node counts and running ratios in scope."
                .to_string(),
            limitations: "Not a metrics-backed forecast: growth and breach hints are heuristics from density and running VM ratio, not time-series models."
                .to_string(),
        }
    }
}
