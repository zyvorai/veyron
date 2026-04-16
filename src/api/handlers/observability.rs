#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
    Router::new().route("/observability/overview", get(get_observability_overview))
}

#[cfg(feature = "web")]
async fn get_observability_overview() -> impl IntoResponse {
    not_implemented("Observability overview")
}
