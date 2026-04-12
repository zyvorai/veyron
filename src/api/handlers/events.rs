#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Event response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    pub id: String,
    pub event_type: String,
    pub reason: String,
    pub message: String,
    pub namespace: String,
    pub involved_object: String,
    pub timestamp: String,
    pub count: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/events", get(list_events))
        .route("/events/recent", get(list_recent_events))
}

#[cfg(feature = "web")]
async fn list_events() -> Json<Vec<EventResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn list_recent_events() -> Json<Vec<EventResponse>> {
    Json(vec![])
}
