#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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

/// Storage usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub pvc_name: String,
    pub namespace: String,
    pub storage_class: String,
    pub capacity: String,
    pub used: String,
    pub usage_percent: f64,
    pub bound_to_vm: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/storage/pools", get(list_storage_pools))
        .route("/storage/usage", get(get_storage_usage))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_storage_pools(State(state): State<SharedState>) -> Json<Vec<StoragePool>> {
    let s = state.read().await;
    let storage_classes = s.client().list_storage_classes().await.unwrap_or_default();

    // Aggregate PVC capacities per StorageClass across all namespaces
    let all_pvcs = {
        use k8s_openapi::api::core::v1::PersistentVolumeClaim;
        let api: kube::api::Api<PersistentVolumeClaim> = kube::api::Api::all(s.client().client());
        api.list(&kube::api::ListParams::default())
            .await
            .map(|l| l.items)
            .unwrap_or_default()
    };

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

    Json(results)
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
async fn get_storage_usage(State(state): State<SharedState>) -> Json<Vec<StorageUsage>> {
    let s = state.read().await;
    let pvcs = s.client().list_pvcs(&s.namespace).await.unwrap_or_default();

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

            // K8s PVC API does not expose filesystem-level usage (requires kubelet metrics).
            // For Bound PVCs the full capacity is allocated, so we report that as used.
            let (used, usage_percent) = if phase == "Bound" && !capacity.is_empty() {
                (capacity.clone(), 100.0)
            } else {
                (String::new(), 0.0)
            };

            StorageUsage {
                pvc_name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                storage_class,
                capacity,
                used,
                usage_percent,
                bound_to_vm,
            }
        })
        .collect();

    Json(results)
}
