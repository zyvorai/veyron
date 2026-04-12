use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod analysis;
pub mod forecasting;
pub mod optimization;
pub mod planning;
pub mod rightsizing;

/// Resource type for capacity planning
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    CPU,
    Memory,
    Storage,
    Network,
    GPU,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::CPU => write!(f, "CPU"),
            ResourceType::Memory => write!(f, "Memory"),
            ResourceType::Storage => write!(f, "Storage"),
            ResourceType::Network => write!(f, "Network"),
            ResourceType::GPU => write!(f, "GPU"),
        }
    }
}

/// Capacity status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapacityStatus {
    Available,
    Warning,
    Critical,
    Exhausted,
}

impl std::fmt::Display for CapacityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapacityStatus::Available => write!(f, "Available"),
            CapacityStatus::Warning => write!(f, "Warning"),
            CapacityStatus::Critical => write!(f, "Critical"),
            CapacityStatus::Exhausted => write!(f, "Exhausted"),
        }
    }
}

/// Resource capacity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapacity {
    pub resource_type: ResourceType,
    pub total_capacity: f64,
    pub used_capacity: f64,
    pub reserved_capacity: f64,
    pub unit: String,
    pub status: CapacityStatus,
    pub last_updated: DateTime<Utc>,
}

impl ResourceCapacity {
    pub fn new(resource_type: ResourceType, total: f64, unit: impl Into<String>) -> Self {
        Self {
            resource_type,
            total_capacity: total,
            used_capacity: 0.0,
            reserved_capacity: 0.0,
            unit: unit.into(),
            status: CapacityStatus::Available,
            last_updated: Utc::now(),
        }
    }

    pub fn available_capacity(&self) -> f64 {
        self.total_capacity - self.used_capacity - self.reserved_capacity
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.total_capacity == 0.0 {
            return 0.0;
        }
        (self.used_capacity / self.total_capacity) * 100.0
    }

    pub fn update_usage(&mut self, used: f64) {
        self.used_capacity = used;
        self.last_updated = Utc::now();
        self.update_status();
    }

    pub fn reserve(&mut self, amount: f64) -> bool {
        if self.available_capacity() >= amount {
            self.reserved_capacity += amount;
            self.last_updated = Utc::now();
            self.update_status();
            true
        } else {
            false
        }
    }

    pub fn release(&mut self, amount: f64) {
        self.reserved_capacity = (self.reserved_capacity - amount).max(0.0);
        self.last_updated = Utc::now();
        self.update_status();
    }

    fn update_status(&mut self) {
        let utilization = self.utilization_percent();
        self.status = if utilization >= 95.0 {
            CapacityStatus::Exhausted
        } else if utilization >= 85.0 {
            CapacityStatus::Critical
        } else if utilization >= 70.0 {
            CapacityStatus::Warning
        } else {
            CapacityStatus::Available
        };
    }

    pub fn is_healthy(&self) -> bool {
        matches!(
            self.status,
            CapacityStatus::Available | CapacityStatus::Warning
        )
    }
}

/// Workload profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadProfile {
    pub id: String,
    pub name: String,
    pub workload_type: String,
    pub resource_requirements: HashMap<ResourceType, f64>,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
}

impl WorkloadProfile {
    pub fn new(name: impl Into<String>, workload_type: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "workload-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            workload_type: workload_type.into(),
            resource_requirements: HashMap::new(),
            priority: 5,
            created_at: Utc::now(),
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn add_requirement(&mut self, resource_type: ResourceType, amount: f64) {
        self.resource_requirements.insert(resource_type, amount);
    }

    pub fn get_requirement(&self, resource_type: &ResourceType) -> Option<f64> {
        self.resource_requirements.get(resource_type).copied()
    }

    pub fn total_cpu(&self) -> f64 {
        self.get_requirement(&ResourceType::CPU).unwrap_or(0.0)
    }

    pub fn total_memory(&self) -> f64 {
        self.get_requirement(&ResourceType::Memory).unwrap_or(0.0)
    }
}

/// Capacity manager
pub struct CapacityManager {
    capacities: HashMap<String, ResourceCapacity>,
    workloads: HashMap<String, WorkloadProfile>,
}

impl CapacityManager {
    pub fn new() -> Self {
        Self {
            capacities: HashMap::new(),
            workloads: HashMap::new(),
        }
    }

    pub fn add_capacity(&mut self, cluster: impl Into<String>, capacity: ResourceCapacity) {
        let key = format!("{}-{}", cluster.into(), capacity.resource_type);
        self.capacities.insert(key, capacity);
    }

    pub fn get_capacity(
        &self,
        cluster: &str,
        resource_type: &ResourceType,
    ) -> Option<&ResourceCapacity> {
        let key = format!("{}-{}", cluster, resource_type);
        self.capacities.get(&key)
    }

    pub fn get_capacity_mut(
        &mut self,
        cluster: &str,
        resource_type: &ResourceType,
    ) -> Option<&mut ResourceCapacity> {
        let key = format!("{}-{}", cluster, resource_type);
        self.capacities.get_mut(&key)
    }

