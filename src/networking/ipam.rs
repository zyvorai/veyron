use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

/// IP allocation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationStatus {
    Available,
    Allocated,
    Reserved,
    Deprecated,
}

/// IP address pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPPool {
    pub id: String,
    pub name: String,
    pub cidr: String,
    pub gateway: Option<IpAddr>,
    pub dns_servers: Vec<IpAddr>,
    pub start_ip: IpAddr,
    pub end_ip: IpAddr,
    pub allocated_count: u32,
    pub total_capacity: u32,
    pub created_at: DateTime<Utc>,
}

impl IPPool {
    pub fn new(
        name: impl Into<String>,
        cidr: impl Into<String>,
        start_ip: IpAddr,
        end_ip: IpAddr,
        capacity: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "pool-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            cidr: cidr.into(),
            gateway: None,
            dns_servers: Vec::new(),
            start_ip,
            end_ip,
            allocated_count: 0,
            total_capacity: capacity,
            created_at: Utc::now(),
        }
    }

    pub fn with_gateway(mut self, gateway: IpAddr) -> Self {
        self.gateway = Some(gateway);
        self
    }

    pub fn add_dns_server(&mut self, dns: IpAddr) {
        self.dns_servers.push(dns);
    }

    pub fn available_count(&self) -> u32 {
        self.total_capacity.saturating_sub(self.allocated_count)
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        (self.allocated_count as f64 / self.total_capacity as f64) * 100.0
    }

    pub fn is_exhausted(&self) -> bool {
        self.allocated_count >= self.total_capacity
    }

    pub fn is_nearly_exhausted(&self) -> bool {
        self.utilization_percent() > 90.0
    }
}

