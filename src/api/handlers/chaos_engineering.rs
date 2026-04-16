#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Chaos experiment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub name: String,
    pub experiment_type: String,
    pub target: String,
    pub namespace: String,
    pub status: String,
    pub duration: String,
    pub results: Option<ChaosResults>,
    pub created_at: String,
}

/// Chaos experiment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResults {
    pub success: bool,
    pub affected_resources: u32,
    pub recovery_time_seconds: Option<u64>,
    pub observations: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/chaos/experiments", get(list_experiments))
        .with_state(state)
}

/// List chaos experiments by checking for Chaos Mesh or LitmusChaos CRDs.
#[cfg(feature = "web")]
async fn list_experiments(State(state): State<SharedState>) -> Json<Vec<ChaosExperiment>> {
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

    let s = state.read().await;
    let api: kube::api::Api<CustomResourceDefinition> = kube::api::Api::all(s.client().client());

    // Check if any chaos engineering CRDs are installed
    let mut chaos_installed = false;
    if let Ok(crds) = api.list(&kube::api::ListParams::default()).await {
        for crd in &crds.items {
            let name = crd.metadata.name.as_deref().unwrap_or("");
            if name.contains("chaos-mesh") || name.contains("litmuschaos") {
                chaos_installed = true;
                break;
            }
        }
    }

    if !chaos_installed {
        return Json(vec![]);
    }

    // If chaos CRDs exist, we'd list experiments here
    // For now return empty since we can't query unknown CRD schemas
    Json(vec![])
}
