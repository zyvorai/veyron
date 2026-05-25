//! Disaster recovery API — snapshot-based failover within namespace.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrFailoverRequest {
    pub namespace: String,
    pub vm_name: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrFailoverResponse {
    pub status: String,
    pub snapshot_name: Option<String>,
    pub message: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/dr/failover", post(dr_failover))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn dr_failover(
    State(_state): State<SharedState>,
    Json(req): Json<DrFailoverRequest>,
) -> Result<Json<DrFailoverResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.namespace.is_empty() || req.vm_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "namespace and vm_name required" })),
        ));
    }

    let manager = crate::snapshots::SnapshotManager::new(&req.namespace)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let snapshots = manager
        .list_snapshots_for_vm(&req.vm_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    if snapshots.is_empty() {
        return Ok(Json(DrFailoverResponse {
            status: "no_snapshots".to_string(),
            snapshot_name: None,
            message: format!("No snapshots for VM {} in {}", req.vm_name, req.namespace),
        }));
    }

    let mut sorted = snapshots;
    sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let latest = &sorted[0];

    if !latest.ready_to_use {
        return Ok(Json(DrFailoverResponse {
            status: "not_ready".to_string(),
            snapshot_name: Some(latest.name.clone()),
            message: "Latest snapshot is not ready to use".to_string(),
        }));
    }

    if req.dry_run {
        return Ok(Json(DrFailoverResponse {
            status: "dry_run".to_string(),
            snapshot_name: Some(latest.name.clone()),
            message: format!(
                "Would restore VM {} from snapshot {}",
                req.vm_name, latest.name
            ),
        }));
    }

    let restore_mgr = crate::snapshots::RestoreManager::new(&req.namespace)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    restore_mgr
        .restore_in_place(&req.vm_name, &latest.name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(DrFailoverResponse {
        status: "restored".to_string(),
        snapshot_name: Some(latest.name.clone()),
        message: format!(
            "Restore initiated for VM {} from snapshot {}",
            req.vm_name, latest.name
        ),
    }))
}
