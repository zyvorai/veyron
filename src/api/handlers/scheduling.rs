// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

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

/// Scheduling status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingStatus {
    pub veyron_context: VmrogueFeatureContext,
    pub pending_pods: u32,
    pub scheduled_pods: u32,
    pub unschedulable_nodes: Vec<String>,
    pub scheduling_latency_ms: f64,
    pub preemptions: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulingQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/scheduling/status", get(get_scheduling_status))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn prometheus_scheduling_latency_ms() -> Option<f64> {
    let base = std::env::var("VEYRON_PROMETHEUS_URL").ok()?;
    let query = "histogram_quantile(0.95, sum(rate(scheduler_scheduling_duration_seconds_bucket[5m])) by (le)) * 1000";
    let samples = crate::api::prometheus::instant_query_vector(&base, query)
        .await
        .ok()?;
    samples.first().map(|(_, v)| *v)
}

#[cfg(feature = "web")]
async fn get_scheduling_status(
    State(state): State<SharedState>,
    Query(query): Query<SchedulingQuery>,
) -> Json<SchedulingStatus> {
    let s = state.read().await;

    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let pods = if namespace == "all" {
        use k8s_openapi::api::core::v1::Pod;
        let api: kube::api::Api<Pod> = kube::api::Api::all(s.client().client());
        api.list(&kube::api::ListParams::default())
            .await
            .map(|list| list.items)
            .unwrap_or_default()
    } else {
        s.client().list_pods(&namespace).await.unwrap_or_default()
    };
    let nodes = s.client().list_nodes().await.unwrap_or_default();
    let events = if namespace == "all" {
        s.client().list_all_events().await.unwrap_or_default()
    } else {
        s.client().list_events(&namespace).await.unwrap_or_default()
    };

    let pending_pods = pods
        .iter()
        .filter(|p| {
            p.status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("")
                == "Pending"
        })
        .count() as u32;

    let scheduled_pods = pods
        .iter()
        .filter(|p| p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_some())
        .count() as u32;

    let unschedulable_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| {
            n.spec
                .as_ref()
                .and_then(|s| s.unschedulable)
                .unwrap_or(false)
        })
        .filter_map(|n| n.metadata.name.clone())
        .collect();

    let preemptions = events
        .iter()
        .filter(|e| {
            e.reason
                .as_deref()
                .map(|r| {
                    r.eq_ignore_ascii_case("Preempting") || r.eq_ignore_ascii_case("Preempted")
                })
                .unwrap_or(false)
        })
        .count() as u32;

    let latency_samples: Vec<f64> = events
        .iter()
        .filter(|e| e.reason.as_deref() == Some("Scheduled"))
        .filter_map(|e| {
            let event_ts = e.first_timestamp.as_ref().or(e.last_timestamp.as_ref())?;
            let obj_create_ts = e.metadata.creation_timestamp.as_ref()?;
            let delta = (event_ts.0 - obj_create_ts.0).num_milliseconds() as f64;
            if delta >= 0.0 { Some(delta) } else { None }
        })
        .collect();
    let mut scheduling_latency_ms = if latency_samples.is_empty() {
        0.0
    } else {
        latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
    };

    let mut ctx = VmrogueFeatureContext::scheduling_events();
    if let Some(prom_latency) = prometheus_scheduling_latency_ms().await {
        scheduling_latency_ms = prom_latency;
        ctx = VmrogueFeatureContext::scheduling_prometheus();
    }

    Json(SchedulingStatus {
        veyron_context: ctx,
        pending_pods,
        scheduled_pods,
        unschedulable_nodes,
        scheduling_latency_ms,
        preemptions,
    })
}
