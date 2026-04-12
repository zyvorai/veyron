use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::CloudProvider;

/// Connection type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    VPN,
    DirectConnect,
    ExpressRoute,
    InterconnectDedicated,
    InterconnectPartner,
    PrivateLink,
}

/// Connection status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Pending,
    Establishing,
    Active,
    Degraded,
    Failed,
    Terminated,
}

/// Cloud connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConnection {
    pub id: String,
    pub name: String,
    pub connection_type: ConnectionType,
    pub source_provider: CloudProvider,
    pub source_region: String,
    pub target_provider: CloudProvider,
    pub target_region: String,
    pub bandwidth_mbps: u32,
    pub latency_ms: Option<u32>,
    pub status: ConnectionStatus,
    pub encrypted: bool,
    pub redundant: bool,
    pub created_at: DateTime<Utc>,
    pub last_tested: Option<DateTime<Utc>>,
}

impl CloudConnection {
    pub fn new(
        name: impl Into<String>,
        connection_type: ConnectionType,
        source_provider: CloudProvider,
        source_region: impl Into<String>,
        target_provider: CloudProvider,
        target_region: impl Into<String>,
        bandwidth_mbps: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "conn-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            connection_type,
            source_provider,
            source_region: source_region.into(),
            target_provider,
            target_region: target_region.into(),
            bandwidth_mbps,
            latency_ms: None,
            status: ConnectionStatus::Pending,
            encrypted: true,
            redundant: false,
            created_at: Utc::now(),
            last_tested: None,
        }
    }

    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn enable_redundancy(mut self) -> Self {
        self.redundant = true;
        self
    }

    pub fn disable_encryption(mut self) -> Self {
        self.encrypted = false;
        self
    }

    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    pub fn record_test(&mut self, latency_ms: u32) {
        self.latency_ms = Some(latency_ms);
        self.last_tested = Some(Utc::now());
    }

    pub fn is_active(&self) -> bool {
        self.status == ConnectionStatus::Active
    }

    pub fn is_healthy(&self) -> bool {
        matches!(
            self.status,
            ConnectionStatus::Active | ConnectionStatus::Degraded
        )
    }

    pub fn is_cross_provider(&self) -> bool {
        self.source_provider != self.target_provider
    }
}

/// Network tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTunnel {
    pub id: String,
    pub name: String,
    pub connection_id: String,
    pub tunnel_type: TunnelType,
    pub local_endpoint: String,
    pub remote_endpoint: String,
    pub local_cidr: String,
    pub remote_cidr: String,
    pub status: TunnelStatus,
    pub created_at: DateTime<Utc>,
}

/// Tunnel type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelType {
    IPSec,
    GRE,
    VXLAN,
    WireGuard,
}

/// Tunnel status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelStatus {
    Down,
    Up,
    Degraded,
}

impl NetworkTunnel {
    pub fn new(
        name: impl Into<String>,
        connection_id: impl Into<String>,
        tunnel_type: TunnelType,
        local_endpoint: impl Into<String>,
        remote_endpoint: impl Into<String>,
        local_cidr: impl Into<String>,
        remote_cidr: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "tunnel-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            connection_id: connection_id.into(),
            tunnel_type,
            local_endpoint: local_endpoint.into(),
            remote_endpoint: remote_endpoint.into(),
            local_cidr: local_cidr.into(),
            remote_cidr: remote_cidr.into(),
            status: TunnelStatus::Down,
            created_at: Utc::now(),
        }
    }

    pub fn set_status(&mut self, status: TunnelStatus) {
        self.status = status;
    }

    pub fn is_up(&self) -> bool {
        self.status == TunnelStatus::Up
    }
}

/// Connectivity manager
pub struct ConnectivityManager {
    connections: HashMap<String, CloudConnection>,
    tunnels: HashMap<String, NetworkTunnel>,
}

