#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

/// Resource heatmap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceHeatmap {
    pub nodes: Vec<NodeHeatmapEntry>,
    pub timestamp: String,
}

/// Node heatmap entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeatmapEntry {
    pub node_name: String,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub disk_utilization: f64,
    pub network_utilization: f64,
    pub vm_count: u32,
    pub heat_score: f64,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/heatmap/resources", get(get_resource_heatmap))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_resource_heatmap(State(state): State<SharedState>) -> Json<ResourceHeatmap> {
    use crate::kube::types::VirtualMachineInstance;

    let s = state.read().await;
    let nodes = s.client().list_nodes().await.unwrap_or_default();

    // Count VMIs per node
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

    let entries: Vec<NodeHeatmapEntry> = nodes
        .iter()
        .map(|node| {
            let name = node.metadata.name.clone().unwrap_or_default();
            let capacity = node.status.as_ref().and_then(|s| s.capacity.as_ref());
            let allocatable = node.status.as_ref().and_then(|s| s.allocatable.as_ref());

            // Estimate utilization as (capacity - allocatable) / capacity
            let cpu_cap = capacity
                .and_then(|c| c.get("cpu"))
                .and_then(|v| v.0.parse::<f64>().ok())
                .unwrap_or(1.0);
            let cpu_alloc = allocatable
                .and_then(|a| a.get("cpu"))
                .and_then(|v| v.0.parse::<f64>().ok())
                .unwrap_or(cpu_cap);

            let mem_cap = capacity
                .and_then(|c| c.get("memory"))
                .map(|v| crate::utils::parse_memory_bytes(&v.0))
                .unwrap_or(1) as f64;
            let mem_alloc = allocatable
                .and_then(|a| a.get("memory"))
                .map(|v| crate::utils::parse_memory_bytes(&v.0))
                .unwrap_or(mem_cap as u64) as f64;

            let cpu_util = ((cpu_cap - cpu_alloc) / cpu_cap * 100.0).max(0.0);
            let mem_util = ((mem_cap - mem_alloc) / mem_cap * 100.0).max(0.0);
            let vm_count = vmi_per_node.get(&name).copied().unwrap_or(0);

            // Heat score: weighted average of utilization metrics
            let heat_score = cpu_util * 0.4 + mem_util * 0.4 + (vm_count as f64 * 5.0).min(20.0);

            NodeHeatmapEntry {
                node_name: name,
                cpu_utilization: (cpu_util * 10.0).round() / 10.0,
                memory_utilization: (mem_util * 10.0).round() / 10.0,
                disk_utilization: 0.0,
                network_utilization: 0.0,
                vm_count,
                heat_score: (heat_score * 10.0).round() / 10.0,
            }
        })
        .collect();

    Json(ResourceHeatmap {
        nodes: entries,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
