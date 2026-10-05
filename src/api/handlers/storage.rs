// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

#[cfg(feature = "web")]
use super::namespace_scope::{self, DashboardNamespaceQuery};

use super::feature_context::VmrogueFeatureContext;

/// Storage pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePool {
    pub name: String,
    pub storage_class: String,
    pub provisioner: String,
    pub total_capacity: String,
    pub used_capacity: String,
    pub available_capacity: String,
    pub volume_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePoolsResponse {
    pub veyron_context: VmrogueFeatureContext,
    pub pools: Vec<StoragePool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsageResponse {
    pub veyron_context: VmrogueFeatureContext,
    /// When set, PVC `used` / `usage_percent` may include kubelet volume stats from this query API.
    pub prometheus_query_url: Option<String>,
    pub usage: Vec<StorageUsage>,
}

/// Storage usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub pvc_name: String,
    pub namespace: String,
    pub storage_class: String,
    pub capacity: String,
    pub used: String,
    pub usage_percent: f64,
    /// How `used` / `usage_percent` were derived (`pvc_allocated`, `kubelet_volume_stats`, or `unknown`).
    pub usage_basis: String,
    /// Minimum reported free capacity for this PVC's StorageClass from `CSIStorageCapacity` (cluster-wide hint), if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csi_free_min_for_class: Option<String>,
    pub bound_to_vm: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "web", derive(Default))]
