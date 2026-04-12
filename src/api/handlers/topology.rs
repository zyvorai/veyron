#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Topology map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyMap {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

/// Topology node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Topology edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/topology/map", get(get_topology_map))
}

#[cfg(feature = "web")]
async fn get_topology_map() -> Json<TopologyMap> {
    Json(TopologyMap {
        nodes: vec![],
        edges: vec![],
    })
}
