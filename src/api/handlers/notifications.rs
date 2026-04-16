#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::{get, post}};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/read", post(mark_notifications_read))
        .with_state(state)
}

/// Build notifications from recent K8s events (Warning events = notifications).
#[cfg(feature = "web")]
async fn list_notifications(State(state): State<SharedState>) -> Json<Vec<NotificationResponse>> {
    let s = state.read().await;
    let events = s.client().list_events(&s.namespace).await.unwrap_or_default();

    let results: Vec<NotificationResponse> = events
        .iter()
        .filter_map(|event| {
            let event_type = event.type_.as_deref().unwrap_or("Normal");
            let reason = event.reason.as_deref().unwrap_or("");
            let message = event.message.as_deref().unwrap_or("");

            // Only surface Warning/Error events as notifications
            if event_type == "Normal" && !reason.contains("Failed") {
                return None;
            }

            let severity = match event_type {
                "Warning" => "warning",
                _ if reason.contains("Error") || reason.contains("Failed") => "error",
                _ => "info",
            };

            Some(NotificationResponse {
                id: event.metadata.uid.clone().unwrap_or_default(),
                title: reason.to_string(),
                message: message.to_string(),
                severity: severity.to_string(),
                read: false,
                created_at: event
                    .last_timestamp
                    .as_ref()
                    .or(event.metadata.creation_timestamp.as_ref())
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            })
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn mark_notifications_read(
    State(_state): State<SharedState>,
    Json(req): Json<MarkReadRequest>,
) -> Json<serde_json::Value> {
    // K8s events don't have a "read" state, so we acknowledge the request
    Json(serde_json::json!({
        "marked_read": req.notification_ids.len(),
        "status": "acknowledged"
    }))
}
