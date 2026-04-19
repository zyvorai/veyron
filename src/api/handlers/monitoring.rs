#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Query, State}, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use k8s_openapi::api::core::v1::Event;

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

/// Anomaly detected on a VM metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub vm_name: String,
    pub namespace: String,
    pub metric: String,
    pub current_value: f64,
    pub threshold: f64,
    pub deviation_percent: f64,
    pub severity: String,
    pub detected_at: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnomalyQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/monitoring/status", get(get_monitoring_status))
        .route("/monitoring/anomalies", get(list_anomalies))
        .with_state(state)
}

/// Detect monitoring stack availability by checking for well-known services.
#[cfg(feature = "web")]
async fn get_monitoring_status(State(state): State<SharedState>) -> Json<MonitoringStatus> {
    use k8s_openapi::api::core::v1::Service;

    let s = state.read().await;
    let client = s.client().client();
    let events = s.client().list_events(&s.namespace).await.unwrap_or_default();

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
    let active_alerts = count_active_alerts(&events);

    // Try to read retention from a Prometheus ConfigMap
    let retention_period = {
        let mut found = None;
        'outer: for ns in &monitoring_namespaces {
            let cm_api: kube::api::Api<k8s_openapi::api::core::v1::ConfigMap> =
                kube::api::Api::namespaced(client.clone(), ns);
            if let Ok(cms) = cm_api.list(&kube::api::ListParams::default()).await {
                for cm in &cms.items {
                    let name = cm.metadata.name.as_deref().unwrap_or("");
                    if name.contains("prometheus") {
                        if let Some(data) = cm.data.as_ref() {
                            for val in data.values() {
                                if let Some(pos) = val.find("retention") {
                                    let tail = &val[pos..];
                                    if let Some(ret) = tail
                                        .split_whitespace()
                                        .nth(1)
                                        .filter(|v| v.chars().any(|c| c.is_ascii_digit()))
                                    {
                                        found = Some(ret.to_string());
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        found.unwrap_or_else(|| "15d".to_string())
    };

    Json(MonitoringStatus {
        prometheus_available: prometheus,
        grafana_available: grafana,
        alertmanager_available: alertmanager,
        metrics_collection_interval: "30s".to_string(),
        retention_period,
        active_alerts,
        total_targets,
        healthy_targets,
    })
}

/// Detect metric anomalies for all VMs in the namespace.
/// Uses simple threshold-based detection: CPU/Memory > 85% = warning, > 95% = critical.
#[cfg(feature = "web")]
async fn list_anomalies(
    State(state): State<SharedState>,
    Query(query): Query<AnomalyQuery>,
) -> Json<Vec<Anomaly>> {
    let s = state.read().await;
    let namespace = query.namespace.clone().unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };
    let now = chrono::Utc::now().to_rfc3339();
    let mut anomalies = Vec::new();

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let vm_ns = vm.metadata.namespace.clone().unwrap_or_else(|| namespace.clone());
        let collector = crate::monitoring::metrics::MetricsCollector::new(vm_ns.clone());
        let metrics = match collector.collect(vm_name).await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let checks: &[(&str, f64, f64)] = &[
            ("cpu_percent", metrics.cpu.usage_percent, 85.0),
            ("memory_percent", metrics.memory.usage_percent, 85.0),
        ];

        for (metric, value, warn_threshold) in checks {
            if *value >= *warn_threshold {
                let severity = if *value >= 95.0 { "critical" } else { "warning" };
                let deviation = (value - warn_threshold) / warn_threshold * 100.0;
                anomalies.push(Anomaly {
                    vm_name: vm_name.to_string(),
                    namespace: vm_ns.clone(),
                    metric: metric.to_string(),
                    current_value: (value * 100.0).round() / 100.0,
                    threshold: *warn_threshold,
                    deviation_percent: (deviation * 100.0).round() / 100.0,
                    severity: severity.to_string(),
                    detected_at: now.clone(),
                    description: format!(
                        "{} {} is {:.1}% (threshold {:.0}%)",
                        vm_name, metric, value, warn_threshold
                    ),
                });
            }
        }

        // Detect high disk I/O: write_bytes > 100 MB/s
        let write_mb = metrics.disk.write_bytes_per_sec as f64 / (1024.0 * 1024.0);
        if write_mb > 100.0 {
            anomalies.push(Anomaly {
                vm_name: vm_name.to_string(),
                namespace: vm_ns.clone(),
                metric: "disk_write_mb_per_sec".to_string(),
                current_value: (write_mb * 100.0).round() / 100.0,
                threshold: 100.0,
                deviation_percent: ((write_mb - 100.0) / 100.0 * 100.0 * 100.0).round() / 100.0,
                severity: if write_mb > 250.0 { "critical" } else { "warning" }.to_string(),
                detected_at: now.clone(),
                description: format!("{} disk write is {:.1} MB/s", vm_name, write_mb),
            });
        }
    }

    Json(anomalies)
}

#[cfg(feature = "web")]
fn count_active_alerts(events: &[Event]) -> u32 {
    events
        .iter()
        .filter(|e| e.type_.as_deref() == Some("Warning"))
        .count() as u32
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn count_active_alerts_only_counts_warning_events() {
        let events = vec![
            Event {
                type_: Some("Warning".to_string()),
                ..Default::default()
            },
            Event {
                type_: Some("Normal".to_string()),
                ..Default::default()
            },
            Event {
                type_: Some("Warning".to_string()),
                ..Default::default()
            },
        ];

        assert_eq!(count_active_alerts(&events), 2);
    }
}
