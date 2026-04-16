#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Scheduling status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingStatus {
    pub pending_pods: u32,
    pub scheduled_pods: u32,
    pub unschedulable_nodes: Vec<String>,
    pub scheduling_latency_ms: f64,
    pub preemptions: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/scheduling/status", get(get_scheduling_status))
}

#[cfg(feature = "web")]
async fn get_scheduling_status() -> impl IntoResponse {
    not_implemented("Scheduling status")
}
