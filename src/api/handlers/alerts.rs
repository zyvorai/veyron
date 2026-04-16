#[cfg(feature = "web")]
use axum::{Json, Router, extract::{Path, State}, http::StatusCode, routing::{get, put}};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Alert response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResponse {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub status: String,
    pub message: String,
    pub source: String,
    pub fired_at: String,
    pub resolved_at: Option<String>,
}

/// Create alert request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRequest {
    pub name: String,
    pub severity: String,
    pub metric: String,
    pub operator: String,
    pub threshold: f64,
    pub duration: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/alerts", get(list_alerts).post(create_alert))
        .route("/alerts/{id}/resolve", put(resolve_alert))
        .with_state(state)
}

/// List alerts derived from Warning events in the namespace.
#[cfg(feature = "web")]
async fn list_alerts(State(state): State<SharedState>) -> Json<Vec<AlertResponse>> {
    let s = state.read().await;
    let events = s.client().list_events(&s.namespace).await.unwrap_or_default();

    let results: Vec<AlertResponse> = events
        .iter()
        .filter(|e| e.type_.as_deref() == Some("Warning"))
        .map(|event| {
            let meta = &event.metadata;
            AlertResponse {
                id: meta.uid.clone().unwrap_or_default(),
                name: event.reason.clone().unwrap_or_default(),
                severity: "warning".to_string(),
                status: "firing".to_string(),
                message: event.message.clone().unwrap_or_default(),
                source: event
                    .reporting_component
                    .clone()
                    .unwrap_or_else(|| "kubernetes".to_string()),
                fired_at: event
                    .last_timestamp
                    .as_ref()
                    .or(meta.creation_timestamp.as_ref())
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                resolved_at: None,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn create_alert(
    State(_state): State<SharedState>,
    Json(req): Json<CreateAlertRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Store alert rule as a log entry (full Alertmanager integration is external)
    log::info!(
        "Alert rule created: name={}, severity={}, metric={} {} {}",
        req.name, req.severity, req.metric, req.operator, req.threshold
    );

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "created",
            "name": req.name,
            "severity": req.severity,
            "note": "Alert rule logged. For persistent alert rules, configure Prometheus Alertmanager with the PrometheusRule CRDs in deploy/monitoring/"
        })),
    )
}

#[cfg(feature = "web")]
async fn resolve_alert(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    log::info!("Alert resolved: id={}", id);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "resolved",
            "id": id,
            "note": "Alert marked as resolved. K8s Warning events are immutable; this acknowledges the alert."
        })),
    )
}
