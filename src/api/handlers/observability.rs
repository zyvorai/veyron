#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
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

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityQuery {
    pub namespace: Option<String>,
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
    Query(query): Query<ObservabilityQuery>,
) -> Json<ObservabilityOverview> {
    use k8s_openapi::api::core::v1::Service;

    let s = state.read().await;
    let client = s.client().client();
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());

    // Check for observability stack services
    let mut metrics_available = false;
    let mut logs_available = false;
    let mut traces_available = false;

    let namespaces = [
        "monitoring",
        "logging",
        "observability",
        "kube-system",
        "cilium-system",
        "vmrogue-system",
        namespace.as_str(),
    ];
    for ns in &namespaces {
        let svc_api: kube::api::Api<Service> = kube::api::Api::namespaced(client.clone(), ns);
        if let Ok(svcs) = svc_api.list(&kube::api::ListParams::default()).await {
            for svc in &svcs.items {
                let name = svc.metadata.name.as_deref().unwrap_or("").to_lowercase();
                if name.contains("prometheus")
                    || name.contains("metrics-server")
                    || name.contains("thanos")
                    || name.contains("victoria")
                    || name.contains("vminsert")
                    || name.contains("vmselect")
                    || name.contains("mimir")
                    || name.contains("kube-state-metrics")
                    || name.contains("cadvisor")
                {
                    metrics_available = true;
                }
                if name.contains("loki")
                    || name.contains("elasticsearch")
                    || name.contains("fluentd")
                    || name.contains("fluent-bit")
                    || name.contains("vector")
                    || name.contains("promtail")
                    || name.contains("grafana-agent")
                    || name.contains("opensearch")
                {
                    logs_available = true;
                }
                if name.contains("jaeger")
                    || name.contains("tempo")
                    || name.contains("otel")
                    || name.contains("opentelemetry")
                    || name.contains("zipkin")
                {
                    traces_available = true;
                }
            }
        }
    }

    // Count events as a proxy for log entries
    let events = if namespace == "all" {
        s.client().list_all_events().await.unwrap_or_default()
    } else {
        s.client().list_events(&namespace).await.unwrap_or_default()
    };
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };

    // Estimate data ingestion rate from event frequency
    let ingestion_rate = if !events.is_empty() {
        let oldest = events
            .iter()
            .filter_map(|e| {
                e.first_timestamp
                    .as_ref()
                    .or(e.metadata.creation_timestamp.as_ref())
                    .map(|t| t.0)
            })
            .min();
        if let Some(oldest_ts) = oldest {
            let age_secs = (chrono::Utc::now() - oldest_ts).num_seconds().max(1) as f64;
            let rate = events.len() as f64 / age_secs;
            if rate >= 1.0 {
                format!("{:.1} events/s", rate)
            } else {
                format!("{:.1} events/min", rate * 60.0)
            }
        } else {
            "0 events/s".to_string()
        }
    } else {
        "0 events/s".to_string()
    };

    // Sum PVC storage across observability namespaces
    let obs_namespaces = ["monitoring", "logging", "observability", namespace.as_str()];
    let mut storage_bytes: u64 = 0;
    for ns in &obs_namespaces {
        if let Ok(pvcs) = s.client().list_pvcs(ns).await {
            for pvc in &pvcs {
                if let Some(cap) = pvc.status.as_ref().and_then(|s| s.capacity.as_ref()) {
                    if let Some(storage) = cap.get("storage") {
                        storage_bytes += crate::utils::parse_memory_bytes(&storage.0);
                    }
                }
            }
        }
    }
    let storage_used = if storage_bytes == 0 {
        "N/A".to_string()
    } else if storage_bytes >= 1 << 30 {
        format!("{:.1} GiB", storage_bytes as f64 / (1u64 << 30) as f64)
    } else {
        format!("{:.0} MiB", storage_bytes as f64 / (1u64 << 20) as f64)
    };

    Json(ObservabilityOverview {
        metrics_status: if metrics_available {
            "active"
        } else {
            "unavailable"
        }
        .to_string(),
        logs_status: if logs_available {
            "active"
        } else {
            "unavailable"
        }
        .to_string(),
        traces_status: if traces_available {
            "active"
        } else {
            "derived"
        }
        .to_string(),
        total_log_entries: events.len() as u64,
        total_metric_series: if metrics_available {
            (events.len() as u64)
                .saturating_mul(2)
                .saturating_add(vms.len() as u64 * 8)
        } else {
            vms.len() as u64 * 4
        },
        total_trace_spans: 0,
        data_ingestion_rate: ingestion_rate,
        storage_used,
    })
}
