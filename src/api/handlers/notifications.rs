#[cfg(feature = "web")]
use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Notification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub title: String,
    pub message: String,
    pub severity: String,
    pub read: bool,
    pub created_at: String,
}

/// Mark notifications read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadRequest {
    pub notification_ids: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/read", post(mark_notifications_read))
}

#[cfg(feature = "web")]
async fn list_notifications() -> impl IntoResponse {
    not_implemented("Notifications")
}

#[cfg(feature = "web")]
async fn mark_notifications_read(axum::Json(req): axum::Json<MarkReadRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("Notification mark-read")
}
