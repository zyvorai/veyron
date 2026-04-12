#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::{delete, get, post}};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/snapshots", get(list_snapshots).post(create_snapshot))
        .route("/snapshots/{id}", delete(delete_snapshot))
        .route("/snapshots/{id}/restore", post(restore_snapshot))
}

#[cfg(feature = "web")]
async fn list_snapshots() -> Json<Vec<SnapshotResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_snapshot(Json(req): Json<CreateSnapshotRequest>) -> Json<Option<SnapshotResponse>> {
    let _ = req;
    Json(None)
}

#[cfg(feature = "web")]
async fn delete_snapshot(Path(id): Path<String>) -> Json<serde_json::Value> {
    serde_json::json!({"deleted": id});
    Json(serde_json::json!({"deleted": id}))
}

#[cfg(feature = "web")]
async fn restore_snapshot(
    Path(id): Path<String>,
    Json(req): Json<RestoreSnapshotRequest>,
) -> Json<serde_json::Value> {
    let _ = req;
    Json(serde_json::json!({"restored": id}))
}
