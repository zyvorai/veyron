#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Node response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub cpu_capacity: String,
    pub memory_capacity: String,
    pub cpu_allocatable: String,
    pub memory_allocatable: String,
    pub kubelet_version: String,
    pub os_image: String,
    pub kernel_version: String,
    pub vm_count: u32,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/{name}", get(get_node))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn list_nodes(State(state): State<SharedState>) -> Json<Vec<NodeResponse>> {
    use crate::kube::types::VirtualMachineInstance;

    let s = state.read().await;
    let nodes = s.client().list_nodes().await.unwrap_or_default();

    // Count running VMs per node via cluster-wide VMI listing
    let mut vmi_per_node: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let vmis_api: kube::api::Api<VirtualMachineInstance> = kube::api::Api::all(s.client().client());
    if let Ok(vmi_list) = vmis_api.list(&kube::api::ListParams::default()).await {
        for vmi in &vmi_list.items {
            if let Some(ref status) = vmi.status {
                if let Some(ref node) = status.node_name {
                    *vmi_per_node.entry(node.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let results: Vec<NodeResponse> = nodes
        .iter()
        .map(|node| {
            let name = node.metadata.name.clone().unwrap_or_default();
            let status = node
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
                .map(|c| {
                    if c.status == "True" {
                        "Ready"
                    } else {
                        "NotReady"
                    }
                })
                .unwrap_or("Unknown")
                .to_string();

            let roles: Vec<String> = node
                .metadata
                .labels
                .as_ref()
                .map(|labels| {
                    labels
                        .keys()
                        .filter_map(|k| {
                            k.strip_prefix("node-role.kubernetes.io/")
                                .map(|r| r.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();

            let capacity = node.status.as_ref().and_then(|s| s.capacity.as_ref());
            let allocatable = node.status.as_ref().and_then(|s| s.allocatable.as_ref());

            let cpu_capacity = capacity
                .and_then(|c| c.get("cpu"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let memory_capacity = capacity
                .and_then(|c| c.get("memory"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let cpu_allocatable = allocatable
                .and_then(|a| a.get("cpu"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let memory_allocatable = allocatable
                .and_then(|a| a.get("memory"))
                .map(|v| v.0.clone())
                .unwrap_or_default();

            let node_info = node.status.as_ref().and_then(|s| s.node_info.as_ref());
            let kubelet_version = node_info
                .map(|i| i.kubelet_version.clone())
                .unwrap_or_default();
            let os_image = node_info.map(|i| i.os_image.clone()).unwrap_or_default();
            let kernel_version = node_info
                .map(|i| i.kernel_version.clone())
                .unwrap_or_default();

            let vm_count = vmi_per_node.get(name.as_str()).copied().unwrap_or(0);

            NodeResponse {
                name,
                status,
                roles,
                cpu_capacity,
                memory_capacity,
                cpu_allocatable,
                memory_allocatable,
                kubelet_version,
                os_image,
                kernel_version,
                vm_count,
            }
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_node(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<Option<NodeResponse>> {
    let nodes = list_nodes(State(state)).await.0;
    Json(nodes.into_iter().find(|n| n.name == name))
}
