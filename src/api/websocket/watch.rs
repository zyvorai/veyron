use serde::{Deserialize, Serialize};

/// Watch subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchRequest {
    pub resource_type: String,
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub label_selector: Option<String>,
}

/// Watch event (sent over WebSocket when resources change)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub event_type: WatchEventType,
    pub resource_type: String,
    pub resource_name: String,
    pub namespace: String,
    pub resource: serde_json::Value,
    pub timestamp: String,
}

/// Watch event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchEventType {
    Added,
    Modified,
    Deleted,
    Bookmark,
    Error,
}
