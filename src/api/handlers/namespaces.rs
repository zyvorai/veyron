// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Namespace response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceResponse {
    pub name: String,
    pub status: String,
    pub vm_count: u32,
    pub labels: std::collections::HashMap<String, String>,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/namespaces", get(list_namespaces))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_namespaces(State(state): State<SharedState>) -> Json<Vec<NamespaceResponse>> {
    let kube_client = { let s = state.read().await; s.kube_client.clone() };
    let namespaces = kube_client.list_namespaces().await.unwrap_or_default();

    // Count VMs per namespace with a single cluster-wide API call
    let mut vm_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    if let Ok(all_vms) = kube_client.list_all_vms().await {
        for vm in &all_vms {
            let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
            *vm_counts.entry(ns.to_string()).or_insert(0) += 1;
        }
    }

    let results: Vec<NamespaceResponse> = namespaces
        .iter()
        .map(|ns| {
            let meta = &ns.metadata;
            let name = meta.name.clone().unwrap_or_default();
            NamespaceResponse {
                vm_count: vm_counts.get(&name).copied().unwrap_or(0),
                name,
                status: ns
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Active")
                    .to_string(),
                created_at: meta
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                labels: meta
                    .labels
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            }
        })
        .collect();

    Json(results)
}
