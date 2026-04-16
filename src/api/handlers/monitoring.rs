#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Monitoring status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStatus {
    pub prometheus_available: bool,
    pub grafana_available: bool,
    pub alertmanager_available: bool,
    pub metrics_collection_interval: String,
    pub retention_period: String,
    pub active_alerts: u32,
    pub total_targets: u32,
    pub healthy_targets: u32,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/monitoring/status", get(get_monitoring_status))
}

#[cfg(feature = "web")]
async fn get_monitoring_status() -> impl IntoResponse {
    not_implemented("Monitoring status")
}
