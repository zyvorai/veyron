// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope;

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

#[derive(Debug, Clone, Deserialize)]
pub struct AlertQuery {
    pub namespace: Option<String>,
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
async fn list_alerts(
    State(state): State<SharedState>,
    Query(query): Query<AlertQuery>,
) -> Json<Vec<AlertResponse>> {
    let (kube_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(query.namespace.clone(), &default_ns);
    let events = kube_client.list_events_for_scope(&scope).await;

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
    State(state): State<SharedState>,
    Json(req): Json<CreateAlertRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;

    let (raw_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.client(), s.namespace.clone())
    };
    let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(raw_client, &default_ns);

    let cm_name = format!("veyron-alert-{}", req.name.to_lowercase().replace(' ', "-"));
    let mut data = std::collections::BTreeMap::new();
    data.insert("name".to_string(), req.name.clone());
    data.insert("severity".to_string(), req.severity.clone());
    data.insert("metric".to_string(), req.metric.clone());
    data.insert("operator".to_string(), req.operator.clone());
    data.insert("threshold".to_string(), req.threshold.to_string());
    if let Some(ref dur) = req.duration {
        data.insert("duration".to_string(), dur.clone());
    }
    data.insert("status".to_string(), "active".to_string());
    data.insert("created_at".to_string(), chrono::Utc::now().to_rfc3339());

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name),
            namespace: Some(default_ns.clone()),
            labels: Some(
                [("veyron.io/type".to_string(), "alert-rule".to_string())]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    match api.create(&kube::api::PostParams::default(), &cm).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": req.name,
                "severity": req.severity,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "CREATE_FAILED",
                "message": format!("Failed to persist alert rule: {}", e)
            })),
        ),
    }
}

#[cfg(feature = "web")]
async fn resolve_alert(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;

    let (raw_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.client(), s.namespace.clone())
    };
    let now = chrono::Utc::now().to_rfc3339();

    // Persist resolved state in a ConfigMap keyed by alert id
    let cm_name = format!("veyron-resolved-{}", id);
    let mut data = std::collections::BTreeMap::new();
    data.insert("alert_id".to_string(), id.clone());
    data.insert("resolved_at".to_string(), now.clone());
    data.insert("status".to_string(), "resolved".to_string());

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name),
            namespace: Some(default_ns.clone()),
            labels: Some(
                [("veyron.io/type".to_string(), "alert-resolved".to_string())]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(raw_client, &default_ns);
    let _ = api.create(&kube::api::PostParams::default(), &cm).await;

    log::info!("Alert resolved: id={}", id);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "resolved",
            "id": id,
            "resolved_at": now,
        })),
    )
}
