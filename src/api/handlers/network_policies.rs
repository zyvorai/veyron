// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/network-policies", get(list_network_policies))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_network_policies(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<NetworkPolicyResponse>> {
    use k8s_openapi::api::networking::v1::NetworkPolicy;

    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let policies = match namespace_scope::list_namespaced_resource::<NetworkPolicy>(
        &s.client().client(),
        &scope,
    )
    .await
    {
        Ok(items) => items,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<NetworkPolicyResponse> = policies
        .iter()
        .map(|np| {
            let spec = np.spec.as_ref();

            let policy_types = spec
                .and_then(|s| s.policy_types.clone())
                .unwrap_or_default();

            let pod_selector: std::collections::HashMap<String, String> = spec
                .and_then(|s| s.pod_selector.match_labels.clone())
                .unwrap_or_default()
                .into_iter()
                .collect();

            let ingress_rules = spec
                .and_then(|s| s.ingress.as_ref())
                .map(|r| r.len() as u32)
                .unwrap_or(0);

            let egress_rules = spec
                .and_then(|s| s.egress.as_ref())
                .map(|r| r.len() as u32)
                .unwrap_or(0);

            NetworkPolicyResponse {
                name: np.metadata.name.clone().unwrap_or_default(),
                namespace: np.metadata.namespace.clone().unwrap_or_default(),
                policy_types,
                pod_selector,
                ingress_rules,
                egress_rules,
                created_at: np
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Json(results)
}
