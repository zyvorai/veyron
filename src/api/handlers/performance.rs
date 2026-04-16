#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/performance/profiles", get(list_performance_profiles))
        .with_state(state)
}

/// Collect basic performance profiles from live VM metrics.
#[cfg(feature = "web")]
async fn list_performance_profiles(
    State(state): State<SharedState>,
) -> Json<Vec<PerformanceProfile>> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    let mut profiles = Vec::new();

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let collector = crate::monitoring::metrics::MetricsCollector::new(
            s.namespace.clone(),
        );

        if let Ok(metrics) = collector.collect(vm_name).await {
            // Single-point metrics — percentiles require historical data collection.
            // Current values are used as the latest observation for all percentile fields.
            let cpu = metrics.cpu.usage_percent;
            let mem = metrics.memory.usage_percent;
            profiles.push(PerformanceProfile {
                name: format!("profile-{}", vm_name),
                vm_name: vm_name.to_string(),
                cpu_p50: cpu,
                cpu_p95: cpu,
                cpu_p99: cpu,
                memory_p50: mem,
                memory_p95: mem,
                memory_p99: mem,
                iops_read: metrics.disk.read_ops_per_sec,
                iops_write: metrics.disk.write_ops_per_sec,
                latency_avg_ms: 0.0,
                collected_at: now.clone(),
            });
        }
    }

    Json(profiles)
}
