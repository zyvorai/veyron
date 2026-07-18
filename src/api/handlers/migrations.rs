// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::kube::MigrateVmOptions;

#[cfg(feature = "web")]
use kube::{
    Api,
    api::{DeleteParams, ListParams},
};

#[cfg(feature = "web")]
use crate::kube::types::VirtualMachineInstanceMigration;

/// Migration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResponse {
    pub id: String,
    pub vm_name: String,
    pub source_node: String,
    pub target_node: String,
    pub status: String,
    pub migration_type: String,
    pub progress_percent: u8,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Create migration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub vm_name: String,
    pub target_node: Option<String>,
    pub migration_type: Option<String>,
    /// Bypass the eligibility gate (KubeVirt still enforces its own rules).
    #[serde(default)]
    pub force: bool,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/migrations", get(list_migrations).post(create_migration))
        .route("/migrations/:id", delete(cancel_migration))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_migrations(State(state): State<SharedState>) -> Json<Vec<MigrationResponse>> {
    let s = state.read().await;
    let api: Api<VirtualMachineInstanceMigration> =
        Api::namespaced(s.client().client(), &s.namespace);

    match api.list(&ListParams::default()).await {
        Ok(list) => {
            let results: Vec<MigrationResponse> = list
                .items
                .iter()
                .map(|m| {
                    let meta = &m.metadata;
                    let status = m.status.as_ref();
                    let migration_state = status.and_then(|s| s.migration_state.as_ref());

                    let phase = status
                        .and_then(|s| s.phase.as_deref())
                        .unwrap_or("Unknown")
                        .to_string();

                    let progress = match phase.as_str() {
                        "Succeeded" => 100,
                        "Failed" => 0,
                        "Running" => {
                            // Use target_node_address presence as a proxy for ~75% progress
                            if migration_state
                                .and_then(|ms| ms.target_node_address.as_ref())
                                .is_some()
                            {
                                75
                            } else {
                                25
                            }
                        }
                        "Preparing" | "Scheduling" => 10,
                        _ => 0,
                    };

                    MigrationResponse {
                        id: meta.name.clone().unwrap_or_default(),
                        vm_name: m.spec.vmi_name.clone().unwrap_or_default(),
                        source_node: migration_state
                            .and_then(|ms| ms.source_node.clone())
                            .unwrap_or_default(),
                        target_node: migration_state
                            .and_then(|ms| ms.target_node.clone())
                            .unwrap_or_default(),
                        status: phase,
                        migration_type: "LiveMigration".to_string(),
                        progress_percent: progress,
                        started_at: migration_state.and_then(|ms| ms.start_timestamp.clone()),
                        completed_at: migration_state.and_then(|ms| ms.end_timestamp.clone()),
                    }
                })
                .collect();

            Json(results)
        }
        Err(_) => Json(vec![]),
    }
}

#[cfg(feature = "web")]
async fn create_migration(
    State(state): State<SharedState>,
    Json(req): Json<CreateMigrationRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let s = state.read().await;
    // Eligibility gate: passthrough-GPU / non-migratable VMs get a structured
    // 409 instead of a doomed migration object. Probe failure never blocks.
    if !req.force {
        if let Ok(elig) = s
            .client()
            .migration_eligibility(&s.namespace, &req.vm_name, Default::default())
            .await
        {
            if !elig.eligible {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "MIGRATION_BLOCKED",
                        "blockers": elig.blockers,
                        "warnings": elig.warnings,
                        "suggestion": elig.blockers.iter().find_map(|b| b.suggestion.clone()),
                    })),
                )
                    .into_response();
            }
        }
    }
    // Look up the current node of the VMI before triggering migration
    let source_node = s
        .client()
        .get_vmi(&s.namespace, &req.vm_name)
        .await
        .ok()
        .and_then(|vmi| vmi.status.as_ref().and_then(|st| st.node_name.clone()))
        .unwrap_or_default();

    let mopts = if let Some(ref tn) = req.target_node {
        if !tn.is_empty() {
            Some(MigrateVmOptions {
                added_node_selector: Some(std::collections::BTreeMap::from([(
                    "kubernetes.io/hostname".to_string(),
                    tn.clone(),
                )])),
                priority: None,
            })
        } else {
            None
        }
    } else {
        None
    };

    match s
        .client()
        .migrate_vm(&s.namespace, &req.vm_name, mopts)
        .await
    {
        Ok(()) => Json(MigrationResponse {
            id: format!(
                "{}-migration-{}",
                req.vm_name,
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            ),
            vm_name: req.vm_name,
            source_node,
            target_node: req.target_node.unwrap_or_default(),
            status: "Pending".to_string(),
            migration_type: req
                .migration_type
                .unwrap_or_else(|| "LiveMigration".to_string()),
            progress_percent: 0,
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            completed_at: None,
        })
        .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(feature = "web")]
async fn cancel_migration(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.read().await;
    let api: Api<VirtualMachineInstanceMigration> =
        Api::namespaced(s.client().client(), &s.namespace);

    match api.delete(&id, &DeleteParams::default()).await {
        Ok(_) => Ok(Json(serde_json::json!({"cancelled": id}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
