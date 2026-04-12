// Network Management - Network interface management and monitoring

use serde::{Deserialize, Serialize};

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub network: String,
    pub mac_address: String,
    pub ip_address: Option<String>,
    pub interface_type: InterfaceType,
    pub model: String,
    pub mtu: u32,
    pub state: InterfaceState,
}

impl NetworkInterface {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            network: "default".to_string(),
            mac_address: String::new(),
            ip_address: None,
            interface_type: InterfaceType::Bridge,
            model: "virtio".to_string(),
            mtu: 1500,
            state: InterfaceState::Down,
        }
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = network.into();
        self
    }

    pub fn with_mac(mut self, mac: impl Into<String>) -> Self {
        let mac = mac.into();
        // Validate MAC address format (XX:XX:XX:XX:XX:XX)
        if !mac.is_empty() {
            let parts: Vec<&str> = mac.split(':').collect();
            if parts.len() != 6 || !parts.iter().all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit())) {
                log::warn!("Invalid MAC address format: {}. Expected XX:XX:XX:XX:XX:XX — ignoring", mac);
                return self;
            }
        }
        self.mac_address = mac;
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn with_type(mut self, iface_type: InterfaceType) -> Self {
        self.interface_type = iface_type;
        self
    }

    pub fn is_connected(&self) -> bool {
        self.state == InterfaceState::Up && self.ip_address.is_some()
    }
}

/// Interface type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterfaceType {
    Bridge,
    Masquerade,
    SRIOV,
    Multus,
}

impl InterfaceType {
    pub fn as_str(&self) -> &str {
        match self {
            InterfaceType::Bridge => "bridge",
            InterfaceType::Masquerade => "masquerade",
            InterfaceType::SRIOV => "sriov",
            InterfaceType::Multus => "multus",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "masquerade" => InterfaceType::Masquerade,
            "sriov" => InterfaceType::SRIOV,
            "multus" => InterfaceType::Multus,
            _ => InterfaceType::Bridge,
        }
    }
}

/// Interface state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterfaceState {
    Up,
    Down,
    Unknown,
}

impl std::fmt::Display for InterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceState::Up => write!(f, "UP"),
            InterfaceState::Down => write!(f, "DOWN"),
            InterfaceState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Network configuration for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub vm_name: String,
    pub interfaces: Vec<NetworkInterface>,
}

impl NetworkConfig {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            interfaces: Vec::new(),
        }
    }

    pub fn add_interface(&mut self, interface: NetworkInterface) {
        self.interfaces.push(interface);
    }

    pub fn remove_interface(&mut self, name: &str) -> bool {
        if let Some(pos) = self.interfaces.iter().position(|i| i.name == name) {
            self.interfaces.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.iter().find(|i| i.name == name)
    }

    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }
}

pub mod bandwidth;
pub mod cilium;
pub mod policies;
pub mod traffic;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_interface_creation() {
        let iface = NetworkInterface::new("eth0")
            .with_network("pod-network")
            .with_mac("52:54:00:12:34:56")
            .with_ip("10.244.0.5");

        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.network, "pod-network");
        assert_eq!(iface.mac_address, "52:54:00:12:34:56");
        assert_eq!(iface.ip_address, Some("10.244.0.5".to_string()));
    }

    #[test]
    fn test_interface_type() {
        assert_eq!(InterfaceType::Bridge.as_str(), "bridge");
        assert_eq!(InterfaceType::Masquerade.as_str(), "masquerade");
        assert_eq!(InterfaceType::SRIOV.as_str(), "sriov");
        assert_eq!(InterfaceType::Multus.as_str(), "multus");

        assert_eq!(InterfaceType::parse("bridge"), InterfaceType::Bridge);
        assert_eq!(
            InterfaceType::parse("masquerade"),
            InterfaceType::Masquerade
        );
    }

    #[test]
    fn test_interface_state() {
        let mut iface = NetworkInterface::new("eth0");
        iface.state = InterfaceState::Down;
        assert!(!iface.is_connected());

        iface.state = InterfaceState::Up;
        iface.ip_address = Some("10.0.0.1".to_string());
        assert!(iface.is_connected());
    }

    #[test]
    fn test_network_config() {
        let mut config = NetworkConfig::new("test-vm");

        let iface1 = NetworkInterface::new("eth0").with_network("default");
        let iface2 = NetworkInterface::new("eth1").with_network("storage");

        config.add_interface(iface1);
        config.add_interface(iface2);

        assert_eq!(config.interface_count(), 2);
        assert!(config.get_interface("eth0").is_some());
        assert!(config.get_interface("eth1").is_some());

        assert!(config.remove_interface("eth0"));
        assert_eq!(config.interface_count(), 1);
    }
}
