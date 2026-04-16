#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Autoscaler policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoscalerPolicy {
    pub name: String,
    pub namespace: String,
    pub target_kind: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub current_replicas: u32,
    pub cpu_threshold: Option<u8>,
    pub memory_threshold: Option<u8>,
    pub enabled: bool,
}

/// Create autoscaler policy request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAutoscalerPolicyRequest {
    pub name: String,
    pub target_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub cpu_threshold: Option<u8>,
    pub memory_threshold: Option<u8>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route(
        "/autoscaler/policies",
        get(list_policies).post(create_policy),
    )
}

#[cfg(feature = "web")]
async fn list_policies() -> impl IntoResponse {
    not_implemented("Autoscaler policies")
}

#[cfg(feature = "web")]
async fn create_policy(
    axum::Json(req): axum::Json<CreateAutoscalerPolicyRequest>,
) -> impl IntoResponse {
    let _ = req;
    not_implemented("Autoscaler policy creation")
}
