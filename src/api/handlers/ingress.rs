#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

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
    Router::new().route("/ingress", get(list_ingress))
}

#[cfg(feature = "web")]
async fn list_ingress() -> impl IntoResponse {
    not_implemented("Ingress")
}
