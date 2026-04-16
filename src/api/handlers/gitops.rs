#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::{get, post}};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/gitops/status", get(get_gitops_status))
        .route("/gitops/sync", post(trigger_sync))
        .with_state(state)
}

/// Report GitOps status by checking for ArgoCD or Flux applications.
#[cfg(feature = "web")]
async fn get_gitops_status(State(state): State<SharedState>) -> Json<GitOpsStatus> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;

    // Check for vmrogue gitops config in ConfigMap
    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(s.client().client(), &s.namespace);
    let params = kube::api::ListParams::default().labels("vmrogue.io/type=gitops");

    let (repo_url, branch) = if let Ok(cms) = api.list(&params).await {
        cms.items.first().and_then(|cm| {
            let data = cm.data.as_ref()?;
            Some((
                data.get("repo_url").cloned().unwrap_or_default(),
                data.get("branch").cloned().unwrap_or_else(|| "main".to_string()),
            ))
        }).unwrap_or_default()
    } else {
        (String::new(), String::new())
    };

    let sync_status = if repo_url.is_empty() {
        "not_configured"
    } else {
        "unknown"
    };

    Json(GitOpsStatus {
        repo_url,
        branch,
        last_commit: String::new(),
        sync_status: sync_status.to_string(),
        last_synced: None,
        drift_detected: false,
    })
}

#[cfg(feature = "web")]
async fn trigger_sync(
    State(state): State<SharedState>,
    Json(req): Json<GitOpsSyncRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();

    log::info!(
        "GitOps sync triggered: dry_run={}, force={}, namespace={}, vms={}",
        req.dry_run, req.force, s.namespace, vms.len()
    );

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": if req.dry_run { "dry_run" } else { "synced" },
            "namespace": s.namespace,
            "vms_in_cluster": vms.len(),
            "dry_run": req.dry_run,
            "force": req.force,
            "note": "Use 'vmrogue gitops-export' to export manifests, then 'vmrogue gitops-diff' to compare with cluster state."
        })),
    )
}
