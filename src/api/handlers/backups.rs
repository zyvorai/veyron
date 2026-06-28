// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Backup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResponse {
    pub id: String,
    pub name: String,
    pub vm_name: String,
    pub namespace: String,
    pub backup_type: String,
    pub status: String,
    pub size_bytes: Option<u64>,
    pub compressed: bool,
    pub encrypted: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Create backup request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub vm_name: String,
    pub name: Option<String>,
    pub backup_type: Option<String>,
    pub compress: bool,
    pub encrypt: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackupQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/backups", get(list_backups).post(create_backup))
        .route("/backups/:id/restore", post(restore_backup))
        .with_state(state)
}

/// Backups are implemented as VirtualMachineSnapshots with backup labels.
#[cfg(feature = "web")]
async fn list_backups(
    State(state): State<SharedState>,
    Query(query): Query<BackupQuery>,
) -> Json<Vec<BackupResponse>> {
    use crate::snapshots::crds::VirtualMachineSnapshot;
    use kube::api::{Api, ListParams};

    let s = state.read().await;
    let client = s.client().client();
    let api: Api<VirtualMachineSnapshot> = match query.namespace.as_deref() {
        Some("all") => Api::all(client),
        Some(ns) => Api::namespaced(client, ns),
        None => Api::namespaced(client, &s.namespace),
    };

    let results = api
        .list(&ListParams::default())
        .await
        .map(|list| list.items.into_iter().map(snapshot_to_backup).collect())
        .unwrap_or_default();

    Json(results)
}

#[cfg(feature = "web")]
async fn create_backup(
    State(state): State<SharedState>,
    Json(req): Json<CreateBackupRequest>,
) -> Result<Json<BackupResponse>, StatusCode> {
    use crate::snapshots::{SnapshotConfig, SnapshotManager};

    let s = state.read().await;
    let manager = SnapshotManager::from_client(s.client().client(), &s.namespace);

    let snapshot_name = req.name.unwrap_or_else(|| {
        format!(
            "backup-{}-{}",
            req.vm_name,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        )
    });

    let config = SnapshotConfig::new(&req.vm_name, &snapshot_name);

    match manager.create_snapshot(&config).await {
        Ok(snap) => Ok(Json(BackupResponse {
            id: snap.name.clone(),
            name: snap.name,
            vm_name: snap.vm_name,
            namespace: snap.namespace,
            backup_type: req.backup_type.unwrap_or_else(|| "snapshot".to_string()),
            status: snap.status.to_string(),
            size_bytes: None,
            compressed: req.compress,
            encrypted: req.encrypt,
            created_at: snap.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            completed_at: None,
        })),
        Err(e) => {
            log::error!("Failed to create backup: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(feature = "web")]
async fn restore_backup(State(state): State<SharedState>, Path(id): Path<String>) -> StatusCode {
    use crate::snapshots::SnapshotManager;
    use crate::snapshots::restore::RestoreManager;

    let s = state.read().await;
    let snap_mgr = SnapshotManager::from_client(s.client().client(), &s.namespace);
    let restore_mgr = RestoreManager::from_client(s.client().client(), &s.namespace);

    match snap_mgr.get_snapshot(&id).await {
        Ok(info) => match restore_mgr.restore_in_place(&info.vm_name, &id).await {
            Ok(_) => StatusCode::OK,
            Err(e) => {
                log::error!("Failed to restore backup '{}': {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        },
        Err(_) => StatusCode::NOT_FOUND,
    }
}

#[cfg(feature = "web")]
fn snapshot_to_backup(snapshot: crate::snapshots::crds::VirtualMachineSnapshot) -> BackupResponse {
    let created_at = snapshot
        .status
        .as_ref()
        .and_then(|s| s.creation_time.clone())
        .or_else(|| {
            snapshot
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
        })
        .unwrap_or_default();

    let ready_to_use = snapshot
        .status
        .as_ref()
        .and_then(|s| s.ready_to_use)
        .unwrap_or(false);

    let status = snapshot
        .status
        .as_ref()
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let vm_name = snapshot
        .metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get("veyron.io/vm"))
        .cloned()
        .unwrap_or_else(|| snapshot.spec.source.name.clone());

    BackupResponse {
        id: snapshot.metadata.name.clone().unwrap_or_default(),
        name: snapshot.metadata.name.unwrap_or_default(),
        vm_name,
        namespace: snapshot.metadata.namespace.unwrap_or_default(),
        backup_type: "snapshot".to_string(),
        status,
        size_bytes: None,
        compressed: false,
        encrypted: false,
        created_at: created_at.clone(),
        completed_at: ready_to_use.then_some(created_at),
    }
}
