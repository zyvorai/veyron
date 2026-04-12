#[cfg(feature = "web")]
use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};

/// Disk response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskResponse {
    pub name: String,
    pub vm_name: String,
    pub size: String,
    pub storage_class: Option<String>,
    pub access_mode: String,
    pub status: String,
}

/// Expand disk request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandDiskRequest {
    pub vm_name: String,
    pub disk_name: String,
    pub new_size: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/disks", get(list_disks))
        .route("/disks/expand", post(expand_disk))
}

#[cfg(feature = "web")]
async fn list_disks() -> Json<Vec<DiskResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn expand_disk(Json(req): Json<ExpandDiskRequest>) -> Json<serde_json::Value> {
    let _ = req;
    Json(serde_json::json!({"status": "expanding"}))
}
