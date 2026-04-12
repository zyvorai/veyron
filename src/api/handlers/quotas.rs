#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Quota response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaResponse {
    pub name: String,
    pub namespace: String,
    pub cpu_limit: String,
    pub cpu_used: String,
    pub memory_limit: String,
    pub memory_used: String,
    pub vm_limit: Option<u32>,
    pub vm_count: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/quotas", get(list_quotas))
}

#[cfg(feature = "web")]
async fn list_quotas() -> Json<Vec<QuotaResponse>> {
    Json(vec![])
}
