#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Helm release response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmReleaseResponse {
    pub name: String,
    pub namespace: String,
    pub chart: String,
    pub chart_version: String,
    pub app_version: String,
    pub status: String,
    pub revision: u32,
    pub updated_at: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/helm/releases", get(list_helm_releases))
        .with_state(state)
}

/// Discover Helm releases by listing Secrets labeled with `owner=helm`.
#[cfg(feature = "web")]
async fn list_helm_releases(State(state): State<SharedState>) -> Json<Vec<HelmReleaseResponse>> {
    use k8s_openapi::api::core::v1::Secret;

    let s = state.read().await;
    let api: kube::api::Api<Secret> = kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let params = kube::api::ListParams::default().labels("owner=helm");
    let secrets = match api.list(&params).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let mut releases: std::collections::HashMap<String, HelmReleaseResponse> =
        std::collections::HashMap::new();

    for secret in &secrets.items {
        let labels = secret.metadata.labels.as_ref();
        let name = labels
            .and_then(|l| l.get("name"))
            .cloned()
            .unwrap_or_default();
        let status = labels
            .and_then(|l| l.get("status"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let version: u32 = labels
            .and_then(|l| l.get("version"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        // Keep only the latest revision per release name
        let entry = releases.entry(name.clone()).or_insert_with(|| {
            HelmReleaseResponse {
                name: name.clone(),
                namespace: secret.metadata.namespace.clone().unwrap_or_default(),
                chart: String::new(),
                chart_version: String::new(),
                app_version: String::new(),
                status: status.clone(),
                revision: 0,
                updated_at: secret
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            }
        });

        if version > entry.revision {
            entry.revision = version;
            entry.status = status;
            entry.updated_at = secret
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
                .unwrap_or_default();
        }
    }

    Json(releases.into_values().collect())
}
