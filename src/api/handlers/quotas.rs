#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
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

#[derive(Debug, Clone, Deserialize)]
pub struct QuotaQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/quotas", get(list_quotas))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_quotas(
    State(state): State<SharedState>,
    Query(query): Query<QuotaQuery>,
) -> Json<Vec<QuotaResponse>> {
    use k8s_openapi::api::core::v1::ResourceQuota;

    let s = state.read().await;
    let client = s.client().client();
    let api: kube::api::Api<ResourceQuota> = match query.namespace.as_deref() {
        Some("all") => kube::api::Api::all(client),
        Some(ns) => kube::api::Api::namespaced(client, ns),
        None => kube::api::Api::namespaced(client, &s.namespace),
    };
    let quotas = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    // Pre-fetch VM counts per namespace (async, so cannot be done inside .map())
    let mut vm_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for q in &quotas.items {
        if let Some(ns) = q.metadata.namespace.as_deref() {
            if !vm_counts.contains_key(ns) {
                let count = s.client().list_vms(ns).await.unwrap_or_default().len() as u32;
                vm_counts.insert(ns.to_string(), count);
            }
        }
    }

    let results: Vec<QuotaResponse> = quotas
        .items
        .iter()
        .map(|q| {
            let hard = q.status.as_ref().and_then(|s| s.hard.as_ref());
            let used = q.status.as_ref().and_then(|s| s.used.as_ref());
            let ns = q.metadata.namespace.clone().unwrap_or_default();

            // KubeVirt quota key for VM count
            let vm_limit = hard
                .and_then(|h| {
                    h.get("count/virtualmachines.kubevirt.io")
                        .or_else(|| h.get("count/vms.kubevirt.io"))
                        .or_else(|| h.get("count/virtualmachineinstances.kubevirt.io"))
                })
                .and_then(|v| v.0.parse::<u32>().ok());

            let vm_count = vm_counts.get(&ns).copied().unwrap_or(0);

            QuotaResponse {
                name: q.metadata.name.clone().unwrap_or_default(),
                namespace: ns,
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
                vm_limit,
                vm_count,
            }
        })
        .collect();

    Json(results)
}
