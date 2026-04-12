#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// VMI (VirtualMachineInstance) response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmiResponse {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node_name: Option<String>,
    pub ip_address: Option<String>,
    pub cpu_cores: u32,
    pub memory: String,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/vmis", get(list_vmis))
        .route("/vmis/{name}", get(get_vmi))
}

#[cfg(feature = "web")]
async fn list_vmis() -> Json<Vec<VmiResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_vmi(Path(name): Path<String>) -> Json<Option<VmiResponse>> {
    let _ = name;
    Json(None)
}
