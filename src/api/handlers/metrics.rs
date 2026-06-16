// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

/// Metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub vm_name: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub timestamp: String,
}

/// Cluster-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub total_vms: u32,
    pub running_vms: u32,
    pub total_cpu_cores: u32,
    pub used_cpu_cores: f64,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/metrics", get(get_cluster_metrics))
        .route("/metrics/timeline", get(get_metrics_timeline))
        .route("/metrics/{vm}", get(get_vm_metrics))
        .with_state(state)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTimelinePoint {
    pub timestamp: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTimelineResponse {
    pub veyron_context: super::feature_context::VmrogueFeatureContext,
    pub namespace: String,
    pub vm_name: String,
    pub metric: String,
    pub unit: String,
    pub points: Vec<MetricsTimelinePoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimelineQuery {
    pub namespace: Option<String>,
    pub vm: Option<String>,
    #[serde(default = "default_metric")]
    pub metric: String,
    #[serde(default = "default_hours")]
    pub hours: u32,
}

fn default_metric() -> String {
    "cpu".to_string()
}

fn default_hours() -> u32 {
    6
}

#[cfg(feature = "web")]
async fn get_cluster_metrics(State(state): State<SharedState>) -> Json<ClusterMetrics> {
    let s = state.read().await;
    let client = s.client();

    let vms = client.list_all_vms().await.unwrap_or_default();
    let nodes = client.list_nodes().await.unwrap_or_default();

    let total_vms = vms.len() as u32;
    let running_vms = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                .map(|s| s == "Running")
                .unwrap_or(false)
        })
        .count() as u32;

    let mut total_cpu: u32 = 0;
    let mut total_mem: u64 = 0;
    for node in &nodes {
        if let Some(cap) = node.status.as_ref().and_then(|s| s.capacity.as_ref()) {
            if let Some(cpu) = cap.get("cpu") {
                total_cpu += cpu.0.parse::<u32>().unwrap_or(0);
            }
            if let Some(mem) = cap.get("memory") {
                total_mem += parse_k8s_memory(&mem.0);
            }
        }
    }

    let used_cpu_cores: f64 = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                .map(|s| s == "Running")
                .unwrap_or(false)
        })
        .filter_map(|vm| {
            vm.spec
                .template
                .spec
                .domain
                .cpu
                .as_ref()
                .and_then(|c| c.cores)
                .map(|c| c as f64)
        })
        .sum();

    let used_memory_bytes: u64 = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                .map(|s| s == "Running")
                .unwrap_or(false)
        })
        .filter_map(|vm| {
            vm.spec
                .template
                .spec
                .domain
                .resources
                .requests
                .as_ref()
                .and_then(|r| r.get("memory"))
                .map(|v| parse_k8s_memory(v))
        })
        .sum();

    Json(ClusterMetrics {
        total_vms,
        running_vms,
        total_cpu_cores: total_cpu,
        used_cpu_cores,
        total_memory_bytes: total_mem,
        used_memory_bytes,
    })
}

#[cfg(feature = "web")]
async fn get_vm_metrics(
    State(state): State<SharedState>,
    Path(vm): Path<String>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Option<MetricsResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let metrics_ns = if namespace_scope::is_all_namespaces(&scope) {
        s.client()
            .list_vms_for_scope("all")
            .await
            .into_iter()
            .find(|v| v.metadata.name.as_deref() == Some(vm.as_str()))
            .and_then(|v| v.metadata.namespace)
            .unwrap_or_else(|| s.namespace.clone())
    } else {
        scope
    };
    let collector = crate::monitoring::metrics::MetricsCollector::new(metrics_ns);

    match collector.collect(&vm).await {
        Ok(m) => Json(Some(MetricsResponse {
            vm_name: vm,
            cpu_usage_percent: m.cpu.usage_percent,
            memory_usage_percent: m.memory.usage_percent,
            disk_read_bytes: m.disk.read_bytes_per_sec,
            disk_write_bytes: m.disk.write_bytes_per_sec,
            network_rx_bytes: m.network.rx_bytes_per_sec,
            network_tx_bytes: m.network.tx_bytes_per_sec,
            timestamp: m.timestamp.to_rfc3339(),
        })),
        Err(_) => Json(None),
    }
}

#[cfg(feature = "web")]
async fn get_metrics_timeline(
    State(state): State<SharedState>,
    Query(q): Query<TimelineQuery>,
) -> Json<MetricsTimelineResponse> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let vm_name = q.vm.clone().unwrap_or_default();
    let hours = q.hours.clamp(1, 72);
    let metric = if q.metric.is_empty() {
        "cpu".to_string()
    } else {
        q.metric.clone()
    };

    let ctx = super::feature_context::VmrogueFeatureContext {
        data_source: "prometheus_range_query".to_string(),
        scope: format!("Prometheus range series for VM {vm_name} ({metric}) over {hours}h."),
        limitations: "Requires VEYRON_PROMETHEUS_URL. Uses kubevirt_vmi_* metrics when present."
            .to_string(),
    };

    let mut points = Vec::new();
    let unit = if metric == "memory" {
        "bytes".to_string()
    } else {
        "percent".to_string()
    };

    if let Ok(prom_base) = std::env::var("VEYRON_PROMETHEUS_URL") {
        if !vm_name.is_empty() && !namespace_scope::is_all_namespaces(&scope) {
            let end = chrono::Utc::now().timestamp();
            let start = end - (hours as i64 * 3600);
            let prom_query = match metric.as_str() {
                "memory" => format!(
                    "kubevirt_vmi_memory_resident_bytes{{namespace=\"{scope}\", name=\"{vm_name}\"}}"
                ),
                _ => format!(
                    "rate(kubevirt_vmi_vcpu_seconds{{namespace=\"{scope}\", name=\"{vm_name}\"}}[5m]) * 100"
                ),
            };
            if let Ok(series) =
                crate::api::prometheus::range_query_series(&prom_base, &prom_query, start, end, 300)
                    .await
            {
                if let Some((_, vals)) = series.into_iter().next() {
                    points = vals
                        .into_iter()
                        .map(|(ts, v)| MetricsTimelinePoint {
                            timestamp: ts,
                            value: v,
                        })
                        .collect();
                }
            }
        }
    }

    Json(MetricsTimelineResponse {
        veyron_context: ctx,
        namespace: scope,
        vm_name,
        metric,
        unit,
        points,
    })
}

fn parse_k8s_memory(s: &str) -> u64 {
    crate::utils::parse_memory_bytes(s)
}
