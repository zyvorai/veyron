#[cfg(feature = "web")]
use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
async fn list_backups() -> impl IntoResponse {
    not_implemented("Backups")
}

#[cfg(feature = "web")]
async fn create_backup(axum::Json(req): axum::Json<CreateBackupRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("Backup creation")
}

#[cfg(feature = "web")]
async fn restore_backup(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("Backup restoration")
}
