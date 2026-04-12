#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Custom resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomResourceResponse {
    pub name: String,
    pub group: String,
    pub version: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub scope: String,
    pub instance_count: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/custom-resources", get(list_custom_resources))
}

#[cfg(feature = "web")]
async fn list_custom_resources() -> Json<Vec<CustomResourceResponse>> {
    Json(vec![])
}
