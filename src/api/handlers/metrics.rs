#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Path, State}, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
        .route("/metrics/{vm}", get(get_vm_metrics))
        .with_state(state)
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
            vm.spec.template.spec.domain.cpu
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
            vm.spec.template.spec.domain.resources.requests
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
) -> Json<Option<MetricsResponse>> {
    let s = state.read().await;
    let collector = crate::monitoring::metrics::MetricsCollector::new(s.namespace.clone());

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

fn parse_k8s_memory(s: &str) -> u64 {
    crate::utils::parse_memory_bytes(s)
}
