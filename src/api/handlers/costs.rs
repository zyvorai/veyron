#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Cost entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub vm_name: String,
    pub namespace: String,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub storage_cost: f64,
    pub network_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    pub period: String,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost: f64,
    pub currency: String,
    pub period: String,
    pub by_namespace: std::collections::HashMap<String, f64>,
    pub by_resource_type: std::collections::HashMap<String, f64>,
}

/// Cost forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    pub current_monthly: f64,
    pub projected_monthly: f64,
    pub trend: String,
    pub currency: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/costs", get(list_costs))
        .route("/costs/summary", get(get_cost_summary))
        .route("/costs/forecast", get(get_cost_forecast))
}

#[cfg(feature = "web")]
async fn list_costs() -> Json<Vec<CostEntry>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_cost_summary() -> Json<CostSummary> {
    Json(CostSummary {
        total_cost: 0.0,
        currency: "USD".to_string(),
        period: "monthly".to_string(),
        by_namespace: std::collections::HashMap::new(),
        by_resource_type: std::collections::HashMap::new(),
    })
}

#[cfg(feature = "web")]
async fn get_cost_forecast() -> Json<CostForecast> {
    Json(CostForecast {
        current_monthly: 0.0,
        projected_monthly: 0.0,
        trend: "stable".to_string(),
        currency: "USD".to_string(),
    })
}
