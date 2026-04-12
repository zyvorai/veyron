#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Observability overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityOverview {
    pub metrics_status: String,
    pub logs_status: String,
    pub traces_status: String,
    pub total_log_entries: u64,
    pub total_metric_series: u64,
    pub total_trace_spans: u64,
    pub data_ingestion_rate: String,
    pub storage_used: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/observability/overview", get(get_observability_overview))
}

#[cfg(feature = "web")]
async fn get_observability_overview() -> Json<ObservabilityOverview> {
    Json(ObservabilityOverview {
        metrics_status: "unknown".to_string(),
        logs_status: "unknown".to_string(),
        traces_status: "unknown".to_string(),
        total_log_entries: 0,
        total_metric_series: 0,
        total_trace_spans: 0,
        data_ingestion_rate: "0/s".to_string(),
        storage_used: "0B".to_string(),
    })
}
