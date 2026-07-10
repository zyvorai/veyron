// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};
#[cfg(feature = "web")]
use crate::snapshots::{SnapshotConfig, SnapshotManager};
#[cfg(feature = "web")]
use kube;

/// Snapshot response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub id: String,
    pub name: String,
    pub vm_name: String,
    pub namespace: String,
    pub status: String,
    #[serde(rename = "ready")]
    pub ready_to_use: bool,
    pub size_bytes: Option<u64>,
    pub created_at: String,
    #[serde(default)]
    pub age: String,
}

/// Create snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {
    /// Namespace containing the VM (required when API default is `all`).
    #[serde(default)]
    pub namespace: Option<String>,
    pub vm_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Request an application-consistent (guest-quiesced) snapshot.
    #[serde(default)]
    pub app_consistent: bool,
}

/// Restore snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSnapshotRequest {
    pub target_vm: Option<String>,
}

#[cfg(feature = "web")]
async fn resolve_snapshot_op_namespace(
    client: &kube::Client,
    query_scope: &str,
    snapshot_id: &str,
) -> Option<String> {
    if !namespace_scope::is_all_namespaces(query_scope) {
        return Some(query_scope.to_string());
    }
    for ns in namespace_scope::kubernetes_namespace_names(client).await {
        let mgr = SnapshotManager::from_client(client.clone(), &ns);
        if mgr.get_snapshot(snapshot_id).await.is_ok() {
            return Some(ns);
        }
    }
    None
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/snapshots", get(list_snapshots).post(create_snapshot))
        .route("/snapshots/:id", delete(delete_snapshot))
        .route("/snapshots/:id/restore", post(restore_snapshot))
        .route("/snapshots/export", get(export_snapshots))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_snapshots(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Result<Json<Vec<SnapshotResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let client = s.client().client();

    let snapshots = if namespace_scope::is_all_namespaces(&scope) {
        let mut merged = Vec::new();
        for ns in namespace_scope::kubernetes_namespace_names(&client).await {
            let manager = SnapshotManager::from_client(client.clone(), &ns);
            if let Ok(mut snaps) = manager.list_all_snapshots().await {
                merged.append(&mut snaps);
            }
        }
        merged
    } else {
        let manager = SnapshotManager::from_client(client.clone(), &scope);
        manager.list_all_snapshots().await.map_err(|e| {
            log::error!("Failed to list snapshots: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": "KUBERNETES_ERROR",
                    "message": "Failed to list snapshots from cluster"
                })),
            )
        })?
    };

    let results: Vec<SnapshotResponse> = snapshots
        .into_iter()
        .map(|snap| {
            let created_at = snap.created_at.map(|t| t.to_rfc3339()).unwrap_or_default();
            SnapshotResponse {
                id: snap.name.clone(),
                name: snap.name,
                vm_name: snap.vm_name,
                namespace: snap.namespace,
                status: snap.status.to_string(),
                ready_to_use: snap.ready_to_use,
                size_bytes: snap.size.as_deref().map(crate::utils::parse_memory_bytes),
                age: fmt_age_from_rfc3339(&created_at),
                created_at,
            }
        })
        .collect();
    Ok(Json(results))
}

#[cfg(feature = "web")]
async fn create_snapshot(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(req): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotResponse>, StatusCode> {
    let s = state.read().await;
    let ns =
        namespace_scope::resolve_opt(req.namespace.clone().or(q.namespace.clone()), &s.namespace);
    if namespace_scope::is_all_namespaces(&ns) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let manager = SnapshotManager::from_client(s.client().client(), &ns);

    let snapshot_name = req.name.unwrap_or_else(|| {
        format!(
            "snap-{}-{}",
            req.vm_name,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        )
    });

    let config = SnapshotConfig::new(&req.vm_name, &snapshot_name)
        .with_description(req.description.unwrap_or_default())
        .with_app_consistent(req.app_consistent);

    match manager.create_snapshot(&config).await {
        Ok(snap) => {
            let created_at = snap.created_at.map(|t| t.to_rfc3339()).unwrap_or_default();
            Ok(Json(SnapshotResponse {
                id: snap.name.clone(),
                name: snap.name,
                vm_name: snap.vm_name,
                namespace: snap.namespace,
                status: snap.status.to_string(),
                ready_to_use: snap.ready_to_use,
                size_bytes: snap.size.as_deref().map(crate::utils::parse_memory_bytes),
                age: fmt_age_from_rfc3339(&created_at),
                created_at,
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_snapshot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> StatusCode {
    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let Some(ns) = resolve_snapshot_op_namespace(&client, &scope, &id).await else {
        return StatusCode::NOT_FOUND;
    };
    let manager = SnapshotManager::from_client(client, &ns);

    match manager.delete_snapshot(&id).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

#[cfg(feature = "web")]
async fn restore_snapshot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<DashboardNamespaceQuery>,
    Json(req): Json<RestoreSnapshotRequest>,
) -> StatusCode {
    use crate::snapshots::restore::RestoreManager;

    let s = state.read().await;
    let client = s.client().client();
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let Some(snap_ns) = resolve_snapshot_op_namespace(&client, &scope, &id).await else {
        return StatusCode::NOT_FOUND;
    };
    let restore_mgr = RestoreManager::from_client(client.clone(), &snap_ns);

    let result = if let Some(target_vm) = req.target_vm {
        restore_mgr.restore_to_new_vm(&id, &target_vm, false).await
    } else {
        // Look up the snapshot to find the original VM name
        let snap_mgr = SnapshotManager::from_client(client.clone(), &snap_ns);
        match snap_mgr.get_snapshot(&id).await {
            Ok(info) => restore_mgr.restore_in_place(&info.vm_name, &id).await,
            Err(_) => return StatusCode::NOT_FOUND,
        }
    };

    match result {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            log::error!("Failed to restore snapshot '{}': {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(feature = "web")]
pub async fn export_snapshots(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let client = s.client().client();

    let snapshots = if namespace_scope::is_all_namespaces(&scope) {
        let mut merged = Vec::new();
        for ns in namespace_scope::kubernetes_namespace_names(&client).await {
            let manager = SnapshotManager::from_client(client.clone(), &ns);
            if let Ok(mut snaps) = manager.list_all_snapshots().await {
                merged.append(&mut snaps);
            }
        }
        merged
    } else {
        let manager = SnapshotManager::from_client(client, &scope);
        manager.list_all_snapshots().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
    };

    Ok(Json(serde_json::json!({
        "namespace": scope,
        "count": snapshots.len(),
        "snapshots": snapshots,
        "export_format": "kubevirt_virtualmachinesnapshot_list",
    })))
}

#[cfg(feature = "web")]
fn fmt_age_from_rfc3339(ts: &str) -> String {
    if ts.is_empty() {
        return "—".to_string();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let now = chrono::Utc::now();
        let secs = (now - dt.with_timezone(&chrono::Utc)).num_seconds().max(0) as u64;
        if secs < 120 {
            return "just now".to_string();
        } else if secs < 3600 {
            return format!("{}m ago", secs / 60);
        } else if secs < 86400 {
            return format!("{}h ago", secs / 3600);
        } else {
            return format!("{}d ago", secs / 86400);
        }
    }
    "—".to_string()
}
