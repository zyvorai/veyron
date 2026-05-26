// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

#[cfg(feature = "web")]
use k8s_openapi::api::core::v1::PersistentVolumeClaim;

#[cfg(feature = "web")]
use kube::{
    Api,
    api::{Patch, PatchParams},
};

/// Disk response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskResponse {
    pub name: String,
    pub vm_name: String,
    pub size: String,
    pub storage_class: Option<String>,
    pub access_mode: String,
    pub status: String,
}

/// Expand disk request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandDiskRequest {
    /// Target namespace for the PVC (defaults to API default namespace).
    #[serde(default)]
    pub namespace: Option<String>,
    pub vm_name: String,
    pub disk_name: String,
    pub new_size: String,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/disks", get(list_disks))
        .route("/disks/expand", post(expand_disk))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_disks(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<DiskResponse>> {
    let s = state.read().await;
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &s.namespace);
    let pvcs = s.client().list_pvcs_for_scope(&scope).await;

    let results: Vec<DiskResponse> = pvcs
        .iter()
        .map(|pvc| {
            let meta = &pvc.metadata;
            let spec = pvc.spec.as_ref();
            let pvc_status = pvc.status.as_ref();

            let vm_name = meta
                .labels
                .as_ref()
                .and_then(|l| l.get("kubevirt.io/vm").cloned())
                .or_else(|| {
                    meta.labels
                        .as_ref()
                        .and_then(|l| l.get("kubevirt.io/created-by").cloned())
                })
                .unwrap_or_default();

            let size = spec
                .and_then(|s| s.resources.as_ref())
                .and_then(|r| r.requests.as_ref())
                .and_then(|req| req.get("storage"))
                .map(|q| q.0.clone())
                .unwrap_or_default();

            let storage_class = spec.and_then(|s| s.storage_class_name.clone());

            let access_mode = spec
                .and_then(|s| s.access_modes.as_ref())
                .and_then(|modes| modes.first())
                .cloned()
                .unwrap_or_else(|| "ReadWriteOnce".to_string());

            let status = pvc_status
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown")
                .to_string();

            DiskResponse {
                name: meta.name.clone().unwrap_or_default(),
                vm_name,
                size,
                storage_class,
                access_mode,
                status,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn expand_disk(
    State(state): State<SharedState>,
    Json(req): Json<ExpandDiskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validate input
    if req.disk_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "disk_name is required"})),
        ));
    }
    // Validate new_size is a valid K8s quantity (e.g. "10Gi", "500Mi")
    let size_bytes = crate::utils::parse_memory_bytes(&req.new_size);
    if size_bytes == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "new_size must be a valid K8s quantity (e.g. '10Gi')"}),
            ),
        ));
    }

    let s = state.read().await;
    let ns = namespace_scope::resolve_opt(req.namespace.clone(), &s.namespace);
    let api: Api<PersistentVolumeClaim> = Api::namespaced(s.client().client(), &ns);

    let patch = serde_json::json!({
        "spec": {
            "resources": {
                "requests": {
                    "storage": req.new_size
                }
            }
        }
    });

    match api
        .patch(
            &req.disk_name,
            &PatchParams::default(),
            &Patch::Merge(&patch),
        )
        .await
    {
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "expanding",
            "disk_name": req.disk_name,
            "new_size": req.new_size
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to expand disk: {}", e)})),
        )),
    }
}
