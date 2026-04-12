#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Network policy response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyResponse {
    pub name: String,
    pub namespace: String,
    pub policy_types: Vec<String>,
    pub pod_selector: std::collections::HashMap<String, String>,
    pub ingress_rules: u32,
    pub egress_rules: u32,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/network-policies", get(list_network_policies))
}

#[cfg(feature = "web")]
async fn list_network_policies() -> Json<Vec<NetworkPolicyResponse>> {
    Json(vec![])
}
