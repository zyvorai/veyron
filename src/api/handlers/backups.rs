#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/backups", get(list_backups).post(create_backup))
        .route("/backups/{id}/restore", post(restore_backup))
}

#[cfg(feature = "web")]
async fn list_backups() -> Json<Vec<BackupResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_backup(Json(req): Json<CreateBackupRequest>) -> Json<Option<BackupResponse>> {
    let _ = req;
    Json(None)
}

#[cfg(feature = "web")]
async fn restore_backup(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"restoring": id}))
}
