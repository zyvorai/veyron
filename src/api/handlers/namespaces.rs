#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Namespace response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceResponse {
    pub name: String,
    pub status: String,
    pub vm_count: u32,
    pub labels: std::collections::HashMap<String, String>,
    pub created_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/namespaces", get(list_namespaces))
}

#[cfg(feature = "web")]
async fn list_namespaces() -> Json<Vec<NamespaceResponse>> {
    Json(vec![])
}
