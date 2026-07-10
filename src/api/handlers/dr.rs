// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Disaster recovery API — snapshot-based failover and manifest export.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
#[cfg(feature = "web")]
use kube::api::{Api, PostParams};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrFailoverRequest {
    pub namespace: String,
    pub vm_name: String,
    #[serde(default)]
    pub dry_run: bool,
    /// Optional kubeconfig context name for cross-cluster DR (export/planning metadata).
    #[serde(default)]
    pub target_kubeconfig_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrFailoverResponse {
    pub status: String,
    pub snapshot_name: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kubeconfig_context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrExportQuery {
    pub namespace: String,
    pub vm_name: String,
}

/// Apply exported VM manifest on the **current** cluster (DR target site).
/// Cross-cluster: export from source, then POST this body on the recovery cluster API.
#[derive(Debug, Clone, Deserialize)]
pub struct DrApplyRequest {
    pub namespace: String,
    pub vm_name: String,
    #[serde(default)]
    pub target_namespace: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    /// Full VirtualMachine JSON from GET /dr/export (optional; defaults to live VM in namespace).
    #[serde(default)]
    pub virtual_machine: Option<serde_json::Value>,
    #[serde(default)]
    pub restore_latest_snapshot: bool,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DrApplyResponse {
    pub status: String,
    pub target_namespace: String,
    pub target_name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_name: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/dr/failover", post(dr_failover))
        .route("/dr/failback", post(dr_failback))
        .route("/dr/export", get(dr_export_manifests))
        .route("/dr/apply", post(dr_apply))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn dr_failover(
    State(_state): State<SharedState>,
    Json(req): Json<DrFailoverRequest>,
) -> Result<Json<DrFailoverResponse>, (StatusCode, Json<serde_json::Value>)> {
    perform_dr_restore(req, "failover").await
}

/// Failback: restore the VM from its latest ready snapshot when returning the
/// workload to its primary site after a failover. Same snapshot-restore path as
/// failover, direction-labelled — pair with `GET /dr/export` for cross-cluster.
#[cfg(feature = "web")]
async fn dr_failback(
    State(_state): State<SharedState>,
    Json(req): Json<DrFailoverRequest>,
) -> Result<Json<DrFailoverResponse>, (StatusCode, Json<serde_json::Value>)> {
    perform_dr_restore(req, "failback").await
}

#[cfg(feature = "web")]
async fn perform_dr_restore(
    req: DrFailoverRequest,
    direction: &str,
) -> Result<Json<DrFailoverResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.namespace.is_empty() || req.vm_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "namespace and vm_name required" })),
        ));
    }

    let manager = crate::snapshots::SnapshotManager::new(&req.namespace)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let snapshots = manager
        .list_snapshots_for_vm(&req.vm_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    if snapshots.is_empty() {
        return Ok(Json(DrFailoverResponse {
            status: "no_snapshots".to_string(),
            snapshot_name: None,
            message: format!("No snapshots for VM {} in {}", req.vm_name, req.namespace),
            target_kubeconfig_context: req.target_kubeconfig_context,
        }));
    }

    let mut sorted = snapshots;
    sorted.sort_by_key(|s| std::cmp::Reverse(s.created_at));
    let latest = &sorted[0];

    if !latest.ready_to_use {
        return Ok(Json(DrFailoverResponse {
            status: "not_ready".to_string(),
            snapshot_name: Some(latest.name.clone()),
            message: "Latest snapshot is not ready to use".to_string(),
            target_kubeconfig_context: req.target_kubeconfig_context,
        }));
    }

    if req.dry_run {
        return Ok(Json(DrFailoverResponse {
            status: "dry_run".to_string(),
            snapshot_name: Some(latest.name.clone()),
            message: format!(
                "[{direction}] Would restore VM {} from snapshot {}{}",
                req.vm_name,
                latest.name,
                req.target_kubeconfig_context
                    .as_ref()
                    .map(|c| format!(" (target context: {c})"))
                    .unwrap_or_default()
            ),
            target_kubeconfig_context: req.target_kubeconfig_context,
        }));
    }

    let restore_mgr = crate::snapshots::RestoreManager::new(&req.namespace)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    restore_mgr
        .restore_in_place(&req.vm_name, &latest.name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(DrFailoverResponse {
        status: "restored".to_string(),
        snapshot_name: Some(latest.name.clone()),
        message: format!(
            "[{direction}] Restored VM {} from snapshot {}{}",
            req.vm_name,
            latest.name,
            req.target_kubeconfig_context
                .as_ref()
                .map(|c| format!(
                    "; apply GET /dr/export manifests to context '{c}' for cross-cluster DR"
                ))
                .unwrap_or_default()
        ),
        target_kubeconfig_context: req.target_kubeconfig_context,
    }))
}

