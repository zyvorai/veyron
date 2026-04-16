#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Cilium status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumStatus {
    pub version: String,
    pub agent_count: u32,
    pub healthy_agents: u32,
    pub cluster_mesh_enabled: bool,
    pub hubble_enabled: bool,
    pub encryption_enabled: bool,
}

/// Cilium network policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiliumPolicy {
    pub name: String,
    pub namespace: String,
    pub enforcement: String,
    pub endpoint_selector: std::collections::HashMap<String, String>,
    pub ingress_rules: u32,
    pub egress_rules: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/cilium/status", get(get_cilium_status))
        .route("/cilium/policies", get(list_cilium_policies))
}

#[cfg(feature = "web")]
async fn get_cilium_status() -> impl IntoResponse {
    not_implemented("Cilium status")
}

#[cfg(feature = "web")]
async fn list_cilium_policies() -> impl IntoResponse {
    not_implemented("Cilium policies")
}
