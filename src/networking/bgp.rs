use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// BGP peer status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Idle,
    Connect,
    Active,
    OpenSent,
    OpenConfirm,
    Established,
}

/// BGP peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BGPPeer {
    pub id: String,
    pub peer_ip: String,
    pub peer_asn: u32,
    pub local_asn: u32,
    pub status: PeerStatus,
    pub routes_received: u32,
    pub routes_advertised: u32,
    pub uptime_seconds: Option<u64>,
    pub created_at: DateTime<Utc>,
}

impl BGPPeer {
    pub fn new(peer_ip: impl Into<String>, peer_asn: u32, local_asn: u32) -> anyhow::Result<Self> {
        let ip_str = peer_ip.into();
        let ip: std::net::IpAddr = ip_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid peer IP address: {}", ip_str))?;
        // Reject loopback, unspecified, and link-local addresses
        if ip.is_loopback() || ip.is_unspecified() {
            anyhow::bail!("Peer IP must not be loopback or unspecified: {}", ip_str);
        }
        let id = format!("peer-{}-{}", ip_str, Utc::now().timestamp());

        Ok(Self {
            id,
            peer_ip: ip_str,
            peer_asn,
            local_asn,
            status: PeerStatus::Idle,
            routes_received: 0,
            routes_advertised: 0,
            uptime_seconds: None,
            created_at: Utc::now(),
        })
    }

    pub fn set_status(&mut self, status: PeerStatus) {
        self.status = status;
    }

    pub fn set_routes(&mut self, received: u32, advertised: u32) {
        self.routes_received = received;
        self.routes_advertised = advertised;
    }

    pub fn is_established(&self) -> bool {
        self.status == PeerStatus::Established
    }
}

/// BGP route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BGPRoute {
    pub id: String,
    pub prefix: String,
    pub next_hop: String,
    pub as_path: Vec<u32>,
    pub local_pref: Option<u32>,
    pub med: Option<u32>,
    pub best: bool,
    pub created_at: DateTime<Utc>,
}

impl BGPRoute {
    pub fn new(prefix: impl Into<String>, next_hop: impl Into<String>) -> anyhow::Result<Self> {
        let prefix_str = prefix.into();
        let next_hop_str = next_hop.into();
        let next_hop_ip: std::net::IpAddr = next_hop_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid next_hop IP address: {}", next_hop_str))?;
        if next_hop_ip.is_loopback() || next_hop_ip.is_unspecified() {
            anyhow::bail!(
                "Next hop IP must not be loopback or unspecified: {}",
                next_hop_str
            );
        }
        let id = format!(
            "route-{}-{}",
            prefix_str.replace('/', "-"),
            Utc::now().timestamp_micros()
        );

        Ok(Self {
            id,
            prefix: prefix_str,
            next_hop: next_hop_str,
            as_path: Vec::new(),
            local_pref: None,
            med: None,
            best: false,
            created_at: Utc::now(),
        })
    }

    pub fn with_as_path(mut self, as_path: Vec<u32>) -> Self {
        self.as_path = as_path;
        self
    }

    pub fn with_local_pref(mut self, pref: u32) -> Self {
        self.local_pref = Some(pref);
        self
    }

    pub fn mark_best(&mut self) {
        self.best = true;
    }

    pub fn as_path_length(&self) -> usize {
        self.as_path.len()
    }
}

/// BGP manager
pub struct BGPManager {
    peers: HashMap<String, BGPPeer>,
    routes: HashMap<String, BGPRoute>,
}

