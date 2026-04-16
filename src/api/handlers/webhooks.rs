#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/webhooks", get(list_webhooks).post(create_webhook))
}

#[cfg(feature = "web")]
async fn list_webhooks() -> impl IntoResponse {
    not_implemented("Webhooks")
}

#[cfg(feature = "web")]
async fn create_webhook(axum::Json(req): axum::Json<CreateWebhookRequest>) -> impl IntoResponse {
    let _ = req;
    not_implemented("Webhook creation")
}
