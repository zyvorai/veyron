// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

use super::feature_context::VmrogueFeatureContext;

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

#[derive(Debug, Clone, Deserialize)]
pub struct ForecastQuery {
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPredictionsResponse {
    pub veyron_context: VmrogueFeatureContext,
    pub predictions: Vec<ForecastPrediction>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/forecasting/predictions", get(list_predictions))
        .with_state(state)
}

/// Generate resource forecasts based on current cluster utilization.
#[cfg(feature = "web")]
async fn list_predictions(
    State(state): State<SharedState>,
    Query(query): Query<ForecastQuery>,
) -> Result<Json<ForecastPredictionsResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await
    } else {
        s.client().list_vms(&namespace).await
    }
    .map_err(crate::api::handlers::kube_list_error("VMs"))?;
    let nodes = s
        .client()
        .list_nodes()
        .await
        .map_err(crate::api::handlers::kube_list_error("nodes"))?;

    let total_vms = vms.len() as f64;
    let running_vms = vms
        .iter()
        .filter(|vm| vm.spec.running.unwrap_or(false))
        .count() as f64;

    let total_nodes = nodes.len().max(1) as f64;

    let vm_density = running_vms / total_nodes;

    // Use running ratio as a proxy for growth pressure:
    // high ratio → more VMs likely to be provisioned → higher growth rate estimate.
    let running_ratio = if total_vms > 0.0 {
        running_vms / total_vms
    } else {
        0.0
    };
    let growth_rate = if running_ratio > 0.9 {
        1.20 // cluster under pressure, fast growth expected
    } else if running_ratio > 0.7 {
        1.12
    } else if running_ratio > 0.4 {
        1.06
    } else {
        1.02 // mostly idle, minimal expected growth
    };

    let predicted_density = vm_density * growth_rate;
    let _predicted_vms = (total_vms * growth_rate).ceil();

    // Estimate days until density threshold breach at current growth rate
    let density_threshold = 20.0f64;
    let breach_days = if vm_density > 0.0 && growth_rate > 1.0 {
        let days = (density_threshold / vm_density).ln() / (growth_rate - 1.0).ln();
        if days > 0.0 && days < 365.0 {
            Some(format!("~{:.0} days", days))
        } else {
            None
        }
    } else {
        None
    };

    let trend_label = if growth_rate >= 1.15 {
        "increasing_fast"
    } else if growth_rate >= 1.08 {
        "increasing"
    } else if growth_rate >= 1.03 {
        "stable"
    } else {
        "flat"
    };

    let mut ctx = VmrogueFeatureContext::forecasting_predictions();
    let mut prom_growth_rate = growth_rate;
    if let Ok(base) = std::env::var("VEYRON_PROMETHEUS_URL") {
        if let Some(rate) = prometheus_vm_count_growth_rate(&base).await {
            prom_growth_rate = rate;
            ctx = VmrogueFeatureContext::forecasting_prometheus();
        }
    }
    let predicted_vms_prom = (total_vms * prom_growth_rate).ceil();

    let mut predictions = vec![
        ForecastPrediction {
            metric: "vm_count".to_string(),
            current_value: total_vms,
            predicted_value: predicted_vms_prom,
            confidence: 0.70,
            prediction_window: "7d".to_string(),
            trend: trend_label.to_string(),
            alert_threshold: Some(total_nodes * density_threshold),
            estimated_breach: None,
        },
        ForecastPrediction {
            metric: "vm_density_per_node".to_string(),
            current_value: (vm_density * 10.0).round() / 10.0,
            predicted_value: (predicted_density * 10.0).round() / 10.0,
            confidence: 0.65,
            prediction_window: "7d".to_string(),
            trend: trend_label.to_string(),
            alert_threshold: Some(density_threshold),
            estimated_breach: breach_days,
        },
        ForecastPrediction {
            metric: "running_vm_ratio".to_string(),
            current_value: (running_ratio * 100.0 * 10.0).round() / 10.0,
            predicted_value: ((running_ratio * growth_rate).min(1.0) * 100.0 * 10.0).round() / 10.0,
            confidence: 0.80,
            prediction_window: "24h".to_string(),
            trend: if running_ratio > 0.85 {
                "increasing"
            } else {
                "stable"
            }
            .to_string(),
            alert_threshold: Some(90.0),
            estimated_breach: if running_ratio > 0.85 {
                Some("~24h".to_string())
            } else {
                None
            },
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

    Ok(Json(ForecastPredictionsResponse {
        veyron_context: ctx,
        predictions,
    }))
}

#[cfg(feature = "web")]
async fn prometheus_vm_count_growth_rate(base: &str) -> Option<f64> {
    let end = chrono::Utc::now().timestamp();
    let start = end - 7 * 86400;
    let query = "count(kubevirt_vmi_phase_count{phase=\"Running\"})";
    let series = crate::api::prometheus::range_query_series(base, query, start, end, 3600)
        .await
        .ok()?;
    let values: Vec<f64> = series
        .into_iter()
        .flat_map(|(_, pts)| pts.into_iter().map(|(_, v)| v))
        .collect();
    if values.len() < 2 {
        return None;
    }
    let first = values.first()?.max(1.0);
    let last = *values.last()?;
    Some((last / first).clamp(1.0, 2.0))
}