/// IP allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPAllocation {
    pub id: String,
    pub pool_id: String,
    pub ip_address: IpAddr,
    pub mac_address: Option<String>,
    pub hostname: Option<String>,
    pub owner: String,
    pub status: AllocationStatus,
    pub allocated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl IPAllocation {
    pub fn new(pool_id: impl Into<String>, ip_address: IpAddr, owner: impl Into<String>) -> Self {
        let pool_id_str = pool_id.into();
        let id = format!("alloc-{}-{}", pool_id_str, Utc::now().timestamp_micros());

        Self {
            id,
            pool_id: pool_id_str,
            ip_address,
            mac_address: None,
            hostname: None,
            owner: owner.into(),
            status: AllocationStatus::Allocated,
            allocated_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_mac(mut self, mac: impl Into<String>) -> Self {
        self.mac_address = Some(mac.into());
        self
    }

    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn release(&mut self) {
        self.status = AllocationStatus::Available;
    }

    pub fn reserve(&mut self) {
        self.status = AllocationStatus::Reserved;
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == AllocationStatus::Allocated && !self.is_expired()
    }
}

/// Subnet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subnet {
    pub id: String,
    pub name: String,
    pub cidr: String,
    pub network: String,
    pub netmask: String,
    pub broadcast: Option<String>,
    pub vlan_id: Option<u16>,
    pub zone: Option<String>,
    pub pool_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Subnet {
    pub fn new(
        name: impl Into<String>,
        cidr: impl Into<String>,
        network: impl Into<String>,
        netmask: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "subnet-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            cidr: cidr.into(),
            network: network.into(),
            netmask: netmask.into(),
            broadcast: None,
            vlan_id: None,
            zone: None,
            pool_ids: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_broadcast(mut self, broadcast: impl Into<String>) -> Self {
        self.broadcast = Some(broadcast.into());
        self
    }

    pub fn with_vlan(mut self, vlan_id: u16) -> Self {
        self.vlan_id = Some(vlan_id);
        self
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }

    pub fn add_pool(&mut self, pool_id: impl Into<String>) {
        self.pool_ids.push(pool_id.into());
    }

    pub fn pool_count(&self) -> usize {
        self.pool_ids.len()
    }
}

/// IPAM manager
pub struct IPAMManager {
    pools: HashMap<String, IPPool>,
    allocations: HashMap<String, IPAllocation>,
    subnets: HashMap<String, Subnet>,
}

impl IPAMManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            allocations: HashMap::new(),
            subnets: HashMap::new(),
        }
    }

    pub fn add_pool(&mut self, pool: IPPool) -> String {
        let id = pool.id.clone();
        self.pools.insert(id.clone(), pool);
        id
    }

    pub fn get_pool(&self, id: &str) -> Option<&IPPool> {
        self.pools.get(id)
    }

    pub fn get_pool_mut(&mut self, id: &str) -> Option<&mut IPPool> {
        self.pools.get_mut(id)
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    pub fn add_allocation(&mut self, allocation: IPAllocation) -> anyhow::Result<String> {
        // Check for duplicate IP in the same pool (skip released/expired allocations)
        let ip = &allocation.ip_address;
        for existing in self.allocations.values() {
            if existing.ip_address == *ip
                && existing.pool_id == allocation.pool_id
                && existing.status == AllocationStatus::Allocated
                && !existing.is_expired()
            {
                anyhow::bail!(
                    "IP address {} is already allocated in pool {}",
                    ip,
                    allocation.pool_id
                );
            }
        }
        let pool_id = allocation.pool_id.clone();
        let id = allocation.id.clone();
        self.allocations.insert(id.clone(), allocation);
        // Update pool allocated count
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            pool.allocated_count += 1;
        }
        Ok(id)
    }

    pub fn get_allocation(&self, id: &str) -> Option<&IPAllocation> {
        self.allocations.get(id)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn add_subnet(&mut self, subnet: Subnet) -> String {
        let id = subnet.id.clone();
        self.subnets.insert(id.clone(), subnet);
        id
    }

    pub fn get_subnet(&self, id: &str) -> Option<&Subnet> {
        self.subnets.get(id)
    }

    pub fn subnet_count(&self) -> usize {
        self.subnets.len()
    }

    pub fn allocations_by_pool(&self, pool_id: &str) -> Vec<&IPAllocation> {
        self.allocations
            .values()
            .filter(|a| a.pool_id == pool_id)
            .collect()
    }

    pub fn allocations_by_owner(&self, owner: &str) -> Vec<&IPAllocation> {
        self.allocations
            .values()
            .filter(|a| a.owner == owner)
            .collect()
    }

    pub fn active_allocations(&self) -> Vec<&IPAllocation> {
        self.allocations
            .values()
            .filter(|a| a.is_active())
            .collect()
    }

    pub fn expired_allocations(&self) -> Vec<&IPAllocation> {
        self.allocations
            .values()
            .filter(|a| a.is_expired())
            .collect()
    }

    pub fn exhausted_pools(&self) -> Vec<&IPPool> {
        self.pools.values().filter(|p| p.is_exhausted()).collect()
    }

    pub fn nearly_exhausted_pools(&self) -> Vec<&IPPool> {
        self.pools
            .values()
            .filter(|p| p.is_nearly_exhausted())
            .collect()
    }

    pub fn total_capacity(&self) -> u32 {
        self.pools.values().map(|p| p.total_capacity).sum()
    }

    pub fn total_allocated(&self) -> u32 {
        self.pools.values().map(|p| p.allocated_count).sum()
    }
}

impl Default for IPAMManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_ip_pool() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 244);

        assert_eq!(pool.name, "test-pool");
        assert_eq!(pool.cidr, "10.0.1.0/24");
        assert_eq!(pool.total_capacity, 244);
        assert_eq!(pool.allocated_count, 0);
    }

    #[test]
    fn test_pool_with_gateway() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));
        let gateway = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1));

        let pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 244).with_gateway(gateway);

        assert_eq!(pool.gateway, Some(gateway));
    }

    #[test]
    fn test_pool_add_dns_server() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 244);

        pool.add_dns_server(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)));
        pool.add_dns_server(IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4)));

        assert_eq!(pool.dns_servers.len(), 2);
    }

    #[test]
    fn test_pool_available_count() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 100);

        assert_eq!(pool.available_count(), 100);

        pool.allocated_count = 30;
        assert_eq!(pool.available_count(), 70);
    }

    #[test]
    fn test_pool_utilization_percent() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 100);

        assert_eq!(pool.utilization_percent(), 0.0);

        pool.allocated_count = 50;
        assert_eq!(pool.utilization_percent(), 50.0);
    }

    #[test]
    fn test_pool_is_exhausted() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 100);

        assert!(!pool.is_exhausted());

        pool.allocated_count = 100;
        assert!(pool.is_exhausted());
    }

    #[test]
    fn test_pool_is_nearly_exhausted() {
        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 100);

        assert!(!pool.is_nearly_exhausted());

        pool.allocated_count = 95;
        assert!(pool.is_nearly_exhausted());
    }

    #[test]
    fn test_ip_allocation() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let allocation = IPAllocation::new("pool-1", ip, "vm-123");

        assert_eq!(allocation.pool_id, "pool-1");
        assert_eq!(allocation.ip_address, ip);
        assert_eq!(allocation.owner, "vm-123");
        assert_eq!(allocation.status, AllocationStatus::Allocated);
    }

    #[test]
    fn test_allocation_with_mac() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let allocation = IPAllocation::new("pool-1", ip, "vm-1").with_mac("00:11:22:33:44:55");

        assert_eq!(
            allocation.mac_address,
            Some("00:11:22:33:44:55".to_string())
        );
    }

    #[test]
    fn test_allocation_with_hostname() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let allocation =
            IPAllocation::new("pool-1", ip, "vm-1").with_hostname("web-server.example.com");

        assert_eq!(
            allocation.hostname,
            Some("web-server.example.com".to_string())
        );
    }

    #[test]
    fn test_allocation_with_expiry() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let expiry = Utc::now() + chrono::TimeDelta::hours(24);
        let allocation = IPAllocation::new("pool-1", ip, "vm-1").with_expiry(expiry);

        assert_eq!(allocation.expires_at, Some(expiry));
    }

    #[test]
    fn test_allocation_release() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let mut allocation = IPAllocation::new("pool-1", ip, "vm-1");

        allocation.release();
        assert_eq!(allocation.status, AllocationStatus::Available);
    }

    #[test]
    fn test_allocation_reserve() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let mut allocation = IPAllocation::new("pool-1", ip, "vm-1");

        allocation.reserve();
        assert_eq!(allocation.status, AllocationStatus::Reserved);
    }

    #[test]
    fn test_allocation_is_expired() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));

        let allocation1 = IPAllocation::new("pool-1", ip, "vm-1");
        assert!(!allocation1.is_expired());

        let past_expiry = Utc::now() - chrono::TimeDelta::hours(1);
        let allocation2 = IPAllocation::new("pool-1", ip, "vm-2").with_expiry(past_expiry);
        assert!(allocation2.is_expired());
    }

    #[test]
    fn test_allocation_is_active() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));

        let allocation1 = IPAllocation::new("pool-1", ip, "vm-1");
        assert!(allocation1.is_active());

        let mut allocation2 = IPAllocation::new("pool-1", ip, "vm-2");
        allocation2.release();
        assert!(!allocation2.is_active());
    }

    #[test]
    fn test_subnet() {
        let subnet = Subnet::new("prod-subnet", "10.0.1.0/24", "10.0.1.0", "255.255.255.0");

        assert_eq!(subnet.name, "prod-subnet");
        assert_eq!(subnet.cidr, "10.0.1.0/24");
        assert_eq!(subnet.network, "10.0.1.0");
        assert_eq!(subnet.netmask, "255.255.255.0");
    }

    #[test]
    fn test_subnet_with_broadcast() {
        let subnet = Subnet::new("test", "10.0.1.0/24", "10.0.1.0", "255.255.255.0")
            .with_broadcast("10.0.1.255");

        assert_eq!(subnet.broadcast, Some("10.0.1.255".to_string()));
    }

    #[test]
    fn test_subnet_with_vlan() {
        let subnet = Subnet::new("test", "10.0.1.0/24", "10.0.1.0", "255.255.255.0").with_vlan(100);

        assert_eq!(subnet.vlan_id, Some(100));
    }

    #[test]
    fn test_subnet_with_zone() {
        let subnet =
            Subnet::new("test", "10.0.1.0/24", "10.0.1.0", "255.255.255.0").with_zone("us-west-1a");

        assert_eq!(subnet.zone, Some("us-west-1a".to_string()));
    }

    #[test]
    fn test_subnet_add_pool() {
        let mut subnet = Subnet::new("test", "10.0.1.0/24", "10.0.1.0", "255.255.255.0");

        subnet.add_pool("pool-1");
        subnet.add_pool("pool-2");

        assert_eq!(subnet.pool_count(), 2);
    }

    #[test]
    fn test_ipam_manager() {
        let mut manager = IPAMManager::new();

        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));
        let pool = IPPool::new("test-pool", "10.0.1.0/24", start, end, 244);

        let id = manager.add_pool(pool);

        assert_eq!(manager.pool_count(), 1);
        assert!(manager.get_pool(&id).is_some());
    }

    #[test]
    fn test_manager_add_allocation() {
        let mut manager = IPAMManager::new();

        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let allocation = IPAllocation::new("pool-1", ip, "vm-1");

        let id = manager.add_allocation(allocation).unwrap();

        assert_eq!(manager.allocation_count(), 1);
        assert!(manager.get_allocation(&id).is_some());
    }

    #[test]
    fn test_manager_duplicate_allocation() {
        let mut manager = IPAMManager::new();

        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let alloc1 = IPAllocation::new("pool-1", ip, "vm-1");
        let alloc2 = IPAllocation::new("pool-1", ip, "vm-2");

        manager.add_allocation(alloc1).unwrap();
        assert!(manager.add_allocation(alloc2).is_err());
    }

    #[test]
    fn test_manager_same_ip_different_pool() {
        let mut manager = IPAMManager::new();

        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let alloc1 = IPAllocation::new("pool-1", ip, "vm-1");
        let alloc2 = IPAllocation::new("pool-2", ip, "vm-2");

        manager.add_allocation(alloc1).unwrap();
        manager.add_allocation(alloc2).unwrap();
        assert_eq!(manager.allocation_count(), 2);
    }

    #[test]
    fn test_manager_add_subnet() {
        let mut manager = IPAMManager::new();

        let subnet = Subnet::new("test", "10.0.1.0/24", "10.0.1.0", "255.255.255.0");
        let id = manager.add_subnet(subnet);

        assert_eq!(manager.subnet_count(), 1);
        assert!(manager.get_subnet(&id).is_some());
    }

    #[test]
    fn test_manager_allocations_by_pool() {
        let mut manager = IPAMManager::new();

        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11));
        let ip3 = IpAddr::V4(Ipv4Addr::new(10, 0, 2, 10));

        manager
            .add_allocation(IPAllocation::new("pool-1", ip1, "vm-1"))
            .unwrap();
        manager
            .add_allocation(IPAllocation::new("pool-2", ip2, "vm-2"))
            .unwrap();
        manager
            .add_allocation(IPAllocation::new("pool-1", ip3, "vm-3"))
            .unwrap();

        let pool1_allocs = manager.allocations_by_pool("pool-1");
        assert_eq!(pool1_allocs.len(), 2);
    }

    #[test]
    fn test_manager_allocations_by_owner() {
        let mut manager = IPAMManager::new();

        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11));

        manager
            .add_allocation(IPAllocation::new("pool-1", ip1, "vm-1"))
            .unwrap();
        manager
            .add_allocation(IPAllocation::new("pool-1", ip2, "vm-1"))
            .unwrap();

        let vm1_allocs = manager.allocations_by_owner("vm-1");
        assert_eq!(vm1_allocs.len(), 2);
    }

    #[test]
    fn test_manager_active_allocations() {
        let mut manager = IPAMManager::new();

        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11));

        let mut alloc1 = IPAllocation::new("pool-1", ip1, "vm-1");
        alloc1.release();

        manager.add_allocation(alloc1).unwrap();
        manager
            .add_allocation(IPAllocation::new("pool-1", ip2, "vm-2"))
            .unwrap();

        let active = manager.active_allocations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_expired_allocations() {
        let mut manager = IPAMManager::new();

        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11));

        let past_expiry = Utc::now() - chrono::TimeDelta::hours(1);

        manager
            .add_allocation(IPAllocation::new("pool-1", ip1, "vm-1").with_expiry(past_expiry))
            .unwrap();
        manager
            .add_allocation(IPAllocation::new("pool-1", ip2, "vm-2"))
            .unwrap();

        let expired = manager.expired_allocations();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_manager_exhausted_pools() {
        let mut manager = IPAMManager::new();

        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool1 = IPPool::new("pool-1", "10.0.1.0/24", start, end, 100);
        pool1.allocated_count = 100;

        let pool2 = IPPool::new("pool-2", "10.0.2.0/24", start, end, 100);

        manager.add_pool(pool1);
        manager.add_pool(pool2);

        let exhausted = manager.exhausted_pools();
        assert_eq!(exhausted.len(), 1);
    }

    #[test]
    fn test_manager_nearly_exhausted_pools() {
        let mut manager = IPAMManager::new();

        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool1 = IPPool::new("pool-1", "10.0.1.0/24", start, end, 100);
        pool1.allocated_count = 95;

        let pool2 = IPPool::new("pool-2", "10.0.2.0/24", start, end, 100);

        manager.add_pool(pool1);
        manager.add_pool(pool2);

        let nearly_exhausted = manager.nearly_exhausted_pools();
        assert_eq!(nearly_exhausted.len(), 1);
    }

    #[test]
    fn test_manager_total_capacity() {
        let mut manager = IPAMManager::new();

        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        manager.add_pool(IPPool::new("pool-1", "10.0.1.0/24", start, end, 100));
        manager.add_pool(IPPool::new("pool-2", "10.0.2.0/24", start, end, 150));

        assert_eq!(manager.total_capacity(), 250);
    }

    #[test]
    fn test_manager_total_allocated() {
        let mut manager = IPAMManager::new();

        let start = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10));
        let end = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 254));

        let mut pool1 = IPPool::new("pool-1", "10.0.1.0/24", start, end, 100);
        pool1.allocated_count = 30;

        let mut pool2 = IPPool::new("pool-2", "10.0.2.0/24", start, end, 150);
        pool2.allocated_count = 50;

        manager.add_pool(pool1);
        manager.add_pool(pool2);

        assert_eq!(manager.total_allocated(), 80);
    }

    #[test]
    fn test_allocation_status_equality() {
        assert_eq!(AllocationStatus::Allocated, AllocationStatus::Allocated);
        assert_ne!(AllocationStatus::Allocated, AllocationStatus::Available);
    }
}
