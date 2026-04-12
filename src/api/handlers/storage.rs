#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Storage pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePool {
    pub name: String,
    pub storage_class: String,
    pub provisioner: String,
    pub total_capacity: String,
    pub used_capacity: String,
    pub available_capacity: String,
    pub volume_count: u32,
}

/// Storage usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub pvc_name: String,
    pub namespace: String,
    pub storage_class: String,
    pub capacity: String,
    pub used: String,
    pub usage_percent: f64,
    pub bound_to_vm: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/storage/pools", get(list_storage_pools))
        .route("/storage/usage", get(get_storage_usage))
}

#[cfg(feature = "web")]
async fn list_storage_pools() -> Json<Vec<StoragePool>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_storage_usage() -> Json<Vec<StorageUsage>> {
    Json(vec![])
}
