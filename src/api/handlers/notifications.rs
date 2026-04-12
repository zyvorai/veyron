#[cfg(feature = "web")]
use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};

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
async fn list_notifications() -> Json<Vec<NotificationResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn mark_notifications_read(Json(req): Json<MarkReadRequest>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"marked_read": req.notification_ids.len()}))
}
