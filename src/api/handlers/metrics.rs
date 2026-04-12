#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/metrics", get(get_cluster_metrics))
        .route("/metrics/{vm}", get(get_vm_metrics))
}

#[cfg(feature = "web")]
async fn get_cluster_metrics() -> Json<ClusterMetrics> {
    Json(ClusterMetrics {
        total_vms: 0,
        running_vms: 0,
        total_cpu_cores: 0,
        used_cpu_cores: 0.0,
        total_memory_bytes: 0,
        used_memory_bytes: 0,
    })
}

#[cfg(feature = "web")]
async fn get_vm_metrics(Path(vm): Path<String>) -> Json<Option<MetricsResponse>> {
    let _ = vm;
    Json(None)
}
