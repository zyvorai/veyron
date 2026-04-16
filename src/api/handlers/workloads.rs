#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Workload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResponse {
    pub name: String,
    pub namespace: String,
    pub workload_type: String,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub cpu_request: String,
    pub memory_request: String,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/workloads", get(list_workloads))
}

#[cfg(feature = "web")]
async fn list_workloads() -> impl IntoResponse {
    not_implemented("Workloads")
}
