// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Atlas storage control-plane integration handlers.
//!
//! Drives **Ceph-backed** VM disk snapshot / clone / restore / backup through Atlas
//! (`../atlas`), the Zyvor storage control plane. This is additive: it never touches
//! Veyron's native KubeVirt `VolumeSnapshot` flow under `/api/v1/snapshots`. When
//! `VEYRON_ATLAS_URL` is unset the routes report `configured: false` and write actions
//! return `503`.
//!
//! A VM's disks are mapped to Atlas volumes by matching each KubeVirt PVC/DataVolume
//! claim against Atlas inventory on `(namespace, pvc_name)` — so the caller works in
//! VM terms and Veyron resolves the underlying RBD volume ids.

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::atlas::{ATLAS_PRODUCT, AtlasClient, AtlasJobAck, AtlasVolume, atlas_tenant};
#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::kube::types::VirtualMachine;

// ----------------------------------------------------------------------------
// Response DTOs
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasStatusResponse {
    pub configured: bool,
    pub reachable: bool,
    pub base_url: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ceph_backends: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots_supported: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub backends: Vec<serde_json::Value>,
}

/// One VM disk mapped (or not) to an Atlas volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmDiskMapping {
    pub disk: String,
    pub pvc_name: String,
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<AtlasVolume>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CephSnapshotRequest {
    /// Optional snapshot name prefix; disk name is appended when multiple disks match.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub snapshot_class: Option<String>,
    /// Restrict to a single VM disk (KubeVirt volume name); default: all Ceph-backed disks.
    #[serde(default)]
    pub disk: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CephBackupRequest {
    /// Target Atlas RGW bucket id (must be `bound`). Required.
    pub bucket_id: String,
    /// `manifest` (default) or `data` (real RBD `export-diff` → S3).
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub keep: Option<u32>,
    #[serde(default)]
    pub max_age_secs: Option<u64>,
    #[serde(default)]
    pub disk: Option<String>,
}

/// Result of a per-disk write action (snapshot/backup) across a VM's disks.
#[derive(Debug, Clone, Serialize)]
pub struct DiskJobResult {
    pub disk: String,
    pub pvc_name: String,
    pub volume_id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<AtlasJobAck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ----------------------------------------------------------------------------
// Router
// ----------------------------------------------------------------------------

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/atlas/status", get(get_status))
        .route("/atlas/backends", get(list_backends))
        .route("/atlas/volumes", get(list_volumes))
        .route("/atlas/volumes/:id/snapshot", post(volume_snapshot))
        .route("/atlas/volumes/:id/backup", post(volume_backup))
        .route("/atlas/buckets", get(list_buckets))
        .route("/atlas/vms/:ns/:name/volumes", get(vm_volumes))
        .route("/atlas/vms/:ns/:name/ceph-snapshot", post(vm_ceph_snapshot))
        .route("/atlas/vms/:ns/:name/ceph-backup", post(vm_ceph_backup))
        .route("/atlas/snapshots", get(list_snapshots))
        .route("/atlas/snapshots/:id/restore", post(restore_snapshot))
        .route("/atlas/snapshots/:id/clone", post(clone_snapshot))
        .route("/atlas/snapshots/:id", delete(delete_snapshot))
        .route("/atlas/backups", get(list_backups))
        .route("/atlas/jobs/:id", get(get_job))
        .with_state(state)
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

#[cfg(feature = "web")]
fn client_or_503() -> Result<AtlasClient, (StatusCode, String)> {
    AtlasClient::from_env().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Atlas not configured — set VEYRON_ATLAS_URL".to_string(),
    ))
}

/// Resolve a KubeVirt VM's disks to `(disk_name, pvc_name)` pairs. Covers both
/// `persistentVolumeClaim` volumes and `dataVolume` templates (the DataVolume name
/// is also the bound PVC name).
#[cfg(feature = "web")]
fn vm_pvcs(vm: &VirtualMachine) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(vols) = vm.spec.template.spec.volumes.as_ref() {
        for vol in vols {
            if let Some(pvc) = vol.persistent_volume_claim.as_ref() {
                out.push((vol.name.clone(), pvc.claim_name.clone()));
            } else if let Some(dv) = vol.data_volume.as_ref() {
                out.push((vol.name.clone(), dv.name.clone()));
            }
        }
    }
    out
}

