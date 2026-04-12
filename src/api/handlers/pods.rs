#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Pod response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodResponse {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node_name: Option<String>,
    pub ip: Option<String>,
    pub containers: Vec<String>,
    pub restart_count: u32,
    pub created_at: String,
}

/// Pod log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodLogEntry {
    pub timestamp: String,
    pub container: String,
    pub message: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/pods", get(list_pods))
        .route("/pods/{name}", get(get_pod))
        .route("/pods/{name}/logs", get(get_pod_logs))
}

#[cfg(feature = "web")]
async fn list_pods() -> Json<Vec<PodResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_pod(Path(name): Path<String>) -> Json<Option<PodResponse>> {
    let _ = name;
    Json(None)
}

#[cfg(feature = "web")]
async fn get_pod_logs(Path(name): Path<String>) -> Json<Vec<PodLogEntry>> {
    let _ = name;
    Json(vec![])
}
