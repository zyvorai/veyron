#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

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
pub fn router() -> Router {
    Router::new()
        .route("/heatmap/resources", get(get_resource_heatmap))
}

#[cfg(feature = "web")]
async fn get_resource_heatmap() -> Json<ResourceHeatmap> {
    Json(ResourceHeatmap {
        nodes: vec![],
        timestamp: String::new(),
    })
}
