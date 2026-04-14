// Cluster Topology Map - Visual representation of nodes, VMs, and relationships

use std::collections::HashMap;

pub mod view;

#[derive(Debug, Clone)]
pub struct TopologyMap {
    pub nodes: Vec<TopologyNode>,
    pub vms: Vec<TopologyVm>,
    pub connections: Vec<TopologyConnection>,
    pub network_zones: Vec<NetworkZone>,
    pub storage_pools: Vec<StoragePool>,
    pub metadata: TopologyMetadata,
}

#[derive(Debug, Clone)]
pub struct TopologyNode {
    pub name: String,
    pub status: NodeStatus,
    pub role: NodeRole,
    pub zone: Option<String>,
    pub cpu_capacity: f64,
    pub cpu_usage: f64,
    pub memory_capacity: f64,
    pub memory_usage: f64,
    pub vm_count: usize,
    pub labels: HashMap<String, String>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Ready,
    NotReady,
    Unknown,
    SchedulingDisabled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeRole {
    ControlPlane,
    Worker,
    Edge,
}

#[derive(Debug, Clone)]
pub struct TopologyVm {
    pub name: String,
    pub namespace: String,
    pub node: String,
    pub status: VmStatus,
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub network_attachments: Vec<String>,
    pub storage_volumes: Vec<String>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VmStatus {
    Running,
    Stopped,
    Paused,
    Migrating,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TopologyConnection {
    pub from: String,
    pub to: String,
    pub connection_type: ConnectionType,
    pub bandwidth: Option<f64>,
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    VmToNode,
    NodeToNode,
    VmToStorage,
    VmToNetwork,
}

#[derive(Debug, Clone)]
pub struct NetworkZone {
    pub name: String,
    pub subnet: String,
    pub vlan_id: Option<u16>,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StoragePool {
    pub name: String,
    pub storage_class: String,
    pub total_capacity: f64,
    pub used_capacity: f64,
    pub provisioner: String,
}

#[derive(Debug, Clone)]
pub struct TopologyMetadata {
    pub cluster_name: String,
    pub total_nodes: usize,
    pub total_vms: usize,
    pub healthy_nodes: usize,
    pub running_vms: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl TopologyMap {
    pub fn new(cluster_name: String) -> Self {
        Self {
            nodes: Vec::new(),
            vms: Vec::new(),
            connections: Vec::new(),
            network_zones: Vec::new(),
            storage_pools: Vec::new(),
            metadata: TopologyMetadata {
                cluster_name,
                total_nodes: 0,
                total_vms: 0,
                healthy_nodes: 0,
                running_vms: 0,
            },
        }
    }

    pub fn add_node(&mut self, node: TopologyNode) {
        self.nodes.push(node);
        self.update_metadata();
    }

    pub fn add_vm(&mut self, vm: TopologyVm) {
        self.connections.push(TopologyConnection {
            from: vm.name.clone(),
            to: vm.node.clone(),
            connection_type: ConnectionType::VmToNode,
            bandwidth: None,
            latency_ms: None,
        });
        self.vms.push(vm);
        self.update_metadata();
    }

    pub fn remove_vm(&mut self, vm_name: &str) {
        self.vms.retain(|v| v.name != vm_name);
        self.connections
            .retain(|c| c.from != vm_name && c.to != vm_name);
        self.update_metadata();
    }

    fn update_metadata(&mut self) {
        self.metadata.total_nodes = self.nodes.len();
        self.metadata.total_vms = self.vms.len();
        self.metadata.healthy_nodes = self
            .nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Ready)
            .count();
        self.metadata.running_vms = self
            .vms
            .iter()
            .filter(|v| v.status == VmStatus::Running)
            .count();
    }

    pub fn calculate_layout(&mut self, width: usize, height: usize) {
        let node_count = self.nodes.len().max(1);
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / node_count as f64;
            let radius = (width.min(height) as f64) / 3.0;
            node.position = Position {
                x: (width as f64 / 2.0) + radius * angle.cos(),
                y: (height as f64 / 2.0) + radius * angle.sin(),
            };
        }

        for vm in &mut self.vms {
            if let Some(node) = self.nodes.iter().find(|n| n.name == vm.node) {
                vm.position = Position {
                    x: node.position.x + 2.0,
                    y: node.position.y + 1.0,
                };
            }
        }
    }

    pub fn vms_on_node(&self, node_name: &str) -> Vec<&TopologyVm> {
        self.vms.iter().filter(|v| v.node == node_name).collect()
    }
}
