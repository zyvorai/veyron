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

/// SLO objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloObjective {
    pub name: String,
    pub service: String,
    pub sli_type: String,
    pub target: f64,
    pub current: f64,
    pub error_budget_remaining: f64,
    pub window: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SloQuery {
    pub namespace: Option<String>,
}

/// Error budget burn rate for an SLO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloErrorBudget {
    pub slo_name: String,
    pub service: String,
    pub target: f64,
    pub current_availability: f64,
    pub budget_total_percent: f64,
    pub budget_consumed_percent: f64,
    pub burn_rate_1h: f64,
    pub burn_rate_24h: f64,
    pub projected_exhaustion_hours: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BurnRateQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/slo/objectives", get(list_slo_objectives))
        .route("/slo/burn-rate", get(list_slo_burn_rates))
        .with_state(state)
}

/// Derive SLO status from VM availability in the namespace.
#[cfg(feature = "web")]
async fn list_slo_objectives(
    State(state): State<SharedState>,
    Query(query): Query<SloQuery>,
) -> Json<Vec<SloObjective>> {
    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };

    if vms.is_empty() {
        return Json(vec![]);
    }

    let total = vms.len() as f64;
    let running = vms
        .iter()
        .filter(|vm| vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false))
        .count() as f64;

    let availability = if total > 0.0 {
        running / total * 100.0
    } else {
        100.0
    };

    let target = 99.9;
    // Error budget remaining: how much downtime is left before SLO breach
    // Positive = budget remaining, negative = budget exhausted
    let error_budget = (100.0 - target) - (100.0 - availability);

    let status = if availability >= target {
        "met"
    } else if availability >= 99.0 {
        "at_risk"
    } else {
        "breached"
    };

    Json(vec![SloObjective {
        name: format!("{}-vm-availability", namespace),
        service: "kubevirt-vms".to_string(),
        sli_type: "availability".to_string(),
        target,
        current: (availability * 100.0).round() / 100.0,
        error_budget_remaining: (error_budget * 100.0).round() / 100.0,
        window: "30d".to_string(),
        status: status.to_string(),
    }])
}

/// Compute error budget burn rates from current VM availability.
/// burn_rate = (consumed_budget / budget_total) × (window / measurement_period)
/// A burn rate > 1 means the budget will be exhausted before the window ends.
#[cfg(feature = "web")]
async fn list_slo_burn_rates(
    State(state): State<SharedState>,
    Query(query): Query<BurnRateQuery>,
) -> Json<Vec<SloErrorBudget>> {
    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let vms = if namespace == "all" {
        s.client().list_all_vms().await.unwrap_or_default()
    } else {
        s.client().list_vms(&namespace).await.unwrap_or_default()
    };

    if vms.is_empty() {
        return Json(vec![]);
    }

    let total = vms.len() as f64;
    let running = vms
        .iter()
        .filter(|vm| vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false))
        .count() as f64;
    let availability = if total > 0.0 {
        running / total * 100.0
    } else {
        100.0
    };

    let target = 99.9_f64;
    let budget_total = 100.0 - target;
    let consumed = ((target - availability) / target * 100.0).max(0.0);
    let consumed_percent = consumed.min(budget_total);

    // Burn rate relative to a 30-day window.
    // We observe one snapshot, so we scale: if 100% consumed in the snapshot it
    // would exhaust the budget at a rate proportional to how far below target.
    let window_hours = 30.0 * 24.0;
    let downtime_fraction = (target - availability).max(0.0) / target;
    let burn_rate_1h = downtime_fraction * window_hours;
    let burn_rate_24h = (downtime_fraction * window_hours / 24.0).max(0.0);

    let projected_exhaustion_hours = if burn_rate_1h > 0.0 {
        let remaining = (budget_total - consumed_percent).max(0.0);
        Some((remaining / burn_rate_1h * window_hours * 100.0).round() / 100.0)
    } else {
        None
    };

    let status = if availability >= target {
        "healthy"
    } else if burn_rate_1h > 14.4 {
        "burning_fast"
    } else if availability >= 99.0 {
        "at_risk"
    } else {
        "exhausted"
    };

    Json(vec![SloErrorBudget {
        slo_name: format!("{}-vm-availability", namespace),
        service: "kubevirt-vms".to_string(),
        target,
        current_availability: (availability * 100.0).round() / 100.0,
        budget_total_percent: budget_total,
        budget_consumed_percent: (consumed_percent * 100.0).round() / 100.0,
        burn_rate_1h: (burn_rate_1h * 100.0).round() / 100.0,
        burn_rate_24h: (burn_rate_24h * 100.0).round() / 100.0,
        projected_exhaustion_hours,
        status: status.to_string(),
    }])
}
