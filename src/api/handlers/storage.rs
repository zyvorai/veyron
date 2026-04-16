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

    let results: Vec<StoragePool> = storage_classes
        .iter()
        .map(|sc| {
            let meta = &sc.metadata;
            StoragePool {
                name: meta.name.clone().unwrap_or_default(),
                storage_class: meta.name.clone().unwrap_or_default(),
                provisioner: sc.provisioner.clone(),
                total_capacity: String::new(),
                used_capacity: String::new(),
                available_capacity: String::new(),
                volume_count: 0,
            }
        })
        .collect();

    Json(results)
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

            let bound_to_vm = meta
                .labels
                .as_ref()
                .and_then(|l| {
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
