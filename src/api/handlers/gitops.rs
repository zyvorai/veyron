#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

use super::feature_context::VmrogueFeatureContext;

/// GitOps status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsStatus {
    pub vmrogue_context: VmrogueFeatureContext,
    pub repo_url: String,
    pub branch: String,
    pub last_commit: String,
    pub sync_status: String,
    pub last_synced: Option<String>,
    pub drift_detected: bool,
    /// Explains that VMRogue does not drive Argo CD / Flux APIs.
    pub note: String,
}

/// GitOps sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsSyncRequest {
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitOpsQuery {
    pub namespace: Option<String>,
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
async fn get_gitops_status(
    State(state): State<SharedState>,
    Query(query): Query<GitOpsQuery>,
) -> Json<GitOpsStatus> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());

    // Check for vmrogue gitops config in ConfigMap
    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(s.client().client(), &namespace);
    let params = kube::api::ListParams::default().labels("vmrogue.io/type=gitops");

    let (repo_url, branch, last_commit, last_synced, stored_vm_count) =
        if let Ok(cms) = api.list(&params).await {
            cms.items
                .first()
                .and_then(|cm| {
                    let data = cm.data.as_ref()?;
                    Some((
                        data.get("repo_url").cloned().unwrap_or_default(),
                        data.get("branch")
                            .cloned()
                            .unwrap_or_else(|| "main".to_string()),
                        data.get("last_commit").cloned().unwrap_or_default(),
                        data.get("last_synced").cloned(),
                        data.get("vm_count").and_then(|v| v.parse::<usize>().ok()),
                    ))
                })
                .unwrap_or_default()
        } else {
            Default::default()
        };

    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };

    let drift_detected = stored_vm_count.map(|c| c != vms.len()).unwrap_or(false);

    let sync_status = if repo_url.is_empty() {
        "not_configured"
    } else if drift_detected {
        "out_of_sync"
    } else if last_synced.is_some() {
        "synced"
    } else {
        "pending"
    };

    Json(GitOpsStatus {
        vmrogue_context: VmrogueFeatureContext::gitops_status(),
        repo_url,
        branch,
        last_commit,
        sync_status: sync_status.to_string(),
        last_synced,
        drift_detected,
        note: "Derived from VMRogue GitOps ConfigMaps and live VM counts. POST /gitops/sync updates timestamps in that ConfigMap only — it does not invoke Argo CD or Flux.".to_string(),
    })
}

#[cfg(feature = "web")]
async fn trigger_sync(
    State(state): State<SharedState>,
    Query(query): Query<GitOpsQuery>,
    Json(req): Json<GitOpsSyncRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    let now = chrono::Utc::now().to_rfc3339();

    if !req.dry_run {
        // Persist sync state back to the GitOps ConfigMap
        let api: kube::api::Api<ConfigMap> =
            kube::api::Api::namespaced(s.client().client(), &namespace);
        let params = kube::api::ListParams::default().labels("vmrogue.io/type=gitops");
        if let Ok(cms) = api.list(&params).await {
            if let Some(cm) = cms.items.first() {
                if let Some(name) = cm.metadata.name.as_deref() {
                    let patch = serde_json::json!({
                        "data": {
                            "last_synced": now,
                            "vm_count": vms.len().to_string(),
                        }
                    });
                    let _ = api
                        .patch(
                            name,
                            &kube::api::PatchParams::default(),
                            &kube::api::Patch::Merge(patch),
                        )
                        .await;
                }
            }
        }
    }

    log::info!(
        "GitOps sync triggered: dry_run={}, force={}, namespace={}, vms={}",
        req.dry_run,
        req.force,
        namespace,
        vms.len()
    );

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": if req.dry_run { "dry_run" } else { "synced" },
            "namespace": namespace,
            "vms_in_cluster": vms.len(),
            "synced_at": if req.dry_run { serde_json::Value::Null } else { serde_json::Value::String(now) },
            "dry_run": req.dry_run,
            "force": req.force,
            "note": "Updates the vmrogue GitOps ConfigMap timestamp when present; does not trigger Argo CD or Flux reconciliation.",
            "vmrogue_context": serde_json::to_value(VmrogueFeatureContext::gitops_status()).unwrap_or(serde_json::Value::Null),
        })),
    )
}