#[cfg(feature = "web")]
async fn dr_export_manifests(
    State(state): State<SharedState>,
    Query(q): Query<DrExportQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if q.namespace.is_empty() || q.vm_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "namespace and vm_name required" })),
        ));
    }
    let s = state.read().await;
    let client = s.client().client();
    let vm_api: kube::Api<crate::kube::types::VirtualMachine> =
        kube::Api::namespaced(client.clone(), &q.namespace);
    let vm = vm_api.get(&q.vm_name).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    let snap_mgr = crate::snapshots::SnapshotManager::from_client(client, &q.namespace);
    let snapshots = snap_mgr
        .list_snapshots_for_vm(&q.vm_name)
        .await
        .unwrap_or_default();
    Ok(Json(serde_json::json!({
        "namespace": q.namespace,
        "vm_name": q.vm_name,
        "virtual_machine": vm,
        "snapshots": snapshots,
        "note": "Apply VirtualMachine on target cluster via POST /dr/apply with virtual_machine payload, or POST /dr/failover for same-cluster in-place restore."
    })))
}

#[cfg(feature = "web")]
async fn dr_apply(
    State(state): State<SharedState>,
    Json(req): Json<DrApplyRequest>,
) -> Result<Json<DrApplyResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.namespace.is_empty() || req.vm_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "namespace and vm_name required" })),
        ));
    }

    let target_ns = req
        .target_namespace
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| req.namespace.clone());
    let target_name = req
        .target_name
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| req.vm_name.clone());

    if req.dry_run {
        return Ok(Json(DrApplyResponse {
            status: "dry_run".to_string(),
            target_namespace: target_ns.clone(),
            target_name: target_name.clone(),
            message: format!(
                "Would apply VM '{}' in namespace '{}' on this cluster",
                target_name, target_ns
            ),
            snapshot_name: None,
        }));
    }

    let s = state.read().await;
    let kube = s.client().clone();
    drop(s);

    let vm: crate::kube::types::VirtualMachine = if let Some(ref raw) = req.virtual_machine {
        serde_json::from_value(raw.clone()).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid virtual_machine: {e}") })),
            )
        })?
    } else {
        kube.get_vm(&req.namespace, &req.vm_name)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?
    };

    let mut to_apply = vm;
    to_apply.status = None;
    to_apply.metadata.name = Some(target_name.clone());
    to_apply.metadata.namespace = Some(target_ns.clone());
    to_apply.metadata.resource_version = None;
    to_apply.metadata.uid = None;
    to_apply.metadata.creation_timestamp = None;

    let vm_api: Api<crate::kube::types::VirtualMachine> =
        Api::namespaced(kube.client(), &target_ns);
    let pp = PostParams::default();
    match vm_api.create(&pp, &to_apply).await {
        Ok(_) => {}
        Err(kube::Error::Api(err)) if err.code == 409 => {
            vm_api
                .replace(&target_name, &pp, &to_apply)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                })?;
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ));
        }
    }

    let mut snapshot_used = None;
    if req.restore_latest_snapshot {
        let snap_mgr = crate::snapshots::SnapshotManager::new(&req.namespace)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        let mut snapshots = snap_mgr
            .list_snapshots_for_vm(&req.vm_name)
            .await
            .unwrap_or_default();
        snapshots.sort_by_key(|s| std::cmp::Reverse(s.created_at));
        if let Some(latest) = snapshots.into_iter().find(|s| s.ready_to_use) {
            let restore_mgr = crate::snapshots::RestoreManager::new(&target_ns)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                })?;
            restore_mgr
                .restore_to_new_vm(&latest.name, &target_name, false)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                })?;
            snapshot_used = Some(latest.name);
        }
    }

    Ok(Json(DrApplyResponse {
        status: "applied".to_string(),
        target_namespace: target_ns.clone(),
        target_name: target_name.clone(),
        message: "VirtualMachine applied on this cluster. Snapshots must be replicated separately for cross-cluster restore.".to_string(),
        snapshot_name: snapshot_used,
    }))
}
