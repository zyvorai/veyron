#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Operator response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResponse {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub status: String,
    pub managed_resources: Vec<String>,
    pub installed_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/operators", get(list_operators))
        .with_state(state)
}

/// Discover operators by listing Deployments with common operator name patterns.
#[cfg(feature = "web")]
async fn list_operators(State(state): State<SharedState>) -> Json<Vec<OperatorResponse>> {
    use k8s_openapi::api::apps::v1::Deployment;

    let s = state.read().await;
    let client = s.client().client();
    let mut results = Vec::new();

    // Check common operator namespaces
    let namespaces = ["kubevirt", "olm", "operators", "vmrogue-system", &s.namespace];

    for ns in &namespaces {
        let api: kube::api::Api<Deployment> = kube::api::Api::namespaced(client.clone(), ns);
        if let Ok(deploys) = api.list(&kube::api::ListParams::default()).await {
            for d in &deploys.items {
                let name = d.metadata.name.as_deref().unwrap_or("");
                // Heuristic: operators typically have "operator", "controller", or "manager" in name
                if name.contains("operator") || name.contains("controller") || name.contains("manager") {
                    let ready = d
                        .status
                        .as_ref()
                        .and_then(|s| s.ready_replicas)
                        .unwrap_or(0);
                    let desired = d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);

                    results.push(OperatorResponse {
                        name: name.to_string(),
                        namespace: ns.to_string(),
                        version: d
                            .metadata
                            .labels
                            .as_ref()
                            .and_then(|l| l.get("app.kubernetes.io/version").cloned())
                            .unwrap_or_default(),
                        status: if ready >= desired {
                            "Running".to_string()
                        } else {
                            "Degraded".to_string()
                        },
                        managed_resources: Vec::new(),
                        installed_at: d
                            .metadata
                            .creation_timestamp
                            .as_ref()
                            .map(|t| t.0.to_rfc3339())
                            .unwrap_or_default(),
                    });
                }
            }
        }
    }

    Json(results)
}
