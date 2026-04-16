#[cfg(feature = "web")]
use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// GitOps status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsStatus {
    pub repo_url: String,
    pub branch: String,
    pub last_commit: String,
    pub sync_status: String,
    pub last_synced: Option<String>,
    pub drift_detected: bool,
}

/// GitOps sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsSyncRequest {
    pub force: bool,
    pub dry_run: bool,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/gitops/status", get(get_gitops_status))
        .route("/gitops/sync", post(trigger_sync))
}

#[cfg(feature = "web")]
async fn get_gitops_status() -> impl IntoResponse {
    not_implemented("GitOps status")
}

#[cfg(feature = "web")]
async fn trigger_sync(axum::Json(req): axum::Json<GitOpsSyncRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("GitOps sync")
}
