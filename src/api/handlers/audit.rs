#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
async fn list_audit_trail() -> Json<Vec<AuditEntry>> {
    Json(vec![])
}

#[cfg(feature = "web")]
async fn get_audit_stats() -> Json<AuditStats> {
    Json(AuditStats {
        total_events: 0,
        by_action: std::collections::HashMap::new(),
        by_user: std::collections::HashMap::new(),
        by_outcome: std::collections::HashMap::new(),
        period: "all".to_string(),
    })
}