/// Fetch the VM and map each of its disks to the Atlas volume backing it (if any).
#[cfg(feature = "web")]
async fn resolve_vm_disks(
    client: &AtlasClient,
    kube: &crate::kube::KubeClient,
    ns: &str,
    name: &str,
    only_disk: Option<&str>,
) -> Result<Vec<VmDiskMapping>, (StatusCode, String)> {
    let vm = kube.get_vm(ns, name).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            format!("VM {ns}/{name} not found: {e}"),
        )
    })?;
    let mut mappings = Vec::new();
    for (disk, pvc) in vm_pvcs(&vm) {
        if let Some(want) = only_disk {
            if disk != want {
                continue;
            }
        }
        let volume = client
            .find_volume_for_pvc(ns, &pvc)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
        mappings.push(VmDiskMapping {
            disk,
            pvc_name: pvc,
            matched: volume.is_some(),
            volume,
        });
    }
    Ok(mappings)
}

// ----------------------------------------------------------------------------
// Handlers
// ----------------------------------------------------------------------------

#[cfg(feature = "web")]
async fn get_status(State(_state): State<SharedState>) -> Json<AtlasStatusResponse> {
    let Some(client) = AtlasClient::from_env() else {
        return Json(AtlasStatusResponse {
            configured: false,
            reachable: false,
            base_url: None,
            message: Some(
                "Atlas not configured — set VEYRON_ATLAS_URL to enable Ceph-backed VM disk protection"
                    .to_string(),
            ),
            ceph_backends: None,
            snapshots_supported: None,
            backends: Vec::new(),
        });
    };

    let health = client.health().await;
    let reachable = health.is_some();
    let backends = client.list_backends().await.unwrap_or_default();
    let ceph = backends
        .iter()
        .filter(|b| b.backend_type.as_deref() == Some("ceph"))
        .count();
    let snap = backends.iter().any(|b| {
        b.capabilities
            .as_ref()
            .map(|c| c.snapshots)
            .unwrap_or(false)
    });

    Json(AtlasStatusResponse {
        configured: true,
        base_url: Some(client.base().to_string()),
        message: if reachable {
            None
        } else {
            Some("Atlas health probe failed".to_string())
        },
        ceph_backends: Some(ceph),
        snapshots_supported: if backends.is_empty() {
            None
        } else {
            Some(snap)
        },
        backends: backends
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect(),
        reachable,
    })
}

