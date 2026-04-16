#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/events", get(list_events))
        .route("/events/recent", get(list_recent_events))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_events(State(state): State<SharedState>) -> Json<Vec<EventResponse>> {
    let s = state.read().await;
    let events = s.client().list_events(&s.namespace).await.unwrap_or_default();

    let results: Vec<EventResponse> = events
        .iter()
        .map(|event| {
            let meta = &event.metadata;
            let involved = &event.involved_object;

            EventResponse {
                id: meta.uid.clone().unwrap_or_default(),
                event_type: event.type_.clone().unwrap_or_else(|| "Normal".to_string()),
                reason: event.reason.clone().unwrap_or_default(),
                message: event.message.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                involved_object: format!(
                    "{}/{}",
                    involved.kind.as_deref().unwrap_or(""),
                    involved.name.as_deref().unwrap_or("")
                ),
                timestamp: event
                    .last_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .or_else(|| {
                        meta.creation_timestamp.as_ref().map(|t| t.0.to_rfc3339())
                    })
                    .unwrap_or_default(),
                count: event.count.unwrap_or(1) as u32,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn list_recent_events(State(state): State<SharedState>) -> Json<Vec<EventResponse>> {
    let mut events = list_events(State(state)).await.0;
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    events.truncate(50);
    Json(events)
}
