#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Forecast prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPrediction {
    pub metric: String,
    pub current_value: f64,
    pub predicted_value: f64,
    pub confidence: f64,
    pub prediction_window: String,
    pub trend: String,
    pub alert_threshold: Option<f64>,
    pub estimated_breach: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/forecasting/predictions", get(list_predictions))
        .with_state(state)
}

/// Generate resource forecasts based on current cluster utilization.
#[cfg(feature = "web")]
async fn list_predictions(State(state): State<SharedState>) -> Json<Vec<ForecastPrediction>> {
    let s = state.read().await;
    let vms = s.client().list_vms(&s.namespace).await.unwrap_or_default();
    let nodes = s.client().list_nodes().await.unwrap_or_default();

    let total_vms = vms.len() as f64;
    let running_vms = vms
        .iter()
        .filter(|vm| vm.spec.running.unwrap_or(false))
        .count() as f64;

    let total_nodes = nodes.len().max(1) as f64;

    // Simple forecasts based on current state
    let vm_density = running_vms / total_nodes;
    let predicted_density = vm_density * 1.1; // 10% growth assumption

    let mut predictions = vec![
        ForecastPrediction {
            metric: "vm_count".to_string(),
            current_value: total_vms,
            predicted_value: (total_vms * 1.1).ceil(),
            confidence: 0.7,
            prediction_window: "7d".to_string(),
            trend: if total_vms > 0.0 { "stable" } else { "flat" }.to_string(),
            alert_threshold: Some(total_nodes * 20.0), // 20 VMs per node threshold
            estimated_breach: None,
        },
        ForecastPrediction {
            metric: "vm_density_per_node".to_string(),
            current_value: (vm_density * 10.0).round() / 10.0,
            predicted_value: (predicted_density * 10.0).round() / 10.0,
            confidence: 0.65,
            prediction_window: "7d".to_string(),
            trend: "increasing".to_string(),
            alert_threshold: Some(20.0),
            estimated_breach: if predicted_density > 20.0 {
                Some("~7 days".to_string())
            } else {
                None
            },
        },
        ForecastPrediction {
            metric: "running_vm_ratio".to_string(),
            current_value: if total_vms > 0.0 {
                (running_vms / total_vms * 100.0 * 10.0).round() / 10.0
            } else {
                0.0
            },
            predicted_value: if total_vms > 0.0 {
                (running_vms / total_vms * 100.0 * 10.0).round() / 10.0
            } else {
                0.0
            },
            confidence: 0.8,
            prediction_window: "24h".to_string(),
            trend: "stable".to_string(),
            alert_threshold: None,
            estimated_breach: None,
        },
    ];

    // Node capacity prediction
    let total_cpu: f64 = nodes
        .iter()
        .filter_map(|n| {
            n.status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("cpu"))
                .and_then(|v| v.0.parse::<f64>().ok())
        })
        .sum();

    if total_cpu > 0.0 {
        let vm_cpu: f64 = vms
            .iter()
            .filter_map(|vm| {
                vm.spec
                    .template
                    .spec
                    .domain
                    .cpu
                    .as_ref()
                    .and_then(|c| c.cores)
                    .map(|c| c as f64)
            })
            .sum();

        let cpu_util = vm_cpu / total_cpu * 100.0;
        predictions.push(ForecastPrediction {
            metric: "cpu_allocation_percent".to_string(),
            current_value: (cpu_util * 10.0).round() / 10.0,
            predicted_value: ((cpu_util * 1.1) * 10.0).round() / 10.0,
            confidence: 0.6,
            prediction_window: "7d".to_string(),
            trend: if cpu_util > 70.0 {
                "increasing"
            } else {
                "stable"
            }
            .to_string(),
            alert_threshold: Some(90.0),
            estimated_breach: if cpu_util > 80.0 {
                Some("~14 days".to_string())
            } else {
                None
            },
        });
    }

    Json(predictions)
}
