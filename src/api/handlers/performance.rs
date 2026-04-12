#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Performance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub name: String,
    pub vm_name: String,
    pub cpu_p50: f64,
    pub cpu_p95: f64,
    pub cpu_p99: f64,
    pub memory_p50: f64,
    pub memory_p95: f64,
    pub memory_p99: f64,
    pub iops_read: u64,
    pub iops_write: u64,
    pub latency_avg_ms: f64,
    pub collected_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/performance/profiles", get(list_performance_profiles))
}

#[cfg(feature = "web")]
async fn list_performance_profiles() -> Json<Vec<PerformanceProfile>> {
    Json(vec![])
}
