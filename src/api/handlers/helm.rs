#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/helm/releases", get(list_helm_releases))
}

#[cfg(feature = "web")]
async fn list_helm_releases() -> Json<Vec<HelmReleaseResponse>> {
    Json(vec![])
}
