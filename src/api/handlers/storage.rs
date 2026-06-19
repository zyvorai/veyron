// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
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

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/storage/pools", get(list_storage_pools))
        .route("/storage/usage", get(get_storage_usage))
        .route("/storage/pvcs/:ns/:name", patch(resize_pvc))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_storage_pools(State(state): State<SharedState>) -> Json<StoragePoolsResponse> {
    let kube_client = { let s = state.read().await; s.kube_client.clone() };
    let storage_classes = kube_client.list_storage_classes().await.unwrap_or_default();

    // Aggregate PVC capacities per StorageClass across all namespaces
    let all_pvcs = kube_client.list_all_pvcs().await.unwrap_or_default();

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

    let results: Vec<StoragePool> = storage_classes
        .iter()
        .map(|sc| {
            let meta = &sc.metadata;
            let sc_name = meta.name.clone().unwrap_or_default();
            let (total_bytes, volume_count) = sc_stats.get(&sc_name).copied().unwrap_or((0, 0));
            let total_str = format_bytes(total_bytes);
            StoragePool {
                name: sc_name.clone(),
                storage_class: sc_name,
                provisioner: sc.provisioner.clone(),
                total_capacity: total_str.clone(),
                used_capacity: total_str, // PVC capacity = allocated/used
                available_capacity: "N/A".to_string(), // requires storage-level API
                volume_count,
            }
        })
        .collect();

    Json(StoragePoolsResponse {
        veyron_context: VmrogueFeatureContext::storage_pools(),
        pools: results,
    })
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
    use k8s_openapi::api::storage::v1::CSIStorageCapacity;
    use kube::api::ListParams;

    let api: kube::Api<CSIStorageCapacity> = kube::Api::all(client.clone());
    let Ok(list) = api.list(&ListParams::default().limit(5000)).await else {
        return std::collections::HashMap::new();
    };

    let mut m: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for item in list.items {
        let sc = item.storage_class_name.clone();
        if sc.is_empty() {
            continue;
        }
        let free = item
            .capacity
            .as_ref()
            .map(|q| crate::utils::parse_memory_bytes(&q.0))
            .unwrap_or(0);
        if free == 0 {
            continue;
        }
        m.entry(sc)
            .and_modify(|e| *e = (*e).min(free))
            .or_insert(free);
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
    let client = { let s = state.read().await; s.kube_client.client() };
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
            Json(serde_json::json!({ "status": "resized", "pvc": name, "namespace": ns, "new_size": req.new_size })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}
