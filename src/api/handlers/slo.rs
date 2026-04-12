#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// SLO objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloObjective {
    pub name: String,
    pub service: String,
    pub sli_type: String,
    pub target: f64,
    pub current: f64,
    pub error_budget_remaining: f64,
    pub window: String,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/slo/objectives", get(list_slo_objectives))
}

#[cfg(feature = "web")]
async fn list_slo_objectives() -> Json<Vec<SloObjective>> {
    Json(vec![])
}
