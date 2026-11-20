// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! GPU fleet inventory: which nodes advertise GPU device-plugin resources,
//! how they are carved up (whole-GPU passthrough / MIG / vGPU mdev), and
//! whether KubeVirt is configured to hand them to VMs.

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
#[cfg(feature = "web")]
use serde_json::json;

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;
#[cfg(feature = "web")]
use crate::kube::gpu_inventory::{
    gpu_workload_config, gpus_reserved_for_containers, node_gpu_resources,
};

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/gpus", get(list_gpus))
        .with_state(state)
}

/// `GET /api/v1/gpus` — per-node GPU inventory. NFD / GPU-operator labels
/// (`nvidia.com/gpu.product`, `.memory`, `.count`) are surfaced when present
/// so the dashboard can show card models without a guest agent.
#[cfg(feature = "web")]
async fn list_gpus(State(state): State<SharedState>) -> Json<serde_json::Value> {
    use k8s_openapi::api::core::v1::Node;
    use kube::api::Api;

    let client = { state.read().await.kube_client.client().clone() };
    let node_api: Api<Node> = Api::all(client);
    let nodes = match node_api.list(&Default::default()).await {
        Ok(l) => l.items,
        Err(e) => {
            return Json(json!({
                "nodes": [],
                "total_allocatable": 0,
                "error": format!("failed to list nodes: {e}"),
            }));
        }
    };

    let mut total_allocatable: i64 = 0;
    let mut vm_allocatable: i64 = 0;
    let mut out = Vec::new();
    for n in &nodes {
        let resources = node_gpu_resources(n);
        if resources.is_empty() {
            continue;
        }
        let vm_usable = !gpus_reserved_for_containers(n);
        let node_allocatable = resources.iter().map(|r| r.allocatable).sum::<i64>();
        total_allocatable += node_allocatable;
        if vm_usable {
            vm_allocatable += node_allocatable;
        }
        let labels = n.metadata.labels.as_ref();
        let label = |k: &str| labels.and_then(|m| m.get(k)).cloned();
        out.push(json!({
            "node": n.metadata.name.clone().unwrap_or_default(),
            "resources": resources,
            "vm_usable": vm_usable,
            "workload_config": gpu_workload_config(n),
            "labels": {
                "product": label("nvidia.com/gpu.product"),
                "memory": label("nvidia.com/gpu.memory"),
                "count": label("nvidia.com/gpu.count"),
                "gpu_passthrough_role": label("veyron.io/gpu-passthrough"),
            },
        }));
    }

    Json(json!({
        "nodes": out,
        "gpu_node_count": out.len(),
        "total_allocatable": total_allocatable,
        "vm_allocatable": vm_allocatable,
    }))
}
