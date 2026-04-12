#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// HPA response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpaResponse {
    pub name: String,
    pub namespace: String,
    pub target_kind: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub current_replicas: u32,
    pub desired_replicas: u32,
    pub cpu_utilization_target: Option<u32>,
    pub cpu_utilization_current: Option<u32>,
    pub memory_utilization_target: Option<u32>,
    pub memory_utilization_current: Option<u32>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/hpa", get(list_hpa))
}

#[cfg(feature = "web")]
async fn list_hpa() -> Json<Vec<HpaResponse>> {
    Json(vec![])
}
