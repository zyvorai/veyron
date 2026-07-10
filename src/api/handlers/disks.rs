// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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
        .route("/vms/:ns/:name/disks/:volume/migrate", post(migrate_disk))
        .route(
            "/disks/conversion/capabilities",
            get(conversion_capabilities),
        )
        .with_state(state)
}

/// Supported disk-image formats and the valid conversion pairs, from the
/// DiskConverter model. Execution (qemu-img) runs out-of-band as a CDI/import
/// Job — this reports what the platform can convert between.
#[cfg(feature = "web")]
async fn conversion_capabilities() -> Json<serde_json::Value> {
    use crate::disk_conversion::{DiskConverter, DiskFormat};
    let dc = DiskConverter::new();
    let formats: Vec<serde_json::Value> = dc
        .supported_formats
        .iter()
        .map(|f| serde_json::json!({ "format": format!("{f:?}").to_lowercase(), "extension": f.extension() }))
        .collect();
    let all = [
        DiskFormat::Raw,
        DiskFormat::Qcow2,
        DiskFormat::Vmdk,
        DiskFormat::Vdi,
        DiskFormat::Vhd,
    ];
    let mut matrix = Vec::new();
    for from in &all {
        let to: Vec<String> = all
            .iter()
            .filter(|t| dc.can_convert(from, t))
            .map(|t| format!("{t:?}").to_lowercase())
            .collect();
        matrix.push(serde_json::json!({ "from": format!("{from:?}").to_lowercase(), "to": to }));
    }
    Json(serde_json::json!({
        "supported_formats": formats,
        "conversions": matrix,
        "note": "execution runs as an out-of-band qemu-img/CDI import Job; this endpoint reports supported format pairs"
    }))
}

#[derive(Debug, Deserialize)]
pub struct MigrateDiskRequest {
    /// Destination StorageClass for the disk.
    pub target_storage_class: String,
    /// Dry run unless explicitly false — reports the plan without changing anything.
    #[serde(default = "default_true")]
    pub dry_run: bool,
}

#[cfg(feature = "web")]
fn default_true() -> bool {
    true
}

/// Migrate a VM disk to a different StorageClass via KubeVirt volume migration:
/// provision a destination PVC on the target class, then patch the VM to point
/// the volume at it with `spec.updateVolumesStrategy: Migration` so KubeVirt
/// copies the data live. Requires the cluster's KubeVirt VolumesUpdateStrategy
/// feature; without it the patch is rejected (surfaced). Dry run by default.
#[cfg(feature = "web")]
async fn migrate_disk(
    State(state): State<SharedState>,
    Path((ns, name, volume)): Path<(String, String, String)>,
    Json(req): Json<MigrateDiskRequest>,
) -> impl IntoResponse {
    use crate::kube::types::VirtualMachine;

    if req.target_storage_class.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "target_storage_class is required".to_string(),
        )
            .into_response();
    }
    let kube = { state.read().await.kube_client.clone() };
    let vm = match kube.get_vm(&ns, &name).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                format!("VM {ns}/{name} not found: {e}"),
            )
                .into_response();
        }
    };
    let mut volumes = vm.spec.template.spec.volumes.clone().unwrap_or_default();
    let Some(idx) = volumes.iter().position(|v| v.name == volume) else {
        return (
            StatusCode::NOT_FOUND,
            format!("volume '{volume}' not found on VM"),
        )
            .into_response();
    };
    let Some(src_claim) = volumes[idx]
        .persistent_volume_claim
        .as_ref()
        .map(|p| p.claim_name.clone())
    else {
        return (
            StatusCode::BAD_REQUEST,
            format!("volume '{volume}' is not PVC-backed (only PVC volumes can be storage-class migrated)"),
        )
            .into_response();
    };

    // Size the destination PVC to the source PVC's request.
    let src_api: Api<PersistentVolumeClaim> = Api::namespaced(kube.client(), &ns);
    let size = match src_api.get(&src_claim).await {
        Ok(p) => p
            .spec
            .and_then(|s| s.resources)
            .and_then(|r| r.requests)
            .and_then(|m| m.get("storage").map(|q| q.0.clone()))
            .unwrap_or_else(|| "10Gi".to_string()),
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("read source PVC {src_claim}: {e}"),
            )
                .into_response();
        }
    };
    let dest_claim = format!(
        "{src_claim}-mig-{}",
        req.target_storage_class.replace([':', '/'], "-")
    );

    if req.dry_run {
        return Json(serde_json::json!({
            "dry_run": true, "vm": name, "volume": volume,
            "source_pvc": src_claim, "size": size,
            "target_storage_class": req.target_storage_class, "destination_pvc": dest_claim,
            "note": "pass dry_run=false to provision the destination PVC and start KubeVirt volume migration"
        }))
        .into_response();
    }

    // 1) Provision the destination PVC on the target class.
    if let Err(e) = kube
        .create_pvc(&ns, &dest_claim, &size, Some(&req.target_storage_class))
        .await
    {
        return (
            StatusCode::BAD_GATEWAY,
            format!("create destination PVC: {e}"),
        )
            .into_response();
    }
    // 2) Point the volume at the new PVC and request a live volume migration.
    if let Some(pvc) = volumes[idx].persistent_volume_claim.as_mut() {
        pvc.claim_name = dest_claim.clone();
    }
    let volumes_json = match serde_json::to_value(&volumes) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("serialize volumes: {e}"),
            )
                .into_response();
        }
    };
    let patch = serde_json::json!({
        "spec": { "updateVolumesStrategy": "Migration", "template": { "spec": { "volumes": volumes_json } } }
    });
    let vms: Api<VirtualMachine> = Api::namespaced(kube.client(), &ns);
    match vms
        .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "ok": true, "vm": name, "volume": volume,
            "from_pvc": src_claim, "to_pvc": dest_claim,
            "target_storage_class": req.target_storage_class,
            "note": "destination PVC created; KubeVirt is migrating the volume live"
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("volume migration patch failed (needs KubeVirt VolumesUpdateStrategy): {e}"),
        )
            .into_response(),
    }
}

#[cfg(feature = "web")]
async fn list_disks(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<Vec<DiskResponse>> {
    let (kube_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let pvcs = kube_client.list_pvcs_for_scope(&scope).await;

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
