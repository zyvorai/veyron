#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Ingress response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressResponse {
    pub name: String,
    pub namespace: String,
    pub class_name: Option<String>,
    pub hosts: Vec<String>,
    pub tls: bool,
    pub rules: Vec<IngressRule>,
    pub created_at: String,
}

/// Ingress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    pub host: String,
    pub path: String,
    pub service_name: String,
    pub service_port: u16,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/ingress", get(list_ingress))
}

#[cfg(feature = "web")]
async fn list_ingress() -> Json<Vec<IngressResponse>> {
    Json(vec![])
}
