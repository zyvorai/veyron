#[cfg(feature = "web")]
use axum::{extract::Path, Json, Router, routing::{get, put}};
use serde::{Deserialize, Serialize};

/// Alert response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResponse {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub status: String,
    pub message: String,
    pub source: String,
    pub fired_at: String,
    pub resolved_at: Option<String>,
}

/// Create alert request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRequest {
    pub name: String,
    pub severity: String,
    pub metric: String,
    pub operator: String,
    pub threshold: f64,
    pub duration: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/alerts", get(list_alerts).post(create_alert))
        .route("/alerts/{id}/resolve", put(resolve_alert))
}

#[cfg(feature = "web")]
async fn list_alerts() -> Json<Vec<AlertResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_alert(Json(req): Json<CreateAlertRequest>) -> Json<Option<AlertResponse>> {
    let _ = req;
    Json(None)
}

#[cfg(feature = "web")]
async fn resolve_alert(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"resolved": id}))
}