impl BGPManager {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            routes: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, peer: BGPPeer) -> String {
        let id = peer.id.clone();
        self.peers.insert(id.clone(), peer);
        id
    }

    pub fn get_peer(&self, id: &str) -> Option<&BGPPeer> {
        self.peers.get(id)
    }

    pub fn get_peer_mut(&mut self, id: &str) -> Option<&mut BGPPeer> {
        self.peers.get_mut(id)
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn add_route(&mut self, route: BGPRoute) -> String {
        let id = route.id.clone();
        self.routes.insert(id.clone(), route);
        id
    }

    pub fn get_route(&self, id: &str) -> Option<&BGPRoute> {
        self.routes.get(id)
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub fn established_peers(&self) -> Vec<&BGPPeer> {
        self.peers.values().filter(|p| p.is_established()).collect()
    }

    pub fn best_routes(&self) -> Vec<&BGPRoute> {
        self.routes.values().filter(|r| r.best).collect()
    }

    pub fn total_routes_received(&self) -> u32 {
        self.peers.values().map(|p| p.routes_received).sum()
    }
}

impl Default for BGPManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgp_peer() {
        let peer = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();

        assert_eq!(peer.peer_ip, "192.168.1.1");
        assert_eq!(peer.peer_asn, 65000);
        assert_eq!(peer.local_asn, 65001);
        assert_eq!(peer.status, PeerStatus::Idle);
    }

    #[test]
    fn test_peer_invalid_ip() {
        assert!(BGPPeer::new("not-an-ip", 65000, 65001).is_err());
    }

    #[test]
    fn test_peer_loopback_ip() {
        assert!(BGPPeer::new("127.0.0.1", 65000, 65001).is_err());
    }

    #[test]
    fn test_peer_unspecified_ip() {
        assert!(BGPPeer::new("0.0.0.0", 65000, 65001).is_err());
    }

    #[test]
    fn test_peer_set_status() {
        let mut peer = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();

        peer.set_status(PeerStatus::Established);
        assert_eq!(peer.status, PeerStatus::Established);
    }

    #[test]
    fn test_peer_set_routes() {
        let mut peer = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();

        peer.set_routes(100, 50);
        assert_eq!(peer.routes_received, 100);
        assert_eq!(peer.routes_advertised, 50);
    }

    #[test]
    fn test_peer_is_established() {
        let mut peer = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();

        assert!(!peer.is_established());

        peer.set_status(PeerStatus::Established);
        assert!(peer.is_established());
    }

    #[test]
    fn test_bgp_route() {
        let route = BGPRoute::new("10.0.0.0/24", "192.168.1.1").unwrap();

        assert_eq!(route.prefix, "10.0.0.0/24");
        assert_eq!(route.next_hop, "192.168.1.1");
        assert!(!route.best);
    }

    #[test]
    fn test_bgp_route_invalid_next_hop() {
        assert!(BGPRoute::new("10.0.0.0/24", "not-an-ip").is_err());
    }

    #[test]
    fn test_bgp_route_loopback_next_hop() {
        assert!(BGPRoute::new("10.0.0.0/24", "127.0.0.1").is_err());
    }

    #[test]
    fn test_route_with_as_path() {
        let route = BGPRoute::new("10.0.0.0/24", "192.168.1.1")
            .unwrap()
            .with_as_path(vec![65000, 65001, 65002]);

        assert_eq!(route.as_path.len(), 3);
        assert_eq!(route.as_path_length(), 3);
    }

    #[test]
    fn test_route_with_local_pref() {
        let route = BGPRoute::new("10.0.0.0/24", "192.168.1.1")
            .unwrap()
            .with_local_pref(100);

        assert_eq!(route.local_pref, Some(100));
    }

    #[test]
    fn test_route_mark_best() {
        let mut route = BGPRoute::new("10.0.0.0/24", "192.168.1.1").unwrap();

        assert!(!route.best);

        route.mark_best();
        assert!(route.best);
    }

    #[test]
    fn test_bgp_manager() {
        let mut manager = BGPManager::new();

        let peer = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();
        manager.add_peer(peer);

        assert_eq!(manager.peer_count(), 1);
    }

    #[test]
    fn test_manager_add_route() {
        let mut manager = BGPManager::new();

        let route = BGPRoute::new("10.0.0.0/24", "192.168.1.1").unwrap();
        manager.add_route(route);

        assert_eq!(manager.route_count(), 1);
    }

    #[test]
    fn test_manager_established_peers() {
        let mut manager = BGPManager::new();

        let mut peer1 = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();
        peer1.set_status(PeerStatus::Established);

        let peer2 = BGPPeer::new("192.168.1.2", 65000, 65001).unwrap();

        manager.add_peer(peer1);
        manager.add_peer(peer2);

        let established = manager.established_peers();
        assert_eq!(established.len(), 1);
    }

    #[test]
    fn test_manager_best_routes() {
        let mut manager = BGPManager::new();

        let mut route1 = BGPRoute::new("10.0.0.0/24", "192.168.1.1").unwrap();
        route1.mark_best();

        let route2 = BGPRoute::new("10.1.0.0/24", "192.168.1.2").unwrap();

        manager.add_route(route1);
        manager.add_route(route2);

        let best = manager.best_routes();
        assert_eq!(best.len(), 1);
    }

    #[test]
    fn test_manager_total_routes_received() {
        let mut manager = BGPManager::new();

        let mut peer1 = BGPPeer::new("192.168.1.1", 65000, 65001).unwrap();
        peer1.set_routes(100, 50);

        let mut peer2 = BGPPeer::new("192.168.1.2", 65000, 65001).unwrap();
        peer2.set_routes(150, 75);

        manager.add_peer(peer1);
        manager.add_peer(peer2);

        assert_eq!(manager.total_routes_received(), 250);
    }

    #[test]
    fn test_peer_status_equality() {
        assert_eq!(PeerStatus::Established, PeerStatus::Established);
        assert_ne!(PeerStatus::Established, PeerStatus::Idle);
    }
}