#[cfg(feature = "web")]
async fn list_backends(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.list_backends().await {
        Ok(b) => Json(b).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct VolumeFilterQuery {
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[cfg(feature = "web")]
async fn list_volumes(
    State(_state): State<SharedState>,
    Query(q): Query<VolumeFilterQuery>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let mut filters: Vec<(&str, String)> = Vec::new();
    if let Some(b) = q.backend.filter(|s| !s.trim().is_empty()) {
        filters.push(("backend", b));
    }
    if let Some(k) = q.kind.filter(|s| !s.trim().is_empty()) {
        filters.push(("kind", k));
    }
    if let Some(s) = q.state.filter(|s| !s.trim().is_empty()) {
        filters.push(("state", s));
    }
    match client.list_volumes(&filters).await {
        Ok(mut vols) => {
            // Atlas has no VM-namespace filter for block volumes; apply it here.
            if let Some(ns) = q.namespace.filter(|s| !s.trim().is_empty() && s != "all") {
                vols.retain(|v| v.kubernetes_namespace.as_deref() == Some(ns.as_str()));
            }
            Json(vols).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[cfg(feature = "web")]
async fn list_buckets(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.list_buckets().await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

/// Direct per-volume snapshot (volume-centric UIs). VM-centric callers use the
/// `/atlas/vms/:ns/:name/ceph-snapshot` route instead.
#[cfg(feature = "web")]
async fn volume_snapshot(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<CephSnapshotRequest>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client
        .create_snapshot(&id, req.name.as_deref(), req.snapshot_class.as_deref())
        .await
    {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[cfg(feature = "web")]
async fn volume_backup(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<CephBackupRequest>,
) -> impl IntoResponse {
    if req.bucket_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "bucket_id is required".to_string()).into_response();
    }
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let body = serde_json::json!({
        "volume_id": id,
        "bucket_id": req.bucket_id,
        "mode": req.mode.as_deref().unwrap_or("data"),
        "keep": req.keep.unwrap_or(0),
        "max_age_secs": req.max_age_secs.unwrap_or(0),
    });
    match client.create_backup_job(&body).await {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[cfg(feature = "web")]
async fn vm_volumes(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let kube = { state.read().await.kube_client.clone() };
    match resolve_vm_disks(&client, &kube, &ns, &name, None).await {
        Ok(m) => Json(m).into_response(),
        Err(e) => e.into_response(),
    }
}

#[cfg(feature = "web")]
async fn vm_ceph_snapshot(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<CephSnapshotRequest>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let kube = { state.read().await.kube_client.clone() };
    let mappings = match resolve_vm_disks(&client, &kube, &ns, &name, req.disk.as_deref()).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };
    let matched: Vec<_> = mappings.into_iter().filter(|m| m.matched).collect();
    if matched.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("no Ceph-backed disks found for VM {ns}/{name} in Atlas inventory"),
        )
            .into_response();
    }
    let multi = matched.len() > 1;
    let mut results = Vec::new();
    for m in matched {
        let vol = m.volume.as_ref().unwrap();
        let snap_name = req.name.as_deref().map(|base| {
            if multi {
                format!("{base}-{}", m.disk)
            } else {
                base.to_string()
            }
        });
        let r = client
            .create_snapshot(&vol.id, snap_name.as_deref(), req.snapshot_class.as_deref())
            .await;
        results.push(match r {
            Ok(job) => DiskJobResult {
                disk: m.disk,
                pvc_name: m.pvc_name,
                volume_id: Some(vol.id.clone()),
                ok: true,
                job: Some(job),
                error: None,
            },
            Err(e) => DiskJobResult {
                disk: m.disk,
                pvc_name: m.pvc_name,
                volume_id: Some(vol.id.clone()),
                ok: false,
                job: None,
                error: Some(e),
            },
        });
    }
    (StatusCode::ACCEPTED, Json(results)).into_response()
}

#[cfg(feature = "web")]
async fn vm_ceph_backup(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<CephBackupRequest>,
) -> impl IntoResponse {
    if req.bucket_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "bucket_id is required".to_string()).into_response();
    }
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let kube = { state.read().await.kube_client.clone() };
    let mappings = match resolve_vm_disks(&client, &kube, &ns, &name, req.disk.as_deref()).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };
    let matched: Vec<_> = mappings.into_iter().filter(|m| m.matched).collect();
    if matched.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("no Ceph-backed disks found for VM {ns}/{name} in Atlas inventory"),
        )
            .into_response();
    }
    let mode = req.mode.as_deref().unwrap_or("data");
    let mut results = Vec::new();
    for m in matched {
        let vol = m.volume.as_ref().unwrap();
        let body = serde_json::json!({
            "volume_id": vol.id,
            "bucket_id": req.bucket_id,
            "mode": mode,
            "keep": req.keep.unwrap_or(0),
            "max_age_secs": req.max_age_secs.unwrap_or(0),
        });
        let r = client.create_backup_job(&body).await;
        results.push(match r {
            Ok(job) => DiskJobResult {
                disk: m.disk,
                pvc_name: m.pvc_name,
                volume_id: Some(vol.id.clone()),
                ok: true,
                job: Some(job),
                error: None,
            },
            Err(e) => DiskJobResult {
                disk: m.disk,
                pvc_name: m.pvc_name,
                volume_id: Some(vol.id.clone()),
                ok: false,
                job: None,
                error: Some(e),
            },
        });
    }
    (StatusCode::ACCEPTED, Json(results)).into_response()
}

#[cfg(feature = "web")]
async fn list_snapshots(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.list_snapshots().await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

/// Body for restore/clone; forwarded to Atlas as-is (name/namespace/storage_class/size_bytes).
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotProvisionRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

#[cfg(feature = "web")]
fn provision_body(req: &SnapshotProvisionRequest, role: &str) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    if let Some(n) = req.name.as_ref().filter(|s| !s.trim().is_empty()) {
        m.insert("name".into(), n.clone().into());
    }
    if let Some(ns) = req.namespace.as_ref().filter(|s| !s.trim().is_empty()) {
        m.insert("namespace".into(), ns.clone().into());
    }
    if let Some(sc) = req.storage_class.as_ref().filter(|s| !s.trim().is_empty()) {
        m.insert("storage_class".into(), sc.clone().into());
    }
    if let Some(sz) = req.size_bytes {
        m.insert("size_bytes".into(), sz.into());
    }
    // Record Veyron as the owning product on the provisioned volume. Atlas's Owner
    // struct requires `resource_id` and `role` — a partial owner is rejected with 422,
    // so send a complete binding (resource_id derived from the new volume name).
    let resource_id = req
        .name
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "veyron-managed".to_string());
    m.insert(
        "owner".into(),
        serde_json::json!({
            "product": ATLAS_PRODUCT,
            "resource_type": "virtual_machine",
            "resource_id": resource_id,
            "role": role,
        }),
    );
    m.insert("tenant_id".into(), atlas_tenant().into());
    serde_json::Value::Object(m)
}

