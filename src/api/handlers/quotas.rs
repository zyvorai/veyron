#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Quota response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaResponse {
    pub name: String,
    pub namespace: String,
    pub cpu_limit: String,
    pub cpu_used: String,
    pub memory_limit: String,
    pub memory_used: String,
    pub vm_limit: Option<u32>,
    pub vm_count: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/quotas", get(list_quotas))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_quotas(State(state): State<SharedState>) -> Json<Vec<QuotaResponse>> {
    use k8s_openapi::api::core::v1::ResourceQuota;

    let s = state.read().await;
    let api: kube::api::Api<ResourceQuota> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let quotas = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<QuotaResponse> = quotas
        .items
        .iter()
        .map(|q| {
            let hard = q.status.as_ref().and_then(|s| s.hard.as_ref());
            let used = q.status.as_ref().and_then(|s| s.used.as_ref());

            QuotaResponse {
                name: q.metadata.name.clone().unwrap_or_default(),
                namespace: q.metadata.namespace.clone().unwrap_or_default(),
                cpu_limit: hard
                    .and_then(|h| h.get("limits.cpu").or_else(|| h.get("cpu")))
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
                cpu_used: used
                    .and_then(|u| u.get("limits.cpu").or_else(|| u.get("cpu")))
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
                memory_limit: hard
                    .and_then(|h| h.get("limits.memory").or_else(|| h.get("memory")))
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
                memory_used: used
                    .and_then(|u| u.get("limits.memory").or_else(|| u.get("memory")))
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
                vm_limit: None,
                vm_count: 0,
            }
        })
        .collect();

    Json(results)
}
