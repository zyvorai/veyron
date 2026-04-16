#[cfg(feature = "web")]
use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
async fn list_alerts() -> impl IntoResponse {
    not_implemented("Alerts")
}

#[cfg(feature = "web")]
async fn create_alert(axum::Json(req): axum::Json<CreateAlertRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("Alert creation")
}

#[cfg(feature = "web")]
async fn resolve_alert(Path(id): Path<String>) -> impl IntoResponse {
    let _ = id;
    not_implemented("Alert resolution")
}
