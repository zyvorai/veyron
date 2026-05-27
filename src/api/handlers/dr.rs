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

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/dr/failover", post(dr_failover))
        .route("/dr/export", get(dr_export_manifests))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn dr_failover(
    State(_state): State<SharedState>,
    Json(req): Json<DrFailoverRequest>,
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
    sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
                "Would restore VM {} from snapshot {}{}",
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
            "Restored VM {} from snapshot {}{}",
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
        "note": "Apply VirtualMachine on target cluster; restore latest ready snapshot via KubeVirt VirtualMachineRestore or POST /dr/failover in target namespace after replication."
    })))
}
