#[cfg(feature = "web")]
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::api::http_server::web::SharedState;

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

#[derive(Debug, Clone, Deserialize)]
pub struct TopologyQuery {
    pub namespace: Option<String>,
}

#[cfg(feature = "web")]
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/topology/map", get(get_topology_map))
        .with_state(state)
}

#[cfg(feature = "web")]
async fn get_topology_map(
    State(state): State<SharedState>,
    Query(query): Query<TopologyQuery>,
) -> Json<TopologyMap> {
    use kube::api::{Api, ListParams};

    let s = state.read().await;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // List cluster nodes
    if let Ok(k8s_nodes) = s.client().list_nodes().await {
        for node in &k8s_nodes {
            let name = node.metadata.name.clone().unwrap_or_default();
            let status = node
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|conds| {
                    conds.iter().find(|c| c.type_ == "Ready").map(|c| {
                        if c.status == "True" {
                            "Ready"
                        } else {
                            "NotReady"
                        }
                    })
                })
                .unwrap_or("Unknown")
                .to_string();

            let mut metadata = std::collections::HashMap::new();
            if let Some(labels) = &node.metadata.labels {
                if let Some(role) = labels.get("kubernetes.io/role") {
                    metadata.insert("role".to_string(), role.clone());
                }
                if let Some(os) = labels.get("kubernetes.io/os") {
                    metadata.insert("os".to_string(), os.clone());
                }
            }

            nodes.push(TopologyNode {
                id: format!("node/{}", name),
                name: name.clone(),
                kind: "Node".to_string(),
                status,
                metadata,
            });
        }
    }

    // List VMIs
    let client = s.client().client();
    let vmi_api: Api<crate::kube::VirtualMachineInstance> = match query.namespace.as_deref() {
        Some("all") => Api::all(client),
        Some(ns) => Api::namespaced(client, ns),
        None => Api::namespaced(client, &s.namespace),
    };

    if let Ok(vmi_list) = vmi_api.list(&ListParams::default()).await {
        let vmis = vmi_list.items;
        for vmi in &vmis {
            let vmi_name = vmi.metadata.name.clone().unwrap_or_default();
            let vmi_status = vmi
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown")
                .to_string();

            let node_name = vmi.status.as_ref().and_then(|s| s.node_name.clone());

            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                "namespace".to_string(),
                vmi.metadata.namespace.clone().unwrap_or_default(),
            );
            if let Some(ref n) = node_name {
                metadata.insert("node".to_string(), n.clone());
            }

            nodes.push(TopologyNode {
                id: format!("vm/{}", vmi_name),
                name: vmi_name.clone(),
                kind: "VirtualMachine".to_string(),
                status: vmi_status,
                metadata,
            });

            // Create edge: VM runs_on Node
            if let Some(ref n) = node_name {
                edges.push(TopologyEdge {
                    source: format!("vm/{}", vmi_name),
                    target: format!("node/{}", n),
                    relation: "runs_on".to_string(),
                });
            }
        }
    }

    Json(TopologyMap { nodes, edges })
}