pub struct PvcResizeRequest {
    pub new_size: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePvcRequest {
    pub name: String,
    #[serde(default = "default_pvc_ns")]
    pub namespace: String,
    pub size: String,
    #[serde(default)]
    pub storage_class: Option<String>,
}

fn default_pvc_ns() -> String {
    "default".to_string()
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/storage/pools", get(list_storage_pools))
        .route("/storage/usage", get(get_storage_usage))
        .route("/storage/pvcs", post(create_pvc))
        .route(
            "/storage/pvcs/:ns/:name",
            patch(resize_pvc).delete(delete_pvc),
        )
        .route(
            "/storage/orphans",
            get(list_orphan_pvcs).delete(reclaim_orphan_pvcs),
        )
        .with_state(state)
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct OrphanQuery {
    #[serde(default)]
    pub namespace: Option<String>,
    /// Must be `true` for DELETE to actually delete; otherwise it's a dry run.
    #[serde(default)]
    pub confirm: bool,
}

#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct OrphanPvc {
    pub namespace: String,
    pub name: String,
    pub storage_class: Option<String>,
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Orphan = a Bound PVC that no VM references AND that has no ownerReferences
/// (so CDI/operator/snapshot-managed PVCs are never touched). Returns the
/// candidate list without deleting.
#[cfg(feature = "web")]
async fn find_orphan_pvcs(
    kube: &crate::kube::KubeClient,
    scope: &str,
) -> Vec<k8s_openapi::api::core::v1::PersistentVolumeClaim> {
    use std::collections::HashSet;
    let vms = kube.list_vms_for_scope(scope).await;
    let mut in_use: HashSet<String> = HashSet::new();
    for vm in &vms {
        let ns = vm.metadata.namespace.clone().unwrap_or_default();
        if let Some(vols) = vm.spec.template.spec.volumes.as_ref() {
            for v in vols {
                if let Some(pvc) = v.persistent_volume_claim.as_ref() {
                    in_use.insert(format!("{ns}/{}", pvc.claim_name));
                } else if let Some(dv) = v.data_volume.as_ref() {
                    in_use.insert(format!("{ns}/{}", dv.name));
                }
            }
        }
    }
    kube.list_pvcs_for_scope(scope)
        .await
        .into_iter()
        .filter(|p| {
            let ns = p.metadata.namespace.clone().unwrap_or_default();
            let name = p.metadata.name.clone().unwrap_or_default();
            let bound = p
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .map(|ph| ph == "Bound")
                .unwrap_or(false);
            let owned = p
                .metadata
                .owner_references
                .as_ref()
                .map(|o| !o.is_empty())
                .unwrap_or(false);
            bound && !owned && !in_use.contains(&format!("{ns}/{name}"))
        })
        .collect()
}

#[cfg(feature = "web")]
fn orphan_to_dto(p: &k8s_openapi::api::core::v1::PersistentVolumeClaim) -> OrphanPvc {
    OrphanPvc {
        namespace: p.metadata.namespace.clone().unwrap_or_default(),
        name: p.metadata.name.clone().unwrap_or_default(),
        storage_class: p.spec.as_ref().and_then(|s| s.storage_class_name.clone()),
        capacity: p
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|q| q.0.clone()),
        deleted: None,
        error: None,
    }
}

#[cfg(feature = "web")]
async fn list_orphan_pvcs(
    State(state): State<SharedState>,
    Query(q): Query<OrphanQuery>,
) -> impl IntoResponse {
    let (kube, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let orphans: Vec<OrphanPvc> = find_orphan_pvcs(&kube, &scope)
        .await
        .iter()
        .map(orphan_to_dto)
        .collect();
    Json(serde_json::json!({ "count": orphans.len(), "orphans": orphans }))
}

/// Reclaim orphan PVCs. Dry run unless `?confirm=true`.
#[cfg(feature = "web")]
async fn reclaim_orphan_pvcs(
    State(state): State<SharedState>,
    Query(q): Query<OrphanQuery>,
) -> impl IntoResponse {
    let (kube, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let candidates = find_orphan_pvcs(&kube, &scope).await;
    if !q.confirm {
        let dto: Vec<OrphanPvc> = candidates.iter().map(orphan_to_dto).collect();
        return Json(serde_json::json!({
            "dry_run": true, "count": dto.len(), "orphans": dto,
            "note": "pass ?confirm=true to delete"
        }));
    }
    let mut results = Vec::with_capacity(candidates.len());
    for p in &candidates {
        let mut dto = orphan_to_dto(p);
        match kube.delete_pvc(&dto.namespace, &dto.name).await {
            Ok(()) => dto.deleted = Some(true),
            Err(e) => {
                dto.deleted = Some(false);
                dto.error = Some(e.to_string());
            }
        }
        results.push(dto);
    }
    let deleted = results.iter().filter(|r| r.deleted == Some(true)).count();
    Json(serde_json::json!({
        "dry_run": false, "count": results.len(), "deleted": deleted, "orphans": results
    }))
}

#[cfg(feature = "web")]
async fn list_storage_pools(
    State(state): State<SharedState>,
) -> Result<Json<StoragePoolsResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let kube_client = {
        let s = state.read().await;
        s.kube_client.clone()
    };
    let storage_classes = kube_client
        .list_storage_classes()
        .await
        .map_err(crate::api::handlers::kube_list_error("storage classes"))?;

    // Aggregate PVC capacities per StorageClass across all namespaces
    let all_pvcs = kube_client
        .list_all_pvcs()
        .await
        .map_err(crate::api::handlers::kube_list_error("PVCs"))?;

    // Build map: storage_class_name → (total_bytes, volume_count)
    let mut sc_stats: std::collections::HashMap<String, (u64, u32)> =
        std::collections::HashMap::new();
    for pvc in &all_pvcs {
        let sc = pvc
            .spec
            .as_ref()
            .and_then(|s| s.storage_class_name.as_deref())
            .unwrap_or("")
            .to_string();
        let cap = pvc
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|q| crate::utils::parse_memory_bytes(&q.0))
            .unwrap_or(0);
        let entry = sc_stats.entry(sc).or_insert((0, 0));
        entry.0 += cap;
        entry.1 += 1;
    }

    let csi_free = csi_free_by_storage_class(&kube_client.client()).await;

    let results: Vec<StoragePool> = storage_classes
        .iter()
        .map(|sc| {
            let meta = &sc.metadata;
            let sc_name = meta.name.clone().unwrap_or_default();
            let (total_bytes, volume_count) = sc_stats.get(&sc_name).copied().unwrap_or((0, 0));
            // CSIStorageCapacity is only published by drivers with storageCapacity: true.
            let available = csi_free
                .get(&sc_name)
                .map(|(_, sum)| format_bytes(*sum))
                .unwrap_or_else(|| "N/A".to_string());
            let total_str = format_bytes(total_bytes);
            StoragePool {
                name: sc_name.clone(),
                storage_class: sc_name,
                provisioner: sc.provisioner.clone(),
                total_capacity: total_str.clone(),
                used_capacity: total_str, // PVC capacity = allocated/used
                available_capacity: available,
                volume_count,
            }
        })
        .collect();

    Ok(Json(StoragePoolsResponse {
        veyron_context: VmrogueFeatureContext::storage_pools(),
        pools: results,
    }))
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0".to_string();
    }
    if bytes >= 1 << 40 {
        format!("{:.1} TiB", bytes as f64 / (1u64 << 40) as f64)
    } else if bytes >= 1 << 30 {
        format!("{:.1} GiB", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.0} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else {
        format!("{} KiB", bytes / 1024)
    }
}

#[cfg(feature = "web")]
async fn csi_min_free_by_storage_class(
    client: &kube::Client,
) -> std::collections::HashMap<String, u64> {
    csi_free_by_storage_class(client)
        .await
        .into_iter()
        .map(|(sc, (min, _))| (sc, min))
        .collect()
}

/// Free capacity per StorageClass from `CSIStorageCapacity` as `(min per topology
/// segment, sum across segments)`. Empty when the CSI driver doesn't publish capacity.
#[cfg(feature = "web")]
async fn csi_free_by_storage_class(
    client: &kube::Client,
) -> std::collections::HashMap<String, (u64, u64)> {
    use k8s_openapi::api::storage::v1::CSIStorageCapacity;
    use kube::api::ListParams;

    let api: kube::Api<CSIStorageCapacity> = kube::Api::all(client.clone());
    let Ok(list) = api.list(&ListParams::default().limit(5000)).await else {
        return std::collections::HashMap::new();
    };

    fold_csi_capacity(list.items.iter().map(|item| {
        let free = item
            .capacity
            .as_ref()
            .map(|q| crate::utils::parse_memory_bytes(&q.0))
            .unwrap_or(0);
        (item.storage_class_name.as_str(), free)
    }))
}

/// `(storage_class, free_bytes)` per topology segment → `(min, sum)` per class.
fn fold_csi_capacity<'a>(
    items: impl Iterator<Item = (&'a str, u64)>,
) -> std::collections::HashMap<String, (u64, u64)> {
    let mut m: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();
    for (sc, free) in items {
        if sc.is_empty() || free == 0 {
            continue;
        }
        m.entry(sc.to_string())
            .and_modify(|e| *e = (e.0.min(free), e.1 + free))
            .or_insert((free, free));
    }
    m
}

#[cfg(feature = "web")]
async fn get_storage_usage(
    State(state): State<SharedState>,
    Query(q): Query<DashboardNamespaceQuery>,
) -> Json<StorageUsageResponse> {
    let (kube_client, default_ns) = {
        let s = state.read().await;
        (s.kube_client.clone(), s.namespace.clone())
    };
    let scope = namespace_scope::resolve_opt(q.namespace.clone(), &default_ns);
    let pvcs = kube_client.list_pvcs_for_scope(&scope).await;
    let kc = kube_client.client();

    let prom_url = std::env::var("VEYRON_PROMETHEUS_URL").ok();
    let pvc_used_bytes = if let Some(ref base) = prom_url {
        match crate::api::prometheus::instant_query_vector(base, "kubelet_volume_stats_used_bytes")
            .await
        {
            Ok(rows) => Some(crate::api::prometheus::index_pvc_used_bytes(rows)),
            Err(e) => {
                log::warn!("Prometheus PVC usage query failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    let csi_free = csi_min_free_by_storage_class(&kc).await;

    let results: Vec<StorageUsage> = pvcs
        .iter()
        .map(|pvc| {
            let meta = &pvc.metadata;
            let spec = pvc.spec.as_ref();
            let status = pvc.status.as_ref();

            let storage_class = spec
                .and_then(|s| s.storage_class_name.clone())
                .unwrap_or_default();

            let capacity = status
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("storage"))
                .map(|q| q.0.clone())
                .unwrap_or_default();

            let bound_to_vm = meta.labels.as_ref().and_then(|l| {
                l.get("kubevirt.io/vm")
                    .or_else(|| l.get("kubevirt.io/created-by"))
                    .cloned()
            });

            let phase = status.and_then(|s| s.phase.as_deref()).unwrap_or("");
            let ns = meta.namespace.clone().unwrap_or_default();
            let pname = meta.name.clone().unwrap_or_default();
            let cap_bytes = if capacity.is_empty() {
                0u64
            } else {
                crate::utils::parse_memory_bytes(&capacity)
            };

            let key = (ns.clone(), pname.clone());
            let prom_used = pvc_used_bytes.as_ref().and_then(|m| m.get(&key).copied());

            let (used, usage_percent, usage_basis) = if let Some(u) = prom_used {
                let pct = if cap_bytes > 0 {
                    ((u as f64) / (cap_bytes as f64)) * 100.0_f64
                } else {
                    0.0
                };
                (
                    format_bytes(u),
                    pct.min(100.0),
                    "kubelet_volume_stats".to_string(),
                )
            } else if phase == "Bound" && !capacity.is_empty() && cap_bytes > 0 {
                (capacity.clone(), 100.0, "pvc_allocated".to_string())
            } else {
                (String::new(), 0.0, "unknown".to_string())
            };

            let csi_free_min_for_class = if !storage_class.is_empty() {
                csi_free.get(&storage_class).copied().map(format_bytes)
            } else {
                None
            };

            StorageUsage {
                pvc_name: pname,
                namespace: ns,
                storage_class,
                capacity,
                used,
                usage_percent,
                usage_basis,
                csi_free_min_for_class,
                bound_to_vm,
            }
        })
        .collect();

    Json(StorageUsageResponse {
        veyron_context: VmrogueFeatureContext::storage_usage(),
        prometheus_query_url: prom_url,
        usage: results,
    })
}

#[cfg(feature = "web")]
async fn resize_pvc(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
    Json(req): Json<PvcResizeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    use k8s_openapi::api::core::v1::PersistentVolumeClaim;
    use kube::api::{Api, Patch, PatchParams};

    if req.new_size.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "new_size is required" })),
        );
    }
    let client = {
        let s = state.read().await;
        s.kube_client.client()
    };
    let api: Api<PersistentVolumeClaim> = Api::namespaced(client, &ns);
    let patch = serde_json::json!({
        "spec": { "resources": { "requests": { "storage": req.new_size } } }
    });
    match api
        .patch(&name, &PatchParams::default(), &Patch::Merge(patch))
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(
                serde_json::json!({ "status": "resized", "pvc": name, "namespace": ns, "new_size": req.new_size }),
            ),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[cfg(feature = "web")]
