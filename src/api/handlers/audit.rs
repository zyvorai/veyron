#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope;

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

#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/audit/trail", get(list_audit_trail))
        .route("/audit/stats", get(get_audit_stats))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_audit_trail(
    State(state): State<SharedState>,
    Query(query): Query<AuditQuery>,
) -> Json<Vec<AuditEntry>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(query.namespace.clone(), &s.namespace);
    let events = s.client().list_events_for_scope(&scope).await;

    Json(map_audit_entries(&events))
}

#[cfg(feature = "web")]
fn map_audit_entries(events: &[k8s_openapi::api::core::v1::Event]) -> Vec<AuditEntry> {
    events
        .iter()
        .map(|event| {
            let meta = &event.metadata;
            let uid = meta.uid.clone().unwrap_or_default();
            let involved = &event.involved_object;
            let resource_type = involved.kind.clone().unwrap_or_default();
            let resource_name = involved.name.clone().unwrap_or_default();

            AuditEntry {
                id: uid,
                timestamp: event
                    .last_timestamp
                    .as_ref()
                    .or(meta.creation_timestamp.as_ref())
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                user: event
                    .reporting_component
                    .clone()
                    .unwrap_or_else(|| "system".to_string()),
                action: event.reason.clone().unwrap_or_default(),
                resource_type,
                resource_name,
                namespace: meta.namespace.clone().unwrap_or_default(),
                outcome: event.type_.clone().unwrap_or_else(|| "Normal".to_string()),
                details: event.message.clone(),
            }
        })
        .collect()
}

#[cfg(feature = "web")]
async fn get_audit_stats(
    State(state): State<SharedState>,
    Query(query): Query<AuditQuery>,
) -> Json<AuditStats> {
    let entries = list_audit_trail(State(state), Query(query)).await.0;

    let mut by_action: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut by_user: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut by_outcome: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for entry in &entries {
        *by_action.entry(entry.action.clone()).or_insert(0) += 1;
        *by_user.entry(entry.user.clone()).or_insert(0) += 1;
        *by_outcome.entry(entry.outcome.clone()).or_insert(0) += 1;
    }

    Json(AuditStats {
        total_events: entries.len() as u64,
        by_action,
        by_user,
        by_outcome,
        period: "all".to_string(),
    })
}
