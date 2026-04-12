#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Operator response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResponse {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub status: String,
    pub managed_resources: Vec<String>,
    pub installed_at: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/operators", get(list_operators))
}

#[cfg(feature = "web")]
async fn list_operators() -> Json<Vec<OperatorResponse>> {
    Json(vec![])
}
