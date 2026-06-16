// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Distributed tracing via Jaeger/Tempo query API, with Kubernetes Events fallback.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope;

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
    pub veyron_context: super::feature_context::VmrogueFeatureContext,
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
fn jaeger_query_url() -> Option<String> {
    crate::api::integrations::env_var("VMROGUE_JAEGER_QUERY_URL")
        .or_else(|| crate::api::integrations::env_var("VMROGUE_TEMPO_QUERY_URL"))
}

#[cfg(feature = "web")]
fn traces_context(jaeger: bool) -> super::feature_context::VmrogueFeatureContext {
    if jaeger {
        super::feature_context::VmrogueFeatureContext {
            data_source: "jaeger_tempo_query_api".to_string(),
            scope: "Distributed traces from Jaeger or Tempo query API.".to_string(),
            limitations: "Configure VMROGUE_JAEGER_QUERY_URL (e.g. http://jaeger:16686/api/traces) or VMROGUE_TEMPO_QUERY_URL.".to_string(),
        }
    } else {
        super::feature_context::VmrogueFeatureContext::traces_fallback()
    }
}

#[cfg(feature = "web")]
async fn fetch_jaeger_traces(limit: u32) -> Option<Vec<TraceRow>> {
    let base = jaeger_query_url()?;
    let client = crate::api::integrations::http_client().await.ok()?;
    let url = if base.contains('?') {
        format!("{base}&limit={limit}")
    } else {
        format!("{base}?limit={limit}")
    };
    let body: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let traces = body.get("data")?.as_array()?;

    let mut rows = Vec::new();
    for trace in traces {
        let trace_id = trace
            .get("traceID")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let spans = trace.get("spans").and_then(|v| v.as_array());
        let span_count = spans.map(|s| s.len() as u32).unwrap_or(1);
        let duration_us = trace
            .get("spans")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|sp| sp.get("duration"))
            .and_then(|d| d.as_u64())
            .unwrap_or(0);
        let duration_ms = (duration_us / 1000).max(1);
        let operation = trace
            .get("spans")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|sp| sp.get("operationName"))
            .and_then(|o| o.as_str())
            .unwrap_or("trace")
            .to_string();
        let service = trace
            .get("processes")
            .and_then(|p| p.as_object())
            .and_then(|m| m.values().next())
            .and_then(|proc| proc.get("serviceName"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string();
        let has_error = trace
            .get("spans")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter().any(|sp| {
                    sp.get("tags")
                        .and_then(|t| t.as_array())
                        .map(|tags| {
                            tags.iter().any(|tag| {
                                tag.get("key").and_then(|k| k.as_str()) == Some("error")
                                    && tag.get("value").and_then(|v| v.as_bool()) == Some(true)
                            })
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let started_at = trace
            .get("spans")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|sp| sp.get("startTime"))
            .and_then(|t| t.as_u64())
            .map(|us| {
                chrono::DateTime::from_timestamp_micros(us as i64)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        rows.push(TraceRow {
            trace_id,
            service,
            operation,
            duration_ms,
            span_count,
            status: if has_error { "error" } else { "ok" }.to_string(),
            started_at,
        });
    }
    Some(rows)
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
    let span_count = e.count.unwrap_or(1).clamp(1, 9999) as u32;

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

    if let Some(mut jaeger_rows) = fetch_jaeger_traces(50).await {
        jaeger_rows.truncate(200);
        let total = jaeger_rows.len() as u32;
        let ok = jaeger_rows.iter().filter(|t| t.status == "ok").count() as u32;
        let success_rate = if total > 0 {
            (ok as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        let mut durations: Vec<u64> = jaeger_rows.iter().map(|t| t.duration_ms).collect();
        durations.sort_unstable();
        let p99_ms = durations.last().map(|&d| d as f64);
        let one_hour_ago = Utc::now() - ChronoDuration::hours(1);
        let errors_1h = jaeger_rows
            .iter()
            .filter(|t| t.status != "ok")
            .filter(|t| {
                chrono::DateTime::parse_from_rfc3339(&t.started_at)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc) >= one_hour_ago)
                    .unwrap_or(false)
            })
            .count() as u32;
        return Json(TracesResponse {
            veyron_context: traces_context(true),
            total_traces: total,
            success_rate,
            p99_ms,
            errors_1h,
            traces: jaeger_rows,
        });
    }

    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(query.namespace.clone(), &s.namespace);
    let mut events = s.client().list_events_for_scope(&scope).await;

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
        veyron_context: traces_context(false),
        total_traces: total,
        success_rate,
        p99_ms: None,
        errors_1h,
        traces,
    })
}