impl ConnectivityManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            tunnels: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, connection: CloudConnection) -> String {
        let id = connection.id.clone();
        self.connections.insert(id.clone(), connection);
        id
    }

    pub fn get_connection(&self, id: &str) -> Option<&CloudConnection> {
        self.connections.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut CloudConnection> {
        self.connections.get_mut(id)
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn add_tunnel(&mut self, tunnel: NetworkTunnel) -> String {
        let id = tunnel.id.clone();
        self.tunnels.insert(id.clone(), tunnel);
        id
    }

    pub fn get_tunnel(&self, id: &str) -> Option<&NetworkTunnel> {
        self.tunnels.get(id)
    }

    pub fn get_tunnel_mut(&mut self, id: &str) -> Option<&mut NetworkTunnel> {
        self.tunnels.get_mut(id)
    }

    pub fn tunnel_count(&self) -> usize {
        self.tunnels.len()
    }

    pub fn active_connections(&self) -> Vec<&CloudConnection> {
        self.connections
            .values()
            .filter(|c| c.is_active())
            .collect()
    }

    pub fn healthy_connections(&self) -> Vec<&CloudConnection> {
        self.connections
            .values()
            .filter(|c| c.is_healthy())
            .collect()
    }

    pub fn cross_provider_connections(&self) -> Vec<&CloudConnection> {
        self.connections
            .values()
            .filter(|c| c.is_cross_provider())
            .collect()
    }

    pub fn connections_by_type(&self, connection_type: &ConnectionType) -> Vec<&CloudConnection> {
        self.connections
            .values()
            .filter(|c| &c.connection_type == connection_type)
            .collect()
    }

    pub fn redundant_connections(&self) -> Vec<&CloudConnection> {
        self.connections.values().filter(|c| c.redundant).collect()
    }

    pub fn tunnels_for_connection(&self, connection_id: &str) -> Vec<&NetworkTunnel> {
        self.tunnels
            .values()
            .filter(|t| t.connection_id == connection_id)
            .collect()
    }

    pub fn active_tunnels(&self) -> Vec<&NetworkTunnel> {
        self.tunnels.values().filter(|t| t.is_up()).collect()
    }
}

impl Default for ConnectivityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_connection() {
        let connection = CloudConnection::new(
            "aws-to-azure",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );

        assert_eq!(connection.name, "aws-to-azure");
        assert_eq!(connection.connection_type, ConnectionType::VPN);
        assert_eq!(connection.source_provider, CloudProvider::AWS);
        assert_eq!(connection.target_provider, CloudProvider::Azure);
        assert_eq!(connection.bandwidth_mbps, 1000);
        assert!(connection.encrypted);
        assert!(!connection.redundant);
    }

    #[test]
    fn test_connection_with_latency() {
        let connection = CloudConnection::new(
            "conn",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        )
        .with_latency(50);

        assert_eq!(connection.latency_ms, Some(50));
    }

    #[test]
    fn test_connection_enable_redundancy() {
        let connection = CloudConnection::new(
            "conn",
            ConnectionType::ExpressRoute,
            CloudProvider::Azure,
            "eastus",
            CloudProvider::Azure,
            "westus",
            5000,
        )
        .enable_redundancy();

        assert!(connection.redundant);
    }

    #[test]
    fn test_connection_disable_encryption() {
        let connection = CloudConnection::new(
            "conn",
            ConnectionType::VPN,
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::GCP,
            "europe-west1",
            1000,
        )
        .disable_encryption();

        assert!(!connection.encrypted);
    }

    #[test]
    fn test_connection_set_status() {
        let mut connection = CloudConnection::new(
            "conn",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );

        connection.set_status(ConnectionStatus::Active);
        assert_eq!(connection.status, ConnectionStatus::Active);
    }

    #[test]
    fn test_connection_record_test() {
        let mut connection = CloudConnection::new(
            "conn",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        );

        connection.record_test(25);
        assert_eq!(connection.latency_ms, Some(25));
        assert!(connection.last_tested.is_some());
    }

    #[test]
    fn test_connection_is_active() {
        let mut connection = CloudConnection::new(
            "conn",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );

        assert!(!connection.is_active());

        connection.set_status(ConnectionStatus::Active);
        assert!(connection.is_active());
    }

    #[test]
    fn test_connection_is_healthy() {
        let mut connection = CloudConnection::new(
            "conn",
            ConnectionType::ExpressRoute,
            CloudProvider::Azure,
            "eastus",
            CloudProvider::AWS,
            "us-east-1",
            5000,
        );

        connection.set_status(ConnectionStatus::Active);
        assert!(connection.is_healthy());

        connection.set_status(ConnectionStatus::Degraded);
        assert!(connection.is_healthy());

        connection.set_status(ConnectionStatus::Failed);
        assert!(!connection.is_healthy());
    }

    #[test]
    fn test_connection_is_cross_provider() {
        let connection1 = CloudConnection::new(
            "cross",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );
        assert!(connection1.is_cross_provider());

        let connection2 = CloudConnection::new(
            "same",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        );
        assert!(!connection2.is_cross_provider());
    }

    #[test]
    fn test_network_tunnel() {
        let tunnel = NetworkTunnel::new(
            "vpn-tunnel",
            "conn-123",
            TunnelType::IPSec,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );

        assert_eq!(tunnel.name, "vpn-tunnel");
        assert_eq!(tunnel.connection_id, "conn-123");
        assert_eq!(tunnel.tunnel_type, TunnelType::IPSec);
        assert_eq!(tunnel.local_endpoint, "10.0.1.1");
        assert_eq!(tunnel.remote_endpoint, "20.0.1.1");
        assert_eq!(tunnel.status, TunnelStatus::Down);
    }

    #[test]
    fn test_tunnel_set_status() {
        let mut tunnel = NetworkTunnel::new(
            "tunnel",
            "conn-1",
            TunnelType::GRE,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );

        tunnel.set_status(TunnelStatus::Up);
        assert_eq!(tunnel.status, TunnelStatus::Up);
    }

    #[test]
    fn test_tunnel_is_up() {
        let mut tunnel = NetworkTunnel::new(
            "tunnel",
            "conn-1",
            TunnelType::VXLAN,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );

        assert!(!tunnel.is_up());

        tunnel.set_status(TunnelStatus::Up);
        assert!(tunnel.is_up());
    }

    #[test]
    fn test_connectivity_manager() {
        let mut manager = ConnectivityManager::new();

        let connection = CloudConnection::new(
            "conn",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );
        let id = manager.add_connection(connection);

        assert_eq!(manager.connection_count(), 1);
        assert!(manager.get_connection(&id).is_some());
    }

    #[test]
    fn test_manager_add_tunnel() {
        let mut manager = ConnectivityManager::new();

        let tunnel = NetworkTunnel::new(
            "tunnel",
            "conn-1",
            TunnelType::IPSec,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );
        let id = manager.add_tunnel(tunnel);

        assert_eq!(manager.tunnel_count(), 1);
        assert!(manager.get_tunnel(&id).is_some());
    }

    #[test]
    fn test_manager_active_connections() {
        let mut manager = ConnectivityManager::new();

        let mut connection1 = CloudConnection::new(
            "c1",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );
        connection1.set_status(ConnectionStatus::Active);

        let connection2 = CloudConnection::new(
            "c2",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        );

        manager.add_connection(connection1);
        manager.add_connection(connection2);

        let active = manager.active_connections();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_healthy_connections() {
        let mut manager = ConnectivityManager::new();

        let mut connection1 = CloudConnection::new(
            "c1",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );
        connection1.set_status(ConnectionStatus::Active);

        let mut connection2 = CloudConnection::new(
            "c2",
            ConnectionType::ExpressRoute,
            CloudProvider::Azure,
            "eastus",
            CloudProvider::AWS,
            "us-east-1",
            5000,
        );
        connection2.set_status(ConnectionStatus::Degraded);

        let mut connection3 = CloudConnection::new(
            "c3",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        );
        connection3.set_status(ConnectionStatus::Failed);

        manager.add_connection(connection1);
        manager.add_connection(connection2);
        manager.add_connection(connection3);

        let healthy = manager.healthy_connections();
        assert_eq!(healthy.len(), 2);
    }

    #[test]
    fn test_manager_cross_provider_connections() {
        let mut manager = ConnectivityManager::new();

        let connection1 = CloudConnection::new(
            "c1",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );
        let connection2 = CloudConnection::new(
            "c2",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        );

        manager.add_connection(connection1);
        manager.add_connection(connection2);

        let cross_provider = manager.cross_provider_connections();
        assert_eq!(cross_provider.len(), 1);
    }

    #[test]
    fn test_manager_connections_by_type() {
        let mut manager = ConnectivityManager::new();

        manager.add_connection(CloudConnection::new(
            "c1",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        ));
        manager.add_connection(CloudConnection::new(
            "c2",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        ));
        manager.add_connection(CloudConnection::new(
            "c3",
            ConnectionType::VPN,
            CloudProvider::GCP,
            "us-central1",
            CloudProvider::AWS,
            "us-east-1",
            1000,
        ));

        let vpns = manager.connections_by_type(&ConnectionType::VPN);
        assert_eq!(vpns.len(), 2);
    }

    #[test]
    fn test_manager_redundant_connections() {
        let mut manager = ConnectivityManager::new();

        let connection1 = CloudConnection::new(
            "c1",
            ConnectionType::DirectConnect,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::AWS,
            "us-west-2",
            10000,
        )
        .enable_redundancy();
        let connection2 = CloudConnection::new(
            "c2",
            ConnectionType::VPN,
            CloudProvider::AWS,
            "us-east-1",
            CloudProvider::Azure,
            "eastus",
            1000,
        );

        manager.add_connection(connection1);
        manager.add_connection(connection2);

        let redundant = manager.redundant_connections();
        assert_eq!(redundant.len(), 1);
    }

    #[test]
    fn test_manager_tunnels_for_connection() {
        let mut manager = ConnectivityManager::new();

        manager.add_tunnel(NetworkTunnel::new(
            "t1",
            "conn-1",
            TunnelType::IPSec,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        ));
        manager.add_tunnel(NetworkTunnel::new(
            "t2",
            "conn-2",
            TunnelType::GRE,
            "10.0.2.1",
            "20.0.2.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        ));
        manager.add_tunnel(NetworkTunnel::new(
            "t3",
            "conn-1",
            TunnelType::VXLAN,
            "10.0.3.1",
            "20.0.3.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        ));

        let tunnels = manager.tunnels_for_connection("conn-1");
        assert_eq!(tunnels.len(), 2);
    }

    #[test]
    fn test_manager_active_tunnels() {
        let mut manager = ConnectivityManager::new();

        let mut tunnel1 = NetworkTunnel::new(
            "t1",
            "conn-1",
            TunnelType::IPSec,
            "10.0.1.1",
            "20.0.1.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );
        tunnel1.set_status(TunnelStatus::Up);

        let tunnel2 = NetworkTunnel::new(
            "t2",
            "conn-2",
            TunnelType::GRE,
            "10.0.2.1",
            "20.0.2.1",
            "10.0.0.0/16",
            "20.0.0.0/16",
        );

        manager.add_tunnel(tunnel1);
        manager.add_tunnel(tunnel2);

        let active = manager.active_tunnels();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_connection_type_equality() {
        assert_eq!(ConnectionType::VPN, ConnectionType::VPN);
        assert_ne!(ConnectionType::VPN, ConnectionType::DirectConnect);
    }

    #[test]
    fn test_connection_status_equality() {
        assert_eq!(ConnectionStatus::Active, ConnectionStatus::Active);
        assert_ne!(ConnectionStatus::Active, ConnectionStatus::Failed);
    }

    #[test]
    fn test_tunnel_type_equality() {
        assert_eq!(TunnelType::IPSec, TunnelType::IPSec);
        assert_ne!(TunnelType::IPSec, TunnelType::GRE);
    }

    #[test]
    fn test_tunnel_status_equality() {
        assert_eq!(TunnelStatus::Up, TunnelStatus::Up);
        assert_ne!(TunnelStatus::Up, TunnelStatus::Down);
    }
}