    pub fn capacity_count(&self) -> usize {
        self.capacities.len()
    }

    pub fn add_workload(&mut self, workload: WorkloadProfile) -> String {
        let id = workload.id.clone();
        self.workloads.insert(id.clone(), workload);
        id
    }

    pub fn get_workload(&self, id: &str) -> Option<&WorkloadProfile> {
        self.workloads.get(id)
    }

    pub fn remove_workload(&mut self, id: &str) -> bool {
        self.workloads.remove(id).is_some()
    }

    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    pub fn critical_capacities(&self) -> Vec<&ResourceCapacity> {
        self.capacities
            .values()
            .filter(|c| {
                c.status == CapacityStatus::Critical || c.status == CapacityStatus::Exhausted
            })
            .collect()
    }

    pub fn healthy_capacities(&self) -> Vec<&ResourceCapacity> {
        self.capacities
            .values()
            .filter(|c| c.is_healthy())
            .collect()
    }

    pub fn by_resource_type(&self, resource_type: &ResourceType) -> Vec<&ResourceCapacity> {
        self.capacities
            .values()
            .filter(|c| &c.resource_type == resource_type)
            .collect()
    }

    pub fn high_priority_workloads(&self) -> Vec<&WorkloadProfile> {
        self.workloads
            .values()
            .filter(|w| w.priority >= 8)
            .collect()
    }
}

impl Default for CapacityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::CPU.to_string(), "CPU");
        assert_eq!(ResourceType::Memory.to_string(), "Memory");
        assert_eq!(ResourceType::Storage.to_string(), "Storage");
    }

    #[test]
    fn test_capacity_status_display() {
        assert_eq!(CapacityStatus::Available.to_string(), "Available");
        assert_eq!(CapacityStatus::Warning.to_string(), "Warning");
        assert_eq!(CapacityStatus::Critical.to_string(), "Critical");
    }

    #[test]
    fn test_resource_capacity() {
        let capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");

        assert_eq!(capacity.resource_type, ResourceType::CPU);
        assert_eq!(capacity.total_capacity, 100.0);
        assert_eq!(capacity.used_capacity, 0.0);
        assert_eq!(capacity.unit, "cores");
        assert_eq!(capacity.status, CapacityStatus::Available);
    }

    #[test]
    fn test_capacity_available() {
        let mut capacity = ResourceCapacity::new(ResourceType::Memory, 1000.0, "GB");
        capacity.used_capacity = 300.0;
        capacity.reserved_capacity = 200.0;

        assert_eq!(capacity.available_capacity(), 500.0);
    }

    #[test]
    fn test_capacity_utilization() {
        let mut capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");
        capacity.used_capacity = 75.0;

        assert_eq!(capacity.utilization_percent(), 75.0);
    }

    #[test]
    fn test_capacity_update_usage() {
        let mut capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");

        capacity.update_usage(50.0);
        assert_eq!(capacity.used_capacity, 50.0);
        assert_eq!(capacity.status, CapacityStatus::Available);

        capacity.update_usage(72.0);
        assert_eq!(capacity.status, CapacityStatus::Warning);

        capacity.update_usage(87.0);
        assert_eq!(capacity.status, CapacityStatus::Critical);

        capacity.update_usage(96.0);
        assert_eq!(capacity.status, CapacityStatus::Exhausted);
    }

    #[test]
    fn test_capacity_reserve() {
        let mut capacity = ResourceCapacity::new(ResourceType::Memory, 1000.0, "GB");
        capacity.used_capacity = 500.0;

        assert!(capacity.reserve(300.0));
        assert_eq!(capacity.reserved_capacity, 300.0);
        assert_eq!(capacity.available_capacity(), 200.0);

        assert!(!capacity.reserve(300.0)); // Should fail - not enough capacity
    }

    #[test]
    fn test_capacity_release() {
        let mut capacity = ResourceCapacity::new(ResourceType::Storage, 1000.0, "TB");
        capacity.reserved_capacity = 500.0;

        capacity.release(200.0);
        assert_eq!(capacity.reserved_capacity, 300.0);

        capacity.release(400.0);
        assert_eq!(capacity.reserved_capacity, 0.0); // Should not go negative
    }

    #[test]
    fn test_capacity_is_healthy() {
        let mut capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");

        capacity.update_usage(50.0);
        assert!(capacity.is_healthy());

        capacity.update_usage(72.0);
        assert!(capacity.is_healthy());

        capacity.update_usage(87.0);
        assert!(!capacity.is_healthy());
    }

    #[test]
    fn test_workload_profile() {
        let profile = WorkloadProfile::new("Web Server", "HTTP");

        assert_eq!(profile.name, "Web Server");
        assert_eq!(profile.workload_type, "HTTP");
        assert_eq!(profile.priority, 5);
        assert_eq!(profile.resource_requirements.len(), 0);
    }

    #[test]
    fn test_workload_builder() {
        let profile = WorkloadProfile::new("Database", "PostgreSQL").with_priority(9);

        assert_eq!(profile.priority, 9);
    }

    #[test]
    fn test_workload_add_requirement() {
        let mut profile = WorkloadProfile::new("App", "Java");

        profile.add_requirement(ResourceType::CPU, 4.0);
        profile.add_requirement(ResourceType::Memory, 16.0);
        profile.add_requirement(ResourceType::Storage, 100.0);

        assert_eq!(profile.resource_requirements.len(), 3);
        assert_eq!(profile.get_requirement(&ResourceType::CPU), Some(4.0));
    }

    #[test]
    fn test_workload_total_resources() {
        let mut profile = WorkloadProfile::new("App", "Node.js");

        profile.add_requirement(ResourceType::CPU, 2.0);
        profile.add_requirement(ResourceType::Memory, 8.0);

        assert_eq!(profile.total_cpu(), 2.0);
        assert_eq!(profile.total_memory(), 8.0);
    }

    #[test]
    fn test_capacity_manager() {
        let mut manager = CapacityManager::new();

        let capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");
        manager.add_capacity("cluster-1", capacity);

        assert_eq!(manager.capacity_count(), 1);
        assert!(manager
            .get_capacity("cluster-1", &ResourceType::CPU)
            .is_some());
    }

    #[test]
    fn test_manager_workloads() {
        let mut manager = CapacityManager::new();

        let workload = WorkloadProfile::new("Test", "App");
        let id = manager.add_workload(workload);

        assert_eq!(manager.workload_count(), 1);
        assert!(manager.get_workload(&id).is_some());
    }

    #[test]
    fn test_manager_critical_capacities() {
        let mut manager = CapacityManager::new();

        let mut capacity1 = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");
        capacity1.update_usage(50.0);

        let mut capacity2 = ResourceCapacity::new(ResourceType::Memory, 1000.0, "GB");
        capacity2.update_usage(900.0);

        manager.add_capacity("cluster-1", capacity1);
        manager.add_capacity("cluster-2", capacity2);

        let critical = manager.critical_capacities();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_manager_healthy_capacities() {
        let mut manager = CapacityManager::new();

        let mut capacity1 = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");
        capacity1.update_usage(60.0);

        let mut capacity2 = ResourceCapacity::new(ResourceType::Memory, 1000.0, "GB");
        capacity2.update_usage(900.0);

        manager.add_capacity("cluster-1", capacity1);
        manager.add_capacity("cluster-2", capacity2);

        let healthy = manager.healthy_capacities();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_manager_by_resource_type() {
        let mut manager = CapacityManager::new();

        manager.add_capacity(
            "cluster-1",
            ResourceCapacity::new(ResourceType::CPU, 100.0, "cores"),
        );
        manager.add_capacity(
            "cluster-2",
            ResourceCapacity::new(ResourceType::CPU, 200.0, "cores"),
        );
        manager.add_capacity(
            "cluster-3",
            ResourceCapacity::new(ResourceType::Memory, 1000.0, "GB"),
        );

        let cpu = manager.by_resource_type(&ResourceType::CPU);
        assert_eq!(cpu.len(), 2);
    }

    #[test]
    fn test_manager_high_priority_workloads() {
        let mut manager = CapacityManager::new();

        manager.add_workload(WorkloadProfile::new("W1", "App").with_priority(5));
        manager.add_workload(WorkloadProfile::new("W2", "DB").with_priority(9));
        manager.add_workload(WorkloadProfile::new("W3", "Cache").with_priority(8));

        let high_priority = manager.high_priority_workloads();
        assert_eq!(high_priority.len(), 2);
    }

    #[test]
    fn test_manager_remove_workload() {
        let mut manager = CapacityManager::new();

        let workload = WorkloadProfile::new("Test", "App");
        let id = manager.add_workload(workload);

        assert!(manager.remove_workload(&id));
        assert_eq!(manager.workload_count(), 0);
    }

    #[test]
    fn test_manager_get_capacity_mut() {
        let mut manager = CapacityManager::new();

        let capacity = ResourceCapacity::new(ResourceType::CPU, 100.0, "cores");
        manager.add_capacity("cluster-1", capacity);

        if let Some(capacity_mut) = manager.get_capacity_mut("cluster-1", &ResourceType::CPU) {
            capacity_mut.update_usage(75.0);
        }

        let capacity = manager
            .get_capacity("cluster-1", &ResourceType::CPU)
            .unwrap();
        assert_eq!(capacity.used_capacity, 75.0);
    }

    #[test]
    fn test_resource_type_equality() {
        assert_eq!(ResourceType::CPU, ResourceType::CPU);
        assert_ne!(ResourceType::CPU, ResourceType::Memory);
    }

    #[test]
    fn test_capacity_status_equality() {
        assert_eq!(CapacityStatus::Available, CapacityStatus::Available);
        assert_ne!(CapacityStatus::Available, CapacityStatus::Critical);
    }

    #[test]
    fn test_capacity_zero_total() {
        let capacity = ResourceCapacity::new(ResourceType::CPU, 0.0, "cores");
        assert_eq!(capacity.utilization_percent(), 0.0);
    }
}
