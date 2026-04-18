#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Query, State}, routing::get};
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

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/slo/objectives", get(list_slo_objectives))
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
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.ready)
                .unwrap_or(false)
        })
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
