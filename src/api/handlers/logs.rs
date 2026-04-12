#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Log query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryParams {
    pub level: Option<String>,
    pub source: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u32>,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/logs", get(list_logs))
        .route("/logs/query", get(query_logs))
}

#[cfg(feature = "web")]
async fn list_logs() -> Json<Vec<LogEntry>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn query_logs() -> Json<Vec<LogEntry>> {
    Json(vec![])
}
