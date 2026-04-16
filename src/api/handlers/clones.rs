#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Clone request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRequest {
    pub source_vm: String,
    pub target_name: String,
    pub namespace: Option<String>,
    pub start_after_clone: bool,
}

/// Clone response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneResponse {
    pub name: String,
    pub source_vm: String,
    pub namespace: String,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/clones", post(create_clone))
}

#[cfg(feature = "web")]
async fn create_clone(axum::Json(req): axum::Json<CloneRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("VM cloning")
}
