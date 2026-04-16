#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/autoscaler/policies",
            get(list_policies).post(create_policy),
        )
        .with_state(state)
}

/// List autoscaler policies by reading HPAs in the namespace.
#[cfg(feature = "web")]
async fn list_policies(State(state): State<SharedState>) -> Json<Vec<AutoscalerPolicy>> {
    use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;

    let s = state.read().await;
    let api: kube::api::Api<HorizontalPodAutoscaler> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let hpas = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<AutoscalerPolicy> = hpas
        .items
        .iter()
        .map(|hpa| {
            let spec = hpa.spec.as_ref();
            let status = hpa.status.as_ref();

            AutoscalerPolicy {
                name: hpa.metadata.name.clone().unwrap_or_default(),
                namespace: hpa.metadata.namespace.clone().unwrap_or_default(),
                target_kind: spec
                    .map(|s| s.scale_target_ref.kind.clone())
                    .unwrap_or_default(),
                target_name: spec
                    .map(|s| s.scale_target_ref.name.clone())
                    .unwrap_or_default(),
                min_replicas: spec.and_then(|s| s.min_replicas).unwrap_or(1) as u32,
                max_replicas: spec.map(|s| s.max_replicas).unwrap_or(1) as u32,
                current_replicas: status
                    .and_then(|s| s.current_replicas)
                    .unwrap_or(0) as u32,
                cpu_threshold: None,
                memory_threshold: None,
                enabled: true,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn create_policy(
    State(_state): State<SharedState>,
    Json(_req): Json<CreateAutoscalerPolicyRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    // HPA creation requires building a full HPA spec — return guidance for now
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "NOT_IMPLEMENTED",
            "message": "Autoscaler policy creation via API is not yet supported. Use kubectl to create HPAs.",
        })),
    )
}
