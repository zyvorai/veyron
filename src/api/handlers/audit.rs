#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub user: String,
    pub action: String,
    pub resource_type: String,
    pub resource_name: String,
    pub namespace: String,
    pub outcome: String,
    pub details: Option<String>,
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: u64,
    pub by_action: std::collections::HashMap<String, u64>,
    pub by_user: std::collections::HashMap<String, u64>,
    pub by_outcome: std::collections::HashMap<String, u64>,
    pub period: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/audit/trail", get(list_audit_trail))
        .route("/audit/stats", get(get_audit_stats))
}

#[cfg(feature = "web")]
async fn list_audit_trail() -> impl IntoResponse {
    not_implemented("Audit trail")
}

#[cfg(feature = "web")]
async fn get_audit_stats() -> impl IntoResponse {
    not_implemented("Audit statistics")
}
