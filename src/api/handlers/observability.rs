#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/observability/overview", get(get_observability_overview))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_observability_overview(
    State(state): State<SharedState>,
) -> Json<ObservabilityOverview> {
    use k8s_openapi::api::core::v1::Service;

    let s = state.read().await;
    let client = s.client().client();

    // Check for observability stack services
    let mut metrics_available = false;
    let mut logs_available = false;

    let namespaces = ["monitoring", "logging", "observability", &s.namespace];
    for ns in &namespaces {
        let svc_api: kube::api::Api<Service> = kube::api::Api::namespaced(client.clone(), ns);
        if let Ok(svcs) = svc_api.list(&kube::api::ListParams::default()).await {
            for svc in &svcs.items {
                let name = svc.metadata.name.as_deref().unwrap_or("");
                if name.contains("prometheus") || name.contains("metrics") {
                    metrics_available = true;
                }
                if name.contains("loki") || name.contains("elasticsearch") || name.contains("fluentd") {
                    logs_available = true;
                }
            }
        }
    }

    // Count events as a proxy for log entries
    let events = s.client().list_events(&s.namespace).await.unwrap_or_default();
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();

    Json(ObservabilityOverview {
        metrics_status: if metrics_available { "active" } else { "unavailable" }.to_string(),
        logs_status: if logs_available { "active" } else { "unavailable" }.to_string(),
        traces_status: "unavailable".to_string(),
        total_log_entries: events.len() as u64,
        total_metric_series: vms.len() as u64 * 4, // ~4 metric series per VM
        total_trace_spans: 0,
        data_ingestion_rate: "N/A".to_string(),
        storage_used: "N/A".to_string(),
    })
}
