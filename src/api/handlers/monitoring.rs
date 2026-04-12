#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
    Router::new()
        .route("/monitoring/status", get(get_monitoring_status))
}

#[cfg(feature = "web")]
async fn get_monitoring_status() -> Json<MonitoringStatus> {
    Json(MonitoringStatus {
        prometheus_available: false,
        grafana_available: false,
        alertmanager_available: false,
        metrics_collection_interval: "30s".to_string(),
        retention_period: "15d".to_string(),
        active_alerts: 0,
        total_targets: 0,
        healthy_targets: 0,
    })
}
