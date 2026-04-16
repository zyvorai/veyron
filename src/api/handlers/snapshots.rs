#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::snapshots::{SnapshotConfig, SnapshotManager};

/// Snapshot response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub id: String,
    pub name: String,
    pub vm_name: String,
    pub namespace: String,
    pub status: String,
    pub ready_to_use: bool,
    pub size_bytes: Option<u64>,
    pub created_at: String,
}

/// Create snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {
    pub vm_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Restore snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSnapshotRequest {
    pub target_vm: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/snapshots", get(list_snapshots).post(create_snapshot))
        .route("/snapshots/{id}", delete(delete_snapshot))
        .route("/snapshots/{id}/restore", post(restore_snapshot))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_snapshots(
    State(state): State<SharedState>,
) -> Result<Json<Vec<SnapshotResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let manager = SnapshotManager::from_client(s.client().client(), &s.namespace);

    match manager.list_all_snapshots().await {
        Ok(snapshots) => {
            let results: Vec<SnapshotResponse> = snapshots
                .into_iter()
                .map(|snap| SnapshotResponse {
                    id: snap.name.clone(),
                    name: snap.name,
                    vm_name: snap.vm_name,
                    namespace: snap.namespace,
                    status: snap.status.to_string(),
                    ready_to_use: snap.ready_to_use,
                    size_bytes: None,
                    created_at: snap
                        .created_at
                        .map(|t| t.to_rfc3339())
                        .unwrap_or_default(),
                })
                .collect();
            Ok(Json(results))
        }
        Err(e) => {
            log::error!("Failed to list snapshots: {}", e);
            Err((
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": "KUBERNETES_ERROR",
                    "message": "Failed to list snapshots from cluster"
                })),
            ))
        }
    }
}

#[cfg(feature = "web")]
async fn create_snapshot(
    State(state): State<SharedState>,
    Json(req): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotResponse>, StatusCode> {
    let s = state.read().await;
    let manager = SnapshotManager::from_client(s.client().client(), &s.namespace);

    let snapshot_name = req
        .name
        .unwrap_or_else(|| {
            format!(
                "snap-{}-{}",
                req.vm_name,
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            )
        });

    let config = SnapshotConfig::new(&req.vm_name, &snapshot_name)
        .with_description(req.description.unwrap_or_default());

    match manager.create_snapshot(&config).await {
        Ok(snap) => Ok(Json(SnapshotResponse {
            id: snap.name.clone(),
            name: snap.name,
            vm_name: snap.vm_name,
            namespace: snap.namespace,
            status: snap.status.to_string(),
            ready_to_use: snap.ready_to_use,
            size_bytes: None,
            created_at: snap
                .created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "web")]
async fn delete_snapshot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> StatusCode {
    let s = state.read().await;
    let manager = SnapshotManager::from_client(s.client().client(), &s.namespace);

    match manager.delete_snapshot(&id).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

#[cfg(feature = "web")]
async fn restore_snapshot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<RestoreSnapshotRequest>,
) -> StatusCode {
    use crate::snapshots::restore::RestoreManager;

    let s = state.read().await;
    let restore_mgr = RestoreManager::from_client(s.client().client(), &s.namespace);

    let result = if let Some(target_vm) = req.target_vm {
        restore_mgr.restore_to_new_vm(&id, &target_vm, false).await
    } else {
        // Look up the snapshot to find the original VM name
        let snap_mgr = SnapshotManager::from_client(s.client().client(), &s.namespace);
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
