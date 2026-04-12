// Resource Quotas - Tenant resource limits and tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource quota definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub limits: ResourceLimits,
    pub usage: ResourceUsage,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ResourceQuota {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "quota-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            namespace: namespace.into(),
            limits: ResourceLimits::default(),
            usage: ResourceUsage::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self.updated_at = Utc::now();
        self
    }

    pub fn is_exceeded(&self) -> bool {
        self.usage.exceeds(&self.limits)
    }

    pub fn can_allocate(&self, requested: &ResourceRequest) -> bool {
        self.usage.can_fit(requested, &self.limits)
    }

    pub fn allocate(&mut self, resources: &ResourceRequest) -> Result<(), String> {
        if resources.cpu_cores == 0 && resources.memory_gi == 0 && resources.storage_gi == 0 {
            return Err("Must allocate at least one resource".to_string());
        }
        if !self.can_allocate(resources) {
            return Err("Quota exceeded".to_string());
        }

        self.usage.vms += 1;
        self.usage.cpu_cores += resources.cpu_cores;
        self.usage.memory_gi += resources.memory_gi;
        self.usage.storage_gi += resources.storage_gi;
        self.updated_at = Utc::now();

        Ok(())
    }

    pub fn deallocate(&mut self, resources: &ResourceRequest) {
        self.usage.vms = self.usage.vms.saturating_sub(1);
        self.usage.cpu_cores = self.usage.cpu_cores.saturating_sub(resources.cpu_cores);
        self.usage.memory_gi = self.usage.memory_gi.saturating_sub(resources.memory_gi);
        self.usage.storage_gi = self.usage.storage_gi.saturating_sub(resources.storage_gi);
        self.updated_at = Utc::now();
    }

    pub fn utilization_percentage(&self) -> HashMap<String, f64> {
        let mut util = HashMap::new();

        if self.limits.max_vms > 0 {
            util.insert(
                "vms".to_string(),
                (self.usage.vms as f64 / self.limits.max_vms as f64) * 100.0,
            );
        }

        if self.limits.max_cpu_cores > 0 {
            util.insert(
                "cpu".to_string(),
                (self.usage.cpu_cores as f64 / self.limits.max_cpu_cores as f64) * 100.0,
            );
        }

        if self.limits.max_memory_gi > 0 {
            util.insert(
                "memory".to_string(),
                (self.usage.memory_gi as f64 / self.limits.max_memory_gi as f64) * 100.0,
            );
        }

        if self.limits.max_storage_gi > 0 {
            util.insert(
                "storage".to_string(),
                (self.usage.storage_gi as f64 / self.limits.max_storage_gi as f64) * 100.0,
            );
        }

        util
    }
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_vms: u32,
    pub max_cpu_cores: u32,
    pub max_memory_gi: u32,
    pub max_storage_gi: u32,
    pub max_snapshots: u32,
    pub max_backups: u32,
}

impl ResourceLimits {
    pub fn new() -> Self {
        Self {
            max_vms: 10,
            max_cpu_cores: 40,
            max_memory_gi: 128,
            max_storage_gi: 1000,
            max_snapshots: 50,
            max_backups: 20,
        }
    }

    pub fn unlimited() -> Self {
        Self {
            max_vms: u32::MAX,
            max_cpu_cores: u32::MAX,
            max_memory_gi: u32::MAX,
            max_storage_gi: u32::MAX,
            max_snapshots: u32::MAX,
            max_backups: u32::MAX,
        }
    }

    pub fn small() -> Self {
        Self {
            max_vms: 5,
            max_cpu_cores: 20,
            max_memory_gi: 64,
            max_storage_gi: 500,
            max_snapshots: 25,
            max_backups: 10,
        }
    }

    pub fn medium() -> Self {
        Self::new() // Default is medium
    }

    pub fn large() -> Self {
        Self {
            max_vms: 50,
            max_cpu_cores: 200,
            max_memory_gi: 512,
            max_storage_gi: 5000,
            max_snapshots: 200,
            max_backups: 100,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Current resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub vms: u32,
    pub cpu_cores: u32,
    pub memory_gi: u32,
    pub storage_gi: u32,
    pub snapshots: u32,
    pub backups: u32,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            vms: 0,
            cpu_cores: 0,
            memory_gi: 0,
            storage_gi: 0,
            snapshots: 0,
            backups: 0,
        }
    }

    pub fn exceeds(&self, limits: &ResourceLimits) -> bool {
        self.vms > limits.max_vms
            || self.cpu_cores > limits.max_cpu_cores
            || self.memory_gi > limits.max_memory_gi
            || self.storage_gi > limits.max_storage_gi
            || self.snapshots > limits.max_snapshots
            || self.backups > limits.max_backups
    }

