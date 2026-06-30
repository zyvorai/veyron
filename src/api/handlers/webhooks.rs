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

/// Webhook response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub active: bool,
    pub secret_configured: bool,
    pub last_triggered: Option<String>,
    pub failure_count: u32,
    pub created_at: String,
}

/// Create webhook request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/webhooks", get(list_webhooks).post(create_webhook))
        .with_state(state)
}

/// List configured webhooks from ConfigMaps labeled `veyron.io/type=webhook`.
#[cfg(feature = "web")]
async fn list_webhooks(
    State(state): State<SharedState>,
    Query(query): Query<WebhookQuery>,
) -> Json<Vec<WebhookResponse>> {
    use k8s_openapi::api::core::v1::ConfigMap;

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(s.client().client(), &namespace);
    let params = kube::api::ListParams::default().labels("veyron.io/type=webhook");

    let cms = match api.list(&params).await {
        Ok(list) => list,
        Err(_) => return Json(vec![]),
    };

    let results: Vec<WebhookResponse> = cms
        .items
        .iter()
        .filter_map(|cm| {
            let data = cm.data.as_ref()?;
            Some(WebhookResponse {
                id: cm.metadata.uid.clone().unwrap_or_default(),
                name: data.get("name").cloned().unwrap_or_default(),
                url: data.get("url").cloned().unwrap_or_default(),
                events: data
                    .get("events")
                    .map(|e| e.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                active: data.get("active").map(|v| v == "true").unwrap_or(true),
                secret_configured: data
                    .get("secret_configured")
                    .map(|v| v == "true")
                    .unwrap_or_else(|| data.get("secret").is_some()),
                last_triggered: data.get("last_triggered").cloned(),
                failure_count: data
                    .get("failure_count")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
                created_at: cm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
            })
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn create_webhook(
    State(state): State<SharedState>,
    Query(query): Query<WebhookQuery>,
    Json(req): Json<CreateWebhookRequest>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::ConfigMap;

    // Validate URL
    if !req.url.starts_with("https://") {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "INVALID_URL",
                "message": "Webhook URL must use HTTPS"
            })),
        );
    }

    let s = state.read().await;
    let namespace = query.namespace.unwrap_or_else(|| s.namespace.clone());
    let api: kube::api::Api<ConfigMap> =
        kube::api::Api::namespaced(s.client().client(), &namespace);

    let cm_name = format!("webhook-{}", req.name.to_lowercase().replace(' ', "-"));

    let mut data = std::collections::BTreeMap::new();
    data.insert("name".to_string(), req.name.clone());
    data.insert("url".to_string(), req.url.clone());
    data.insert("events".to_string(), req.events.join(","));
    data.insert("active".to_string(), "true".to_string());

    // Store the HMAC signing secret in a Secret (not the ConfigMap, which is
    // unencrypted at rest and visible to a wider RBAC audience). The ConfigMap
    // only records that a secret is configured and the Secret's name.
    if let Some(ref secret) = req.secret {
        use k8s_openapi::ByteString;
        use k8s_openapi::api::core::v1::Secret;
        let secret_name = format!("{cm_name}-hmac");
        let secret_api: kube::api::Api<Secret> =
            kube::api::Api::namespaced(s.client().client(), &namespace);
        let secret_obj = Secret {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(secret_name.clone()),
                namespace: Some(namespace.clone()),
                labels: Some(
                    [("veyron.io/type".to_string(), "webhook-secret".to_string())]
                        .into_iter()
                        .collect(),
                ),
                ..Default::default()
            },
            data: Some(
                [(
                    "secret".to_string(),
                    ByteString(secret.clone().into_bytes()),
                )]
                .into_iter()
                .collect(),
            ),
            ..Default::default()
        };
        if let Err(e) = secret_api
            .create(&kube::api::PostParams::default(), &secret_obj)
            .await
        {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "WEBHOOK_SECRET_FAILED",
                    "message": format!("Failed to store webhook secret: {e}"),
                })),
            );
        }
        data.insert("secret_configured".to_string(), "true".to_string());
        data.insert("secret_ref".to_string(), secret_name);
    }

    let cm = ConfigMap {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(cm_name),
            namespace: Some(namespace),
            labels: Some(
                [("veyron.io/type".to_string(), "webhook".to_string())]
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
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "name": req.name,
                "url": req.url
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "CREATE_FAILED",
                "message": format!("Failed to create webhook: {}", e)
            })),
        ),
    }
}
