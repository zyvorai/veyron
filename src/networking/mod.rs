// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

pub mod bgp;
pub mod dns;
pub mod ipam;
pub mod policies;
pub mod qos;
pub mod topology;

/// Network protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkProtocol {
    TCP,
    UDP,
    ICMP,
    SCTP,
    All,
}

/// Network interface type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceType {
    Bridge,
    Masquerade,
    SRIOV,
    Macvtap,
    Passt,
}

/// VLAN configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLANConfig {
    pub id: u16,
    pub name: String,
    pub description: String,
    pub subnet: String,
    pub gateway: Option<IpAddr>,
    pub created_at: DateTime<Utc>,
}

impl VLANConfig {
    pub fn new(id: u16, name: impl Into<String>, subnet: impl Into<String>) -> Self {
        let id = if id == 0 || id > 4094 {
            log::warn!("Invalid VLAN ID: {}. Valid range is 1-4094, clamping.", id);
            id.clamp(1, 4094)
        } else {
            id
        };
        Self {
            id,
            name: name.into(),
            description: String::new(),
            subnet: subnet.into(),
            gateway: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_gateway(mut self, gateway: IpAddr) -> Self {
        self.gateway = Some(gateway);
        self
    }

    pub fn is_valid_id(&self) -> bool {
        self.id > 0 && self.id <= 4094
    }
}

/// Network attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachment {
    pub id: String,
    pub name: String,
    pub interface_type: InterfaceType,
    pub network_name: String,
    pub vlan_id: Option<u16>,
    pub ip_address: Option<IpAddr>,
    pub mac_address: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl NetworkAttachment {
    pub fn new(
        name: impl Into<String>,
        interface_type: InterfaceType,
        network_name: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!("attach-{}-{}", name_str, Utc::now().timestamp_micros());

        Self {
            id,
            name: name_str,
            interface_type,
            network_name: network_name.into(),
            vlan_id: None,
            ip_address: None,
            mac_address: None,
            bandwidth_mbps: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_vlan(mut self, vlan_id: u16) -> Self {
        self.vlan_id = Some(vlan_id);
        self
    }

    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_mac(mut self, mac: impl Into<String>) -> Self {
        let mac = mac.into();
        // Validate MAC address format (XX:XX:XX:XX:XX:XX)
        let parts: Vec<&str> = mac.split(':').collect();
        if parts.len() != 6
            || !parts
                .iter()
                .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
        {
            log::warn!(
                "Invalid MAC address format: {}. Expected XX:XX:XX:XX:XX:XX",
                mac
            );
            return self;
        }
        self.mac_address = Some(mac);
        self
    }

    pub fn with_bandwidth(mut self, mbps: u32) -> Self {
        self.bandwidth_mbps = Some(mbps);
        self
    }

    pub fn is_ipv6(&self) -> bool {
        matches!(self.ip_address, Some(IpAddr::V6(_)))
    }
}

/// Network namespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNamespace {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub isolated: bool,
    pub attachments: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl NetworkNamespace {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!("netns-{}-{}", name_str, Utc::now().timestamp());

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            isolated: false,
            attachments: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn isolate(mut self) -> Self {
        self.isolated = true;
        self
    }

    pub fn add_attachment(&mut self, attachment_id: impl Into<String>) {
        self.attachments.push(attachment_id.into());
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }
}

/// Network manager
pub struct NetworkManager {
    vlans: HashMap<u16, VLANConfig>,
    attachments: HashMap<String, NetworkAttachment>,
    namespaces: HashMap<String, NetworkNamespace>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            vlans: HashMap::new(),
            attachments: HashMap::new(),
            namespaces: HashMap::new(),
        }
    }

    pub fn add_vlan(&mut self, vlan: VLANConfig) -> u16 {
        let id = vlan.id;
        self.vlans.insert(id, vlan);
        id
    }

    pub fn get_vlan(&self, id: u16) -> Option<&VLANConfig> {
        self.vlans.get(&id)
    }

    pub fn vlan_count(&self) -> usize {
        self.vlans.len()
    }

    pub fn add_attachment(&mut self, attachment: NetworkAttachment) -> String {
        let id = attachment.id.clone();
        self.attachments.insert(id.clone(), attachment);
        id
    }

    pub fn get_attachment(&self, id: &str) -> Option<&NetworkAttachment> {
        self.attachments.get(id)
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    pub fn add_namespace(&mut self, namespace: NetworkNamespace) -> String {
        let id = namespace.id.clone();
        self.namespaces.insert(id.clone(), namespace);
        id
    }

    pub fn get_namespace(&self, id: &str) -> Option<&NetworkNamespace> {
        self.namespaces.get(id)
    }

    pub fn namespace_count(&self) -> usize {
        self.namespaces.len()
    }

    pub fn attachments_by_network(&self, network_name: &str) -> Vec<&NetworkAttachment> {
        self.attachments
            .values()
            .filter(|a| a.network_name == network_name)
            .collect()
    }

    pub fn attachments_by_vlan(&self, vlan_id: u16) -> Vec<&NetworkAttachment> {
        self.attachments
            .values()
            .filter(|a| a.vlan_id == Some(vlan_id))
            .collect()
    }

    pub fn isolated_namespaces(&self) -> Vec<&NetworkNamespace> {
        self.namespaces.values().filter(|n| n.isolated).collect()
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_vlan_config() {
        let vlan = VLANConfig::new(100, "production", "10.0.100.0/24");

        assert_eq!(vlan.id, 100);
        assert_eq!(vlan.name, "production");
        assert_eq!(vlan.subnet, "10.0.100.0/24");
        assert!(vlan.gateway.is_none());
    }

    #[test]
    fn test_vlan_with_description() {
        let vlan = VLANConfig::new(200, "staging", "10.0.200.0/24")
            .with_description("Staging environment");

        assert_eq!(vlan.description, "Staging environment");
    }

    #[test]
    fn test_vlan_with_gateway() {
        let gateway = IpAddr::V4(Ipv4Addr::new(10, 0, 100, 1));
        let vlan = VLANConfig::new(100, "prod", "10.0.100.0/24").with_gateway(gateway);

        assert_eq!(vlan.gateway, Some(gateway));
    }

    #[test]
    fn test_vlan_is_valid_id() {
        let vlan1 = VLANConfig::new(100, "test", "10.0.0.0/24");
        assert!(vlan1.is_valid_id());

        // Invalid IDs are now clamped to valid range (1-4094)
        let vlan2 = VLANConfig::new(0, "clamped", "10.0.0.0/24");
        assert_eq!(vlan2.id, 1); // Clamped from 0 to 1
        assert!(vlan2.is_valid_id());

        let vlan3 = VLANConfig::new(4095, "clamped", "10.0.0.0/24");
        assert_eq!(vlan3.id, 4094); // Clamped from 4095 to 4094
        assert!(vlan3.is_valid_id());
    }

    #[test]
    fn test_network_attachment() {
        let attachment = NetworkAttachment::new("eth0", InterfaceType::Bridge, "default");

        assert_eq!(attachment.name, "eth0");
        assert_eq!(attachment.interface_type, InterfaceType::Bridge);
        assert_eq!(attachment.network_name, "default");
    }

    #[test]
    fn test_attachment_with_vlan() {
        let attachment =
            NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1").with_vlan(100);

        assert_eq!(attachment.vlan_id, Some(100));
    }

    #[test]
    fn test_attachment_with_ip() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
        let attachment = NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1").with_ip(ip);

        assert_eq!(attachment.ip_address, Some(ip));
    }

    #[test]
    fn test_attachment_with_mac() {
        let attachment = NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1")
            .with_mac("00:11:22:33:44:55");

        assert_eq!(
            attachment.mac_address,
            Some("00:11:22:33:44:55".to_string())
        );
    }

    #[test]
    fn test_attachment_with_bandwidth() {
        let attachment =
            NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1").with_bandwidth(1000);

        assert_eq!(attachment.bandwidth_mbps, Some(1000));
    }

    #[test]
    fn test_attachment_is_ipv6() {
        let ipv4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
        let attachment1 =
            NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1").with_ip(ipv4);
        assert!(!attachment1.is_ipv6());

        let ipv6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let attachment2 =
            NetworkAttachment::new("eth1", InterfaceType::Bridge, "net1").with_ip(ipv6);
        assert!(attachment2.is_ipv6());
    }

    #[test]
    fn test_network_namespace() {
        let netns = NetworkNamespace::new("vm-net", "default");

        assert_eq!(netns.name, "vm-net");
        assert_eq!(netns.namespace, "default");
        assert!(!netns.isolated);
    }

    #[test]
    fn test_namespace_isolate() {
        let netns = NetworkNamespace::new("vm-net", "default").isolate();

        assert!(netns.isolated);
    }

    #[test]
    fn test_namespace_add_attachment() {
        let mut netns = NetworkNamespace::new("vm-net", "default");

        netns.add_attachment("attach-1");
        netns.add_attachment("attach-2");

        assert_eq!(netns.attachment_count(), 2);
    }

    #[test]
    fn test_network_manager() {
        let mut manager = NetworkManager::new();

        let vlan = VLANConfig::new(100, "prod", "10.0.100.0/24");
        let id = manager.add_vlan(vlan);

        assert_eq!(manager.vlan_count(), 1);
        assert!(manager.get_vlan(id).is_some());
    }

    #[test]
    fn test_manager_add_attachment() {
        let mut manager = NetworkManager::new();

        let attachment = NetworkAttachment::new("eth0", InterfaceType::Bridge, "default");
        let id = manager.add_attachment(attachment);

        assert_eq!(manager.attachment_count(), 1);
        assert!(manager.get_attachment(&id).is_some());
    }

    #[test]
    fn test_manager_add_namespace() {
        let mut manager = NetworkManager::new();

        let netns = NetworkNamespace::new("vm-net", "default");
        let id = manager.add_namespace(netns);

        assert_eq!(manager.namespace_count(), 1);
        assert!(manager.get_namespace(&id).is_some());
    }

    #[test]
    fn test_manager_attachments_by_network() {
        let mut manager = NetworkManager::new();

        manager.add_attachment(NetworkAttachment::new(
            "eth0",
            InterfaceType::Bridge,
            "net1",
        ));
        manager.add_attachment(NetworkAttachment::new(
            "eth1",
            InterfaceType::Bridge,
            "net2",
        ));
        manager.add_attachment(NetworkAttachment::new(
            "eth2",
            InterfaceType::Bridge,
            "net1",
        ));

        let net1_attachments = manager.attachments_by_network("net1");
        assert_eq!(net1_attachments.len(), 2);
    }

    #[test]
    fn test_manager_attachments_by_vlan() {
        let mut manager = NetworkManager::new();

        manager.add_attachment(
            NetworkAttachment::new("eth0", InterfaceType::Bridge, "net1").with_vlan(100),
        );
        manager.add_attachment(
            NetworkAttachment::new("eth1", InterfaceType::Bridge, "net1").with_vlan(200),
        );
        manager.add_attachment(
            NetworkAttachment::new("eth2", InterfaceType::Bridge, "net1").with_vlan(100),
        );

        let vlan100 = manager.attachments_by_vlan(100);
        assert_eq!(vlan100.len(), 2);
    }

    #[test]
    fn test_manager_isolated_namespaces() {
        let mut manager = NetworkManager::new();

        manager.add_namespace(NetworkNamespace::new("ns1", "default"));
        manager.add_namespace(NetworkNamespace::new("ns2", "default").isolate());
        manager.add_namespace(NetworkNamespace::new("ns3", "default").isolate());

        let isolated = manager.isolated_namespaces();
        assert_eq!(isolated.len(), 2);
    }

    #[test]
    fn test_network_protocol_equality() {
        assert_eq!(NetworkProtocol::TCP, NetworkProtocol::TCP);
        assert_ne!(NetworkProtocol::TCP, NetworkProtocol::UDP);
    }

    #[test]
    fn test_interface_type_equality() {
        assert_eq!(InterfaceType::Bridge, InterfaceType::Bridge);
        assert_ne!(InterfaceType::Bridge, InterfaceType::SRIOV);
    }
}
