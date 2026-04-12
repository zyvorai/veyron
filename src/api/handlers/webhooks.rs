#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
    Router::new()
        .route("/webhooks", get(list_webhooks).post(create_webhook))
}

#[cfg(feature = "web")]
async fn list_webhooks() -> Json<Vec<WebhookResponse>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn create_webhook(Json(req): Json<CreateWebhookRequest>) -> Json<Option<WebhookResponse>> {
    let _ = req;
    Json(None)
}