    pub fn can_fit(&self, request: &ResourceRequest, limits: &ResourceLimits) -> bool {
        self.vms < limits.max_vms
            && self.cpu_cores + request.cpu_cores <= limits.max_cpu_cores
            && self.memory_gi + request.memory_gi <= limits.max_memory_gi
            && self.storage_gi + request.storage_gi <= limits.max_storage_gi
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub cpu_cores: u32,
    pub memory_gi: u32,
    pub storage_gi: u32,
}

impl ResourceRequest {
    pub fn new(cpu_cores: u32, memory_gi: u32, storage_gi: u32) -> Self {
        Self {
            cpu_cores,
            memory_gi,
            storage_gi,
        }
    }
}

/// Quota manager
pub struct QuotaManager {
    quotas: HashMap<String, ResourceQuota>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: HashMap::new(),
        }
    }

    pub fn create_quota(&mut self, quota: ResourceQuota) {
        self.quotas.insert(quota.id.clone(), quota);
    }

    pub fn get_quota(&self, quota_id: &str) -> Option<&ResourceQuota> {
        self.quotas.get(quota_id)
    }

    pub fn get_quota_mut(&mut self, quota_id: &str) -> Option<&mut ResourceQuota> {
        self.quotas.get_mut(quota_id)
    }

    pub fn get_quota_by_namespace(&self, namespace: &str) -> Option<&ResourceQuota> {
        self.quotas.values().find(|q| q.namespace == namespace)
    }

    pub fn get_quota_by_namespace_mut(&mut self, namespace: &str) -> Option<&mut ResourceQuota> {
        self.quotas.values_mut().find(|q| q.namespace == namespace)
    }

    pub fn delete_quota(&mut self, quota_id: &str) -> bool {
        self.quotas.remove(quota_id).is_some()
    }

    pub fn list_quotas(&self) -> Vec<&ResourceQuota> {
        self.quotas.values().collect()
    }

    pub fn exceeded_quotas(&self) -> Vec<&ResourceQuota> {
        self.quotas.values().filter(|q| q.is_exceeded()).collect()
    }

    pub fn check_allocation(
        &self,
        namespace: &str,
        request: &ResourceRequest,
    ) -> Result<(), String> {
        if let Some(quota) = self.get_quota_by_namespace(namespace) {
            if !quota.can_allocate(request) {
                return Err(format!("Quota exceeded for namespace: {}", namespace));
            }
        }
        Ok(())
    }

    pub fn allocate(&mut self, namespace: &str, request: &ResourceRequest) -> Result<(), String> {
        if let Some(quota) = self.get_quota_by_namespace_mut(namespace) {
            quota.allocate(request)?;
        } else {
            log::warn!(
                "No quota defined for namespace '{}'; allocation proceeding without enforcement",
                namespace
            );
        }
        Ok(())
    }

    pub fn deallocate(&mut self, namespace: &str, request: &ResourceRequest) {
        if let Some(quota) = self.get_quota_by_namespace_mut(namespace) {
            quota.deallocate(request);
        }
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_presets() {
        let small = ResourceLimits::small();
        assert_eq!(small.max_vms, 5);

        let medium = ResourceLimits::medium();
        assert_eq!(medium.max_vms, 10);

        let large = ResourceLimits::large();
        assert_eq!(large.max_vms, 50);

        let unlimited = ResourceLimits::unlimited();
        assert_eq!(unlimited.max_vms, u32::MAX);
    }

    #[test]
    fn test_resource_quota_creation() {
        let quota = ResourceQuota::new("test-quota", "default");

        assert_eq!(quota.name, "test-quota");
        assert_eq!(quota.namespace, "default");
        assert_eq!(quota.usage.vms, 0);
    }

    #[test]
    fn test_resource_quota_with_limits() {
        let limits = ResourceLimits::small();
        let quota = ResourceQuota::new("test", "default").with_limits(limits.clone());

        assert_eq!(quota.limits.max_vms, limits.max_vms);
    }

    #[test]
    fn test_quota_allocation() {
        let mut quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::small());

        let request = ResourceRequest::new(2, 4, 20);

        assert!(quota.can_allocate(&request));
        assert!(quota.allocate(&request).is_ok());

        assert_eq!(quota.usage.vms, 1);
        assert_eq!(quota.usage.cpu_cores, 2);
        assert_eq!(quota.usage.memory_gi, 4);
        assert_eq!(quota.usage.storage_gi, 20);
    }

    #[test]
    fn test_quota_exceeded() {
        let mut quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::small());

        // Allocate close to limit
        let request = ResourceRequest::new(20, 64, 500);
        quota.allocate(&request).unwrap();

        // This should fail
        let request2 = ResourceRequest::new(5, 10, 100);
        assert!(quota.allocate(&request2).is_err());
    }

    #[test]
    fn test_quota_deallocation() {
        let mut quota = ResourceQuota::new("test", "default");

        let request = ResourceRequest::new(4, 8, 50);
        quota.allocate(&request).unwrap();

        assert_eq!(quota.usage.vms, 1);
        assert_eq!(quota.usage.cpu_cores, 4);

        quota.deallocate(&request);

        assert_eq!(quota.usage.vms, 0);
        assert_eq!(quota.usage.cpu_cores, 0);
    }

    #[test]
    fn test_quota_utilization() {
        let mut quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::new());

        let request = ResourceRequest::new(10, 32, 250);
        quota.allocate(&request).unwrap();

        let util = quota.utilization_percentage();

        assert!((util["vms"] - 10.0).abs() < 0.1); // 1/10 = 10%
        assert!((util["cpu"] - 25.0).abs() < 0.1); // 10/40 = 25%
    }

    #[test]
    fn test_resource_usage_exceeds() {
        let usage = ResourceUsage {
            vms: 15,
            cpu_cores: 50,
            memory_gi: 100,
            storage_gi: 1000,
            snapshots: 30,
            backups: 15,
        };

        let limits = ResourceLimits::small();
        assert!(usage.exceeds(&limits));
    }

    #[test]
    fn test_resource_usage_can_fit() {
        let usage = ResourceUsage {
            vms: 3,
            cpu_cores: 10,
            memory_gi: 30,
            storage_gi: 200,
            snapshots: 10,
            backups: 5,
        };

        let limits = ResourceLimits::medium();
        let request = ResourceRequest::new(5, 10, 50);

        assert!(usage.can_fit(&request, &limits));
    }

    #[test]
    fn test_quota_manager() {
        let mut manager = QuotaManager::new();

        let quota = ResourceQuota::new("test", "default");
        manager.create_quota(quota);

        assert_eq!(manager.list_quotas().len(), 1);
    }

    #[test]
    fn test_quota_manager_by_namespace() {
        let mut manager = QuotaManager::new();

        let quota = ResourceQuota::new("test", "production");
        manager.create_quota(quota);

        assert!(manager.get_quota_by_namespace("production").is_some());
        assert!(manager.get_quota_by_namespace("staging").is_none());
    }

    #[test]
    fn test_quota_manager_allocation() {
        let mut manager = QuotaManager::new();

        let quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::medium());
        manager.create_quota(quota);

        let request = ResourceRequest::new(4, 8, 50);

        assert!(manager.check_allocation("default", &request).is_ok());
        assert!(manager.allocate("default", &request).is_ok());

        let quota = manager.get_quota_by_namespace("default").unwrap();
        assert_eq!(quota.usage.cpu_cores, 4);
    }

    #[test]
    fn test_quota_manager_deallocation() {
        let mut manager = QuotaManager::new();

        let quota = ResourceQuota::new("test", "default");
        manager.create_quota(quota);

        let request = ResourceRequest::new(4, 8, 50);
        manager.allocate("default", &request).unwrap();
        manager.deallocate("default", &request);

        let quota = manager.get_quota_by_namespace("default").unwrap();
        assert_eq!(quota.usage.vms, 0);
    }

    #[test]
    fn test_quota_manager_exceeded() {
        let mut manager = QuotaManager::new();

        let mut quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::small());

        // Manually set usage to exceed limits
        quota.usage.vms = 10;

        manager.create_quota(quota);

        assert_eq!(manager.exceeded_quotas().len(), 1);
    }

    #[test]
    fn test_quota_manager_delete() {
        let mut manager = QuotaManager::new();

        let quota = ResourceQuota::new("test", "default");
        let quota_id = quota.id.clone();

        manager.create_quota(quota);
        assert_eq!(manager.list_quotas().len(), 1);

        assert!(manager.delete_quota(&quota_id));
        assert_eq!(manager.list_quotas().len(), 0);
    }

    #[test]
    fn test_resource_request() {
        let request = ResourceRequest::new(8, 16, 100);

        assert_eq!(request.cpu_cores, 8);
        assert_eq!(request.memory_gi, 16);
        assert_eq!(request.storage_gi, 100);
    }

    #[test]
    fn test_quota_is_exceeded() {
        let mut quota = ResourceQuota::new("test", "default").with_limits(ResourceLimits::small());

        assert!(!quota.is_exceeded());

        quota.usage.vms = 100; // Exceed limit
        assert!(quota.is_exceeded());
    }
}
