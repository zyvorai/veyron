// VM Dependency Graph - Relationship mapping between VMs and resources

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
    pub clusters: Vec<DependencyCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub namespace: String,
    pub status: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    VirtualMachine,
    Database,
    Backend,
    Frontend,
    Cache,
    Queue,
    Worker,
    LoadBalancer,
    Storage,
    ConfigMap,
    Secret,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: DependencyType,
    pub strength: DependencyStrength,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    Network,
    Storage,
    Service,
    ConfigMap,
    Secret,
    Database,
    Cache,
    Queue,
    API,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyStrength {
    Critical,
    High,
    Medium,
    Weak,
}

#[derive(Debug, Clone)]
pub struct DependencyCluster {
    pub name: String,
    pub node_ids: Vec<String>,
    pub description: String,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: DependencyNode) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    pub fn add_edge(&mut self, edge: DependencyEdge) {
        self.edges.push(edge);
    }

    pub fn add_cluster(&mut self, cluster: DependencyCluster) {
        self.clusters.push(cluster);
    }

    pub fn get_dependencies(&self, node_id: &str) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    pub fn get_dependents(&self, node_id: &str) -> Vec<&DependencyEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    pub fn get_node(&self, id: &str) -> Option<&DependencyNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn critical_paths(&self) -> Vec<Vec<&DependencyNode>> {
        let mut paths = Vec::new();
        for edge in self
            .edges
            .iter()
            .filter(|e| e.strength == DependencyStrength::Critical)
        {
            if let (Some(from), Some(to)) = (self.get_node(&edge.from), self.get_node(&edge.to)) {
                paths.push(vec![from, to]);
            }
        }
        paths
    }

    pub fn impact_analysis(&self, node_id: &str) -> Vec<&DependencyNode> {
        let mut affected = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_dependents(node_id, &mut affected, &mut visited);
        affected
    }

    fn collect_dependents<'a>(
        &'a self,
        node_id: &str,
        affected: &mut Vec<&'a DependencyNode>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        for edge in self.get_dependents(node_id) {
            if let Some(node) = self.get_node(&edge.from) {
                affected.push(node);
                self.collect_dependents(&edge.from, affected, visited);
            }
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}