async fn create_pvc(
    State(state): State<SharedState>,
    Json(req): Json<CreatePvcRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if req.name.is_empty() || req.size.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name and size are required" })),
        );
    }
    let kube = { state.read().await.kube_client.clone() };
    match kube
        .create_pvc(
            &req.namespace,
            &req.name,
            &req.size,
            req.storage_class.as_deref(),
        )
        .await
    {
        Ok(pvc) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "pvc": pvc.metadata.name,
                "namespace": req.namespace,
                "size": req.size,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[cfg(feature = "web")]
async fn delete_pvc(
    State(state): State<SharedState>,
    Path((ns, name)): Path<(String, String)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let kube = { state.read().await.kube_client.clone() };
    match kube.delete_pvc(&ns, &name).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "deleted", "pvc": name, "namespace": ns })),
        ),
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("NotFound") || msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (code, Json(serde_json::json!({ "error": msg })))
        }
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn csi_capacity_folds_min_and_sum_per_class() {
        let m = fold_csi_capacity(
            [
                ("local", 10u64),
                ("local", 30),
                ("ceph", 100),
                ("", 5),
                ("ceph", 0),
            ]
            .into_iter(),
        );
        assert_eq!(m.get("local"), Some(&(10, 40)));
        assert_eq!(m.get("ceph"), Some(&(100, 100)));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn format_bytes_picks_binary_units() {
        assert_eq!(format_bytes(0), "0");
        assert_eq!(format_bytes(1 << 40), "1.0 TiB");
    }
}
