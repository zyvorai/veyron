#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/scheduling/status", get(get_scheduling_status))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_scheduling_status(State(state): State<SharedState>) -> Json<SchedulingStatus> {
    let s = state.read().await;

    let pods = s.client().list_pods(&s.namespace).await.unwrap_or_default();
    let nodes = s.client().list_nodes().await.unwrap_or_default();

    let pending_pods = pods
        .iter()
        .filter(|p| {
            p.status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("")
                == "Pending"
        })
        .count() as u32;

    let scheduled_pods = pods
        .iter()
        .filter(|p| p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_some())
        .count() as u32;

    let unschedulable_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| {
            n.spec
                .as_ref()
                .and_then(|s| s.unschedulable)
                .unwrap_or(false)
        })
        .filter_map(|n| n.metadata.name.clone())
        .collect();

    Json(SchedulingStatus {
        pending_pods,
        scheduled_pods,
        unschedulable_nodes,
        scheduling_latency_ms: 0.0,
        preemptions: 0,
    })
}
