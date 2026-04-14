// Hypervisor Connector - Direct hypervisor communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervisorConnector {
    pub connections: Vec<HypervisorConnection>,
    pub capabilities: Vec<HypervisorCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervisorConnection {
    pub id: String,
    pub hypervisor_type: HypervisorType,
    pub host: String,
    pub port: u16,
    pub status: ConnectionStatus,
    pub version: String,
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HypervisorType {
    KVM,
    QEMU,
    Xen,
    VMware,
    HyperV,
    VirtualBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
    Authenticating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervisorCapability {
    pub name: String,
    pub supported: bool,
    pub version: Option<String>,
}

impl HypervisorConnector {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    pub fn add_connection(&mut self, conn: HypervisorConnection) {
        self.connections.push(conn);
    }

    pub fn get_connection(&self, id: &str) -> Option<&HypervisorConnection> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn connected(&self) -> Vec<&HypervisorConnection> {
        self.connections
            .iter()
            .filter(|c| c.status == ConnectionStatus::Connected)
            .collect()
    }

    pub fn detect_capabilities(&mut self) {
        self.capabilities = vec![
            HypervisorCapability {
                name: "live_migration".to_string(),
                supported: true,
                version: Some("1.0".to_string()),
            },
            HypervisorCapability {
                name: "snapshot".to_string(),
                supported: true,
                version: Some("1.0".to_string()),
            },
            HypervisorCapability {
                name: "hot_plug".to_string(),
                supported: true,
                version: Some("1.0".to_string()),
            },
            HypervisorCapability {
                name: "nested_virtualization".to_string(),
                supported: false,
                version: None,
            },
            HypervisorCapability {
                name: "gpu_passthrough".to_string(),
                supported: true,
                version: Some("1.0".to_string()),
            },
        ];
    }

    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.name == capability && c.supported)
    }
}

impl Default for HypervisorConnector {
    fn default() -> Self {
        Self::new()
    }
}
