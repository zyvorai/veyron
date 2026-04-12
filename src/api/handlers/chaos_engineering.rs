#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Chaos experiment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub name: String,
    pub experiment_type: String,
    pub target: String,
    pub namespace: String,
    pub status: String,
    pub duration: String,
    pub results: Option<ChaosResults>,
    pub created_at: String,
}

/// Chaos experiment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResults {
    pub success: bool,
    pub affected_resources: u32,
    pub recovery_time_seconds: Option<u64>,
    pub observations: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/chaos/experiments", get(list_experiments))
}

#[cfg(feature = "web")]
async fn list_experiments() -> Json<Vec<ChaosExperiment>> {
    Json(vec![])
}
