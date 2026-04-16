#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/monitoring/status", get(get_monitoring_status))
        .with_state(state)
}

/// Detect monitoring stack availability by checking for well-known services.
#[cfg(feature = "web")]
async fn get_monitoring_status(State(state): State<SharedState>) -> Json<MonitoringStatus> {
    use k8s_openapi::api::core::v1::Service;

    let s = state.read().await;
    let client = s.client().client();

    // Check for Prometheus, Grafana, Alertmanager across common namespaces
    let monitoring_namespaces = ["monitoring", "prometheus", "observability", &s.namespace];
    let mut prometheus = false;
    let mut grafana = false;
    let mut alertmanager = false;

    for ns in &monitoring_namespaces {
        let svc_api: kube::api::Api<Service> = kube::api::Api::namespaced(client.clone(), ns);
        if let Ok(svcs) = svc_api.list(&kube::api::ListParams::default()).await {
            for svc in &svcs.items {
                let name = svc.metadata.name.as_deref().unwrap_or("");
                if name.contains("prometheus") {
                    prometheus = true;
                }
                if name.contains("grafana") {
                    grafana = true;
                }
                if name.contains("alertmanager") {
                    alertmanager = true;
                }
            }
        }
    }

    let nodes = s.client().list_nodes().await.unwrap_or_default();
    let total_targets = nodes.len() as u32;
    let healthy_targets = nodes
        .iter()
        .filter(|n| {
            n.status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
                .map(|c| c.status == "True")
                .unwrap_or(false)
        })
        .count() as u32;

    Json(MonitoringStatus {
        prometheus_available: prometheus,
        grafana_available: grafana,
        alertmanager_available: alertmanager,
        metrics_collection_interval: "30s".to_string(),
        retention_period: "15d".to_string(),
        active_alerts: 0,
        total_targets,
        healthy_targets,
    })
}
