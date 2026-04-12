#[cfg(feature = "web")]
use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};

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
async fn get_gitops_status() -> Json<GitOpsStatus> {
    Json(GitOpsStatus {
        repo_url: String::new(),
        branch: "main".to_string(),
        last_commit: String::new(),
        sync_status: "unknown".to_string(),
        last_synced: None,
        drift_detected: false,
    })
}

#[cfg(feature = "web")]
async fn trigger_sync(Json(req): Json<GitOpsSyncRequest>) -> Json<serde_json::Value> {
    let _ = req;
    Json(serde_json::json!({"status": "sync_triggered"}))
}
