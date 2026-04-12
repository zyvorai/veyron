use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node capability
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCapability {
    Compute,
    Storage,
    AiMl,
    VideoProcessing,
    IoTGateway,
}

/// Edge node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNode {
    pub id: String,
    pub name: String,
    pub deployment_id: String,
    pub capabilities: Vec<NodeCapability>,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub gpu_available: bool,
    pub online: bool,
    pub workload_count: u32,
    pub created_at: DateTime<Utc>,
}

impl EdgeNode {
    pub fn new(
        name: impl Into<String>,
        deployment_id: impl Into<String>,
        cpu_cores: u32,
        memory_gb: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "node-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            name: name_str,
            deployment_id: deployment_id.into(),
            capabilities: Vec::new(),
            cpu_cores,
            memory_gb,
            storage_gb: 0,
            gpu_available: false,
            online: false,
            workload_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_storage(mut self, gb: u32) -> Self {
        self.storage_gb = gb;
        self
    }

    pub fn with_gpu(mut self) -> Self {
        self.gpu_available = true;
        self
    }

    pub fn add_capability(&mut self, capability: NodeCapability) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn has_capability(&self, capability: &NodeCapability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn is_available(&self) -> bool {
        self.online && self.workload_count < 10
    }
}

/// Node manager
pub struct NodeManager {
    nodes: HashMap<String, EdgeNode>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: EdgeNode) -> String {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        id
    }

    pub fn get_node(&self, id: &str) -> Option<&EdgeNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut EdgeNode> {
        self.nodes.get_mut(id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes_by_deployment(&self, deployment_id: &str) -> Vec<&EdgeNode> {
        self.nodes
            .values()
            .filter(|n| n.deployment_id == deployment_id)
            .collect()
    }

    pub fn online_nodes(&self) -> Vec<&EdgeNode> {
        self.nodes.values().filter(|n| n.online).collect()
    }

    pub fn available_nodes(&self) -> Vec<&EdgeNode> {
        self.nodes.values().filter(|n| n.is_available()).collect()
    }

    pub fn nodes_with_capability(&self, capability: &NodeCapability) -> Vec<&EdgeNode> {
        self.nodes
            .values()
            .filter(|n| n.has_capability(capability))
            .collect()
    }

    pub fn nodes_with_gpu(&self) -> Vec<&EdgeNode> {
        self.nodes.values().filter(|n| n.gpu_available).collect()
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_node() {
        let node = EdgeNode::new("edge-1", "deploy-1", 4, 16);

        assert_eq!(node.name, "edge-1");
        assert_eq!(node.deployment_id, "deploy-1");
        assert_eq!(node.cpu_cores, 4);
        assert_eq!(node.memory_gb, 16);
        assert!(!node.online);
    }

    #[test]
    fn test_node_with_storage() {
        let node = EdgeNode::new("edge-1", "deploy-1", 4, 16).with_storage(500);

        assert_eq!(node.storage_gb, 500);
    }

    #[test]
    fn test_node_with_gpu() {
        let node = EdgeNode::new("edge-1", "deploy-1", 4, 16).with_gpu();

        assert!(node.gpu_available);
    }

    #[test]
    fn test_node_add_capability() {
        let mut node = EdgeNode::new("edge-1", "deploy-1", 4, 16);

        node.add_capability(NodeCapability::Compute);
        node.add_capability(NodeCapability::AiMl);
        node.add_capability(NodeCapability::Compute); // Duplicate

        assert_eq!(node.capabilities.len(), 2);
    }

    #[test]
    fn test_node_set_online() {
        let mut node = EdgeNode::new("edge-1", "deploy-1", 4, 16);

        node.set_online(true);
        assert!(node.online);

        node.set_online(false);
        assert!(!node.online);
    }

    #[test]
    fn test_node_has_capability() {
        let mut node = EdgeNode::new("edge-1", "deploy-1", 4, 16);

        node.add_capability(NodeCapability::VideoProcessing);
        assert!(node.has_capability(&NodeCapability::VideoProcessing));
        assert!(!node.has_capability(&NodeCapability::AiMl));
    }

    #[test]
    fn test_node_is_available() {
        let mut node = EdgeNode::new("edge-1", "deploy-1", 4, 16);

        assert!(!node.is_available()); // Offline

        node.set_online(true);
        assert!(node.is_available()); // Online, low workload

        node.workload_count = 15;
        assert!(!node.is_available()); // Online, but high workload
    }

    #[test]
    fn test_node_manager() {
        let mut manager = NodeManager::new();

        let node = EdgeNode::new("edge-1", "deploy-1", 4, 16);
        let id = manager.add_node(node);

        assert_eq!(manager.node_count(), 1);
        assert!(manager.get_node(&id).is_some());
    }

    #[test]
    fn test_manager_nodes_by_deployment() {
        let mut manager = NodeManager::new();

        manager.add_node(EdgeNode::new("node-1", "deploy-1", 4, 16));
        manager.add_node(EdgeNode::new("node-2", "deploy-2", 4, 16));
        manager.add_node(EdgeNode::new("node-3", "deploy-1", 4, 16));

        let deploy1_nodes = manager.nodes_by_deployment("deploy-1");
        assert_eq!(deploy1_nodes.len(), 2);
    }

    #[test]
    fn test_manager_online_nodes() {
        let mut manager = NodeManager::new();

        let mut node1 = EdgeNode::new("node-1", "deploy-1", 4, 16);
        node1.set_online(true);

        let node2 = EdgeNode::new("node-2", "deploy-1", 4, 16);

        manager.add_node(node1);
        manager.add_node(node2);

        let online = manager.online_nodes();
        assert_eq!(online.len(), 1);
    }

    #[test]
    fn test_manager_available_nodes() {
        let mut manager = NodeManager::new();

        let mut node1 = EdgeNode::new("node-1", "deploy-1", 4, 16);
        node1.set_online(true);

        let mut node2 = EdgeNode::new("node-2", "deploy-1", 4, 16);
        node2.set_online(true);
        node2.workload_count = 15;

        manager.add_node(node1);
        manager.add_node(node2);

        let available = manager.available_nodes();
        assert_eq!(available.len(), 1);
    }

    #[test]
    fn test_manager_nodes_with_capability() {
        let mut manager = NodeManager::new();

        let mut node1 = EdgeNode::new("node-1", "deploy-1", 4, 16);
        node1.add_capability(NodeCapability::AiMl);

        let mut node2 = EdgeNode::new("node-2", "deploy-1", 4, 16);
        node2.add_capability(NodeCapability::VideoProcessing);

        let mut node3 = EdgeNode::new("node-3", "deploy-1", 4, 16);
        node3.add_capability(NodeCapability::AiMl);

        manager.add_node(node1);
        manager.add_node(node2);
        manager.add_node(node3);

        let ai_nodes = manager.nodes_with_capability(&NodeCapability::AiMl);
        assert_eq!(ai_nodes.len(), 2);
    }

    #[test]
    fn test_manager_nodes_with_gpu() {
        let mut manager = NodeManager::new();

        manager.add_node(EdgeNode::new("node-1", "deploy-1", 4, 16).with_gpu());
        manager.add_node(EdgeNode::new("node-2", "deploy-1", 4, 16));
        manager.add_node(EdgeNode::new("node-3", "deploy-1", 4, 16).with_gpu());

        let gpu_nodes = manager.nodes_with_gpu();
        assert_eq!(gpu_nodes.len(), 2);
    }

    #[test]
    fn test_node_capability_equality() {
        assert_eq!(NodeCapability::AiMl, NodeCapability::AiMl);
        assert_ne!(NodeCapability::AiMl, NodeCapability::Compute);
    }
}
