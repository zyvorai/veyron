#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
    Router::new()
        .route("/scheduling/status", get(get_scheduling_status))
}

#[cfg(feature = "web")]
async fn get_scheduling_status() -> Json<SchedulingStatus> {
    Json(SchedulingStatus {
        pending_pods: 0,
        scheduled_pods: 0,
        unschedulable_nodes: vec![],
        scheduling_latency_ms: 0.0,
        preemptions: 0,
    })
}
