#[cfg(feature = "web")]
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};

/// Dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}

/// Dependency node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub namespace: String,
}

/// Dependency edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub dependency_type: String,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/dependencies/graph", get(get_dependency_graph))
}

#[cfg(feature = "web")]
async fn get_dependency_graph() -> Json<DependencyGraph> {
    Json(DependencyGraph {
        nodes: vec![],
        edges: vec![],
    })
}
