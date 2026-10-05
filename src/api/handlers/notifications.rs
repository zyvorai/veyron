// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope;

/// Notification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub title: String,
    pub message: String,
    pub severity: String,
    pub read: bool,
    pub created_at: String,
}

/// Mark notifications read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadRequest {
    pub notification_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/read", post(mark_notifications_read))
        .route("/notifications/test", post(test_notifications))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_notifications(
    State(state): State<SharedState>,
    Query(query): Query<NotificationQuery>,
) -> Json<Vec<NotificationResponse>> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let (kube_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(query.namespace.clone(), &default_ns);
    let events = kube_client.list_events_for_scope(&scope).await;

    // Read-state ConfigMap lives in the API default namespace when viewing cluster-wide.
    let read_cm_ns = if namespace_scope::is_all_namespaces(&scope) {
        default_ns.clone()
    } else {
        scope.clone()
    };

    // Load set of read notification IDs from ConfigMap
    let read_ids: std::collections::HashSet<String> = {
        let api: kube::api::Api<ConfigMap> =
            kube::api::Api::namespaced(kube_client.client(), &read_cm_ns);
        api.get("veyron-notifications-read")
            .await
            .ok()
            .and_then(|cm| cm.data)
            .and_then(|d| d.get("ids").cloned())
            .map(|ids| ids.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    };

    let mut notifications = map_notifications(&events);
    for n in &mut notifications {
        if read_ids.contains(&n.id) {
            n.read = true;
        }
    }

    Json(notifications)
}

#[cfg(feature = "web")]
fn map_notifications(events: &[k8s_openapi::api::core::v1::Event]) -> Vec<NotificationResponse> {
    events
        .iter()
        .filter_map(|event| {
            let event_type = event.type_.as_deref().unwrap_or("Normal");
            let reason = event.reason.as_deref().unwrap_or("");
            let message = event.message.as_deref().unwrap_or("");

            // Only surface Warning/Error events as notifications
            if event_type == "Normal" && !reason.contains("Failed") {
                return None;
            }

            let severity = match event_type {
                "Warning" => "warning",
                _ if reason.contains("Error") || reason.contains("Failed") => "error",
                _ => "info",
            };

            Some(NotificationResponse {
                id: event.metadata.uid.clone().unwrap_or_default(),
                title: reason.to_string(),
                message: message.to_string(),
                severity: severity.to_string(),
                read: false,
                created_at: event
                    .last_timestamp
                    .as_ref()
                    .or(event.metadata.creation_timestamp.as_ref())
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(feature = "web")]
async fn mark_notifications_read(
    State(state): State<SharedState>,
    Json(req): Json<MarkReadRequest>,
) -> Json<serde_json::Value> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let (raw_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.client(), s.namespace.clone())
    };
    let api: kube::api::Api<ConfigMap> = kube::api::Api::namespaced(raw_client, &default_ns);
    let cm_name = "veyron-notifications-read";

    // Merge new IDs with any already stored
    let existing_ids: std::collections::HashSet<String> = api
        .get(cm_name)
        .await
        .ok()
        .and_then(|cm| cm.data)
        .and_then(|d| d.get("ids").cloned())
        .map(|ids| ids.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut all_ids: std::collections::HashSet<String> = existing_ids;
    all_ids.extend(req.notification_ids.iter().cloned());
    let ids_value = all_ids.iter().cloned().collect::<Vec<_>>().join(",");

    let mut data = std::collections::BTreeMap::new();
    data.insert("ids".to_string(), ids_value);

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name.to_string()),
            namespace: Some(default_ns.clone()),
            labels: Some(
                [(
                    "veyron.io/type".to_string(),
                    "notification-state".to_string(),
                )]
                .into_iter()
                .collect(),
            ),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    // Apply (create or update)
    let patch = serde_json::to_value(&cm).unwrap_or_default();
    let _ = api
        .patch(
            cm_name,
            &kube::api::PatchParams::apply("veyron"),
            &kube::api::Patch::Apply(patch),
        )
        .await;

    Json(serde_json::json!({
        "marked_read": req.notification_ids.len(),
        "status": "ok",
    }))
}

#[cfg(feature = "web")]
async fn test_notifications() -> Json<serde_json::Value> {
    let results = crate::notifications::delivery::deliver_test_notification().await;
    Json(serde_json::json!({
        "channels_tested": results.len(),
        "results": results.iter().map(|(name, ok, target)| {
            serde_json::json!({ "channel": name, "success": ok, "target": target })
        }).collect::<Vec<_>>(),
    }))
}
