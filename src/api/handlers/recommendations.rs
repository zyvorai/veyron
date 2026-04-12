#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub category: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub resource: String,
    pub estimated_savings: Option<f64>,
    pub impact: String,
    pub effort: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/recommendations", get(list_recommendations))
}

#[cfg(feature = "web")]
async fn list_recommendations() -> Json<Vec<Recommendation>> {
    Json(vec![])
}
