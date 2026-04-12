use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network node type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    VM,
    Router,
    Switch,
    LoadBalancer,
    Gateway,
}

/// Network link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    pub id: String,
    pub name: String,
    pub source_node: String,
    pub target_node: String,
    pub bandwidth_mbps: u32,
    pub latency_ms: Option<u32>,
    pub status: LinkStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStatus {
    Up,
    Down,
    Degraded,
}

impl NetworkLink {
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        bandwidth_mbps: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!("link-{}-{}", name_str, Utc::now().timestamp_micros());

        Self {
            id,
            name: name_str,
            source_node: source.into(),
            target_node: target.into(),
            bandwidth_mbps,
            latency_ms: None,
            status: LinkStatus::Up,
            created_at: Utc::now(),
        }
    }

    pub fn with_latency(mut self, ms: u32) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    pub fn set_status(&mut self, status: LinkStatus) {
        self.status = status;
    }

    pub fn is_operational(&self) -> bool {
        self.status == LinkStatus::Up
    }
}

/// Network node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub zone: Option<String>,
    pub links: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl NetworkNode {
    pub fn new(name: impl Into<String>, node_type: NodeType) -> Self {
        let name_str = name.into();
        let id = format!("node-{}-{}", name_str, Utc::now().timestamp_micros());

        Self {
            id,
            name: name_str,
            node_type,
            zone: None,
            links: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }

    pub fn add_link(&mut self, link_id: impl Into<String>) {
        self.links.push(link_id.into());
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }
}

/// Topology manager
pub struct TopologyManager {
    nodes: HashMap<String, NetworkNode>,
    links: HashMap<String, NetworkLink>,
}

impl TopologyManager {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            links: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NetworkNode) -> String {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        id
    }

    pub fn get_node(&self, id: &str) -> Option<&NetworkNode> {
        self.nodes.get(id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn add_link(&mut self, link: NetworkLink) -> String {
        let id = link.id.clone();
        self.links.insert(id.clone(), link);
        id
    }

    pub fn get_link(&self, id: &str) -> Option<&NetworkLink> {
        self.links.get(id)
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    pub fn nodes_by_type(&self, node_type: &NodeType) -> Vec<&NetworkNode> {
        self.nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .collect()
    }

    pub fn operational_links(&self) -> Vec<&NetworkLink> {
        self.links.values().filter(|l| l.is_operational()).collect()
    }
}

impl Default for TopologyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_link() {
        let link = NetworkLink::new("link1", "node-a", "node-b", 1000);

        assert_eq!(link.name, "link1");
        assert_eq!(link.source_node, "node-a");
        assert_eq!(link.target_node, "node-b");
        assert_eq!(link.bandwidth_mbps, 1000);
        assert_eq!(link.status, LinkStatus::Up);
    }

    #[test]
    fn test_link_with_latency() {
        let link = NetworkLink::new("link1", "a", "b", 1000).with_latency(10);

        assert_eq!(link.latency_ms, Some(10));
    }

    #[test]
    fn test_link_set_status() {
        let mut link = NetworkLink::new("link1", "a", "b", 1000);

        link.set_status(LinkStatus::Down);
        assert_eq!(link.status, LinkStatus::Down);
    }

    #[test]
    fn test_link_is_operational() {
        let link1 = NetworkLink::new("link1", "a", "b", 1000);
        assert!(link1.is_operational());

        let mut link2 = NetworkLink::new("link2", "a", "b", 1000);
        link2.set_status(LinkStatus::Down);
        assert!(!link2.is_operational());
    }

    #[test]
    fn test_network_node() {
        let node = NetworkNode::new("router1", NodeType::Router);

        assert_eq!(node.name, "router1");
        assert_eq!(node.node_type, NodeType::Router);
    }

    #[test]
    fn test_node_with_zone() {
        let node = NetworkNode::new("vm1", NodeType::VM).with_zone("us-west-1a");

        assert_eq!(node.zone, Some("us-west-1a".to_string()));
    }

    #[test]
    fn test_node_add_link() {
        let mut node = NetworkNode::new("router1", NodeType::Router);

        node.add_link("link-1");
        node.add_link("link-2");

        assert_eq!(node.link_count(), 2);
    }

    #[test]
    fn test_topology_manager() {
        let mut manager = TopologyManager::new();

        let node = NetworkNode::new("router1", NodeType::Router);
        manager.add_node(node);

        assert_eq!(manager.node_count(), 1);
    }

    #[test]
    fn test_manager_add_link() {
        let mut manager = TopologyManager::new();

        let link = NetworkLink::new("link1", "a", "b", 1000);
        manager.add_link(link);

        assert_eq!(manager.link_count(), 1);
    }

    #[test]
    fn test_manager_nodes_by_type() {
        let mut manager = TopologyManager::new();

        manager.add_node(NetworkNode::new("vm1", NodeType::VM));
        manager.add_node(NetworkNode::new("router1", NodeType::Router));
        manager.add_node(NetworkNode::new("vm2", NodeType::VM));

        let vms = manager.nodes_by_type(&NodeType::VM);
        assert_eq!(vms.len(), 2);
    }

    #[test]
    fn test_manager_operational_links() {
        let mut manager = TopologyManager::new();

        let link1 = NetworkLink::new("link1", "a", "b", 1000);
        let mut link2 = NetworkLink::new("link2", "b", "c", 1000);
        link2.set_status(LinkStatus::Down);

        manager.add_link(link1);
        manager.add_link(link2);

        let operational = manager.operational_links();
        assert_eq!(operational.len(), 1);
    }

    #[test]
    fn test_node_type_equality() {
        assert_eq!(NodeType::VM, NodeType::VM);
        assert_ne!(NodeType::VM, NodeType::Router);
    }

    #[test]
    fn test_link_status_equality() {
        assert_eq!(LinkStatus::Up, LinkStatus::Up);
        assert_ne!(LinkStatus::Up, LinkStatus::Down);
    }
}
