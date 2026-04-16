#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Custom resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomResourceResponse {
    pub name: String,
    pub group: String,
    pub version: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub scope: String,
    pub instance_count: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/custom-resources", get(list_custom_resources))
        .with_state(state)
}

/// Discover CRDs installed in the cluster via the apiextensions API.
#[cfg(feature = "web")]
async fn list_custom_resources(State(state): State<SharedState>) -> Json<Vec<CustomResourceResponse>> {
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

    let s = state.read().await;
    let api: kube::api::Api<CustomResourceDefinition> = kube::api::Api::all(s.client().client());

    let crds = match api.list(&kube::api::ListParams::default()).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<CustomResourceResponse> = crds
        .items
        .iter()
        .map(|crd| {
            let spec = &crd.spec;
            let version = spec
                .versions
                .first()
                .map(|v| v.name.clone())
                .unwrap_or_default();

            CustomResourceResponse {
                name: crd.metadata.name.clone().unwrap_or_default(),
                group: spec.group.clone(),
                version,
                kind: spec.names.kind.clone(),
                namespace: None,
                scope: spec.scope.clone(),
                instance_count: 0,
            }
        })
        .collect();

    Json(results)
}
