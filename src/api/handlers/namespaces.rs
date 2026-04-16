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
    let s = state.read().await;
    let namespaces = s.client().list_namespaces().await.unwrap_or_default();

    let results: Vec<NamespaceResponse> = namespaces
        .iter()
        .map(|ns| {
            let meta = &ns.metadata;
            NamespaceResponse {
                name: meta.name.clone().unwrap_or_default(),
                status: ns
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Active")
                    .to_string(),
                vm_count: 0,
                created_at: meta
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                labels: meta.labels.clone().unwrap_or_default().into_iter().collect(),
            }
        })
        .collect();

    Json(results)
}
