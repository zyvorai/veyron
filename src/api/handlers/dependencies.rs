// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/dependencies/graph", get(get_dependency_graph))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_dependency_graph(State(state): State<SharedState>) -> Json<DependencyGraph> {
    let s = state.read().await;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // List all VMs
    if let Ok(vms) = s.client().list_all_vms().await {
        for vm in &vms {
            let name = vm.metadata.name.clone().unwrap_or_default();
            let namespace = vm.metadata.namespace.clone().unwrap_or_default();

            nodes.push(DependencyNode {
                id: format!("{}/{}", namespace, name),
                name: name.clone(),
                resource_type: "VirtualMachine".to_string(),
                namespace: namespace.clone(),
            });

            // Check for vmrogue.io/depends-on annotation
            if let Some(annotations) = &vm.metadata.annotations {
                if let Some(depends_on) = annotations.get("vmrogue.io/depends-on") {
                    // depends-on can be comma-separated list of VM names
                    for dep in depends_on.split(',') {
                        let dep = dep.trim();
                        if !dep.is_empty() {
                            let dep_id = if dep.contains('/') {
                                dep.to_string()
                            } else {
                                format!("{}/{}", namespace, dep)
                            };
                            edges.push(DependencyEdge {
                                from: format!("{}/{}", namespace, name),
                                to: dep_id,
                                dependency_type: "depends_on".to_string(),
                            });
                        }
                    }
                }
            }

            // Also check labels for dependency info
            if let Some(labels) = &vm.metadata.labels {
                if let Some(depends_on) = labels.get("vmrogue.io/depends-on") {
                    for dep in depends_on.split(',') {
                        let dep = dep.trim();
                        if !dep.is_empty() {
                            let dep_id = if dep.contains('/') {
                                dep.to_string()
                            } else {
                                format!("{}/{}", namespace, dep)
                            };
                            // Avoid duplicate edges
                            let edge = DependencyEdge {
                                from: format!("{}/{}", namespace, name),
                                to: dep_id,
                                dependency_type: "depends_on".to_string(),
                            };
                            if !edges.iter().any(|e| e.from == edge.from && e.to == edge.to) {
                                edges.push(edge);
                            }
                        }
                    }
                }
            }
        }
    }

    Json(DependencyGraph { nodes, edges })
}
