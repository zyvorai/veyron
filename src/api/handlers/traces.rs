//! Distributed-tracing style view backed by live Kubernetes Events.
//!
//! When no OpenTelemetry collector is present, we surface recent control-plane
//! activity as trace-shaped rows so the dashboard Traces page is useful.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Deserialize)]
pub struct TraceQuery {
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceRow {
    pub trace_id: String,
    pub service: String,
    pub operation: String,
    pub duration_ms: u64,
    pub span_count: u32,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TracesResponse {
    pub total_traces: u32,
    pub success_rate: f64,
    pub p99_ms: Option<f64>,
    pub errors_1h: u32,
    pub traces: Vec<TraceRow>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/traces", get(list_traces))
        .with_state(state)
}

#[cfg(feature = "web")]
fn event_to_row(e: &k8s_openapi::api::core::v1::Event) -> TraceRow {
    let uid = e
        .metadata
        .uid
        .as_deref()
        .filter(|u| !u.is_empty())
        .unwrap_or("unknown");
    let trace_id: String = uid.chars().take(16).collect();

    let kind = e.involved_object.kind.as_deref().unwrap_or("Resource");
    let name = e.involved_object.name.as_deref().unwrap_or("-");
    let service = format!("{kind}/{name}");

    let operation = e.reason.clone().unwrap_or_else(|| "Event".to_string());
    let span_count = e.count.unwrap_or(1).max(1).min(9999) as u32;

    let is_normal = e.type_.as_deref() == Some("Normal");
    let status = if is_normal { "ok" } else { "error" }.to_string();

    let started_at = e
        .last_timestamp
        .as_ref()
        .map(|t| t.0.to_rfc3339())
        .or_else(|| {
            e.metadata
                .creation_timestamp
                .as_ref()
                .map(|t| t.0.to_rfc3339())
        })
        .unwrap_or_default();

    TraceRow {
        trace_id,
        service,
        operation,
        duration_ms: 1,
        span_count,
        status,
        started_at,
    }
}

#[cfg(feature = "web")]
async fn list_traces(
    State(state): State<SharedState>,
    Query(query): Query<TraceQuery>,
) -> Json<TracesResponse> {
    use chrono::{Duration as ChronoDuration, Utc};

    let s = state.read().await;
    let mut events = match query.namespace.as_deref() {
        Some("all") => s.client().list_all_events().await.unwrap_or_default(),
        Some(ns) => s.client().list_events(ns).await.unwrap_or_default(),
        None => s
            .client()
            .list_events(&s.namespace)
            .await
            .unwrap_or_default(),
    };

    events.sort_by(|a, b| {
        let ta = a
            .last_timestamp
            .as_ref()
            .map(|t| t.0)
            .or_else(|| a.metadata.creation_timestamp.as_ref().map(|t| t.0));
        let tb = b
            .last_timestamp
            .as_ref()
            .map(|t| t.0)
            .or_else(|| b.metadata.creation_timestamp.as_ref().map(|t| t.0));
        tb.cmp(&ta)
    });
    events.truncate(200);

    let one_hour_ago = Utc::now() - ChronoDuration::hours(1);
    let mut errors_1h: u32 = 0;
    for e in &events {
        if e.type_.as_deref() != Some("Normal") {
            let ts = e
                .last_timestamp
                .as_ref()
                .map(|t| t.0)
                .or_else(|| e.metadata.creation_timestamp.as_ref().map(|t| t.0));
            if ts.is_some_and(|t| t >= one_hour_ago) {
                errors_1h += 1;
            }
        }
    }

    let traces: Vec<TraceRow> = events.iter().map(event_to_row).collect();
    let total = traces.len() as u32;
    let ok = traces.iter().filter(|t| t.status == "ok").count() as u32;
    let success_rate = if total > 0 {
        (ok as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    Json(TracesResponse {
        total_traces: total,
        success_rate,
        p99_ms: None,
        errors_1h,
        traces,
    })
}
