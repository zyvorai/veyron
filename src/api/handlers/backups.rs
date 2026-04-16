#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Path, State}, http::StatusCode, routing::{get, post}};
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

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/backups", get(list_backups).post(create_backup))
        .route("/backups/{id}/restore", post(restore_backup))
        .with_state(state)
}

/// Backups are implemented as VirtualMachineSnapshots with backup labels.
#[cfg(feature = "web")]
async fn list_backups(State(state): State<SharedState>) -> Json<Vec<BackupResponse>> {
    use crate::snapshots::SnapshotManager;

    let s = state.read().await;
    let manager = SnapshotManager::from_client(s.client().client(), &s.namespace);

    let snapshots = manager.list_all_snapshots().await.unwrap_or_default();

    let results: Vec<BackupResponse> = snapshots
        .into_iter()
        .map(|snap| BackupResponse {
            id: snap.name.clone(),
            name: snap.name,
            vm_name: snap.vm_name,
            namespace: snap.namespace,
            backup_type: "snapshot".to_string(),
            status: snap.status.to_string(),
            size_bytes: None,
            compressed: false,
            encrypted: false,
            created_at: snap
                .created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            completed_at: snap.completed_at.map(|t| t.to_rfc3339()),
        })
        .collect();

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
            created_at: snap
                .created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            completed_at: None,
        })),
        Err(e) => {
            log::error!("Failed to create backup: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(feature = "web")]
async fn restore_backup(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> StatusCode {
    use crate::snapshots::restore::RestoreManager;
    use crate::snapshots::SnapshotManager;

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