#[cfg(feature = "web")]
async fn restore_snapshot(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<SnapshotProvisionRequest>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client
        .restore_snapshot(&id, &provision_body(&req, "restored-disk"))
        .await
    {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[cfg(feature = "web")]
async fn clone_snapshot(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<SnapshotProvisionRequest>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client
        .clone_snapshot(&id, &provision_body(&req, "cloned-disk"))
        .await
    {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ForceQuery {
    #[serde(default)]
    pub force: bool,
}

#[cfg(feature = "web")]
async fn delete_snapshot(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<ForceQuery>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.delete_snapshot(&id, q.force).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct BackupQuery {
    #[serde(default)]
    pub volume_id: Option<String>,
}

#[cfg(feature = "web")]
async fn list_backups(
    State(_state): State<SharedState>,
    Query(q): Query<BackupQuery>,
) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.list_backups(q.volume_id.as_deref()).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[cfg(feature = "web")]
async fn get_job(State(_state): State<SharedState>, Path(id): Path<String>) -> impl IntoResponse {
    let client = match client_or_503() {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.get_job(&id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

/// Unit tests — pure, no gateway needed.
#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    // Regression guard: Atlas's Owner struct requires `resource_id` + `role`; a
    // partial owner was rejected with HTTP 422 on restore/clone (fixed).
    #[test]
    fn provision_body_owner_is_complete() {
        let req = SnapshotProvisionRequest {
            name: Some("restore-42".into()),
            namespace: None,
            storage_class: None,
            size_bytes: None,
        };
        let body = provision_body(&req, "restored-disk");
        let owner = body.get("owner").expect("owner present");
        assert_eq!(
            owner.get("product").and_then(|v| v.as_str()),
            Some(ATLAS_PRODUCT)
        );
        assert_eq!(
            owner.get("resource_type").and_then(|v| v.as_str()),
            Some("virtual_machine")
        );
        assert_eq!(
            owner.get("resource_id").and_then(|v| v.as_str()),
            Some("restore-42")
        );
        assert_eq!(
            owner.get("role").and_then(|v| v.as_str()),
            Some("restored-disk")
        );
        assert!(body.get("tenant_id").is_some());
    }

    #[test]
    fn provision_body_resource_id_falls_back_without_name() {
        let req = SnapshotProvisionRequest {
            name: None,
            namespace: None,
            storage_class: None,
            size_bytes: None,
        };
        let body = provision_body(&req, "cloned-disk");
        assert_eq!(
            body["owner"].get("resource_id").and_then(|v| v.as_str()),
            Some("veyron-managed")
        );
    }
}
