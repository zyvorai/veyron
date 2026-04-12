#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Node response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub cpu_capacity: String,
    pub memory_capacity: String,
    pub cpu_allocatable: String,
    pub memory_allocatable: String,
    pub kubelet_version: String,
    pub os_image: String,
    pub kernel_version: String,
    pub vm_count: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/{name}", get(get_node))
}

#[cfg(feature = "web")]
async fn list_nodes() -> Json<Vec<NodeResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_node(Path(name): Path<String>) -> Json<Option<NodeResponse>> {
    let _ = name;
    Json(None)
}
