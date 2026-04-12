#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
    Router::new()
        .route("/autoscaler/policies", get(list_policies).post(create_policy))
}

#[cfg(feature = "web")]
async fn list_policies() -> Json<Vec<AutoscalerPolicy>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_policy(Json(req): Json<CreateAutoscalerPolicyRequest>) -> Json<Option<AutoscalerPolicy>> {
    let _ = req;
    Json(None)
}
