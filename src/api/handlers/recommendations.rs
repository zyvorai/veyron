#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
    Router::new().route("/recommendations", get(list_recommendations))
}

#[cfg(feature = "web")]
async fn list_recommendations() -> impl IntoResponse {
    not_implemented("Recommendations")
}
