// Tenant Management - Multi-tenant organization and isolation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::quotas::ResourceQuota;

/// Tenant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub namespaces: HashSet<String>,
    pub owner_id: String,
    pub contact_email: String,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

impl Tenant {
    pub fn new(
        name: impl Into<String>,
        owner_id: impl Into<String>,
        contact_email: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!("tenant-{}", name_str.to_lowercase().replace(' ', "-"));

        Self {
            id,
            name: name_str,
            description: String::new(),
            namespaces: HashSet::new(),
            owner_id: owner_id.into(),
            contact_email: contact_email.into(),
            labels: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            active: true,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self.updated_at = Utc::now();
        self
    }

    pub fn add_namespace(&mut self, namespace: impl Into<String>) {
        self.namespaces.insert(namespace.into());
        self.updated_at = Utc::now();
    }

    pub fn remove_namespace(&mut self, namespace: &str) {
        self.namespaces.remove(namespace);
        self.updated_at = Utc::now();
    }

    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    pub fn has_namespace(&self, namespace: &str) -> bool {
        self.namespaces.contains(namespace)
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = Utc::now();
    }

    pub fn namespace_count(&self) -> usize {
        self.namespaces.len()
    }
}

/// Tenant isolation policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsolationLevel {
    Strict,   // Complete isolation - no cross-tenant access
    Moderate, // Isolated by default, cross-tenant allowed with permissions
    Relaxed,  // Minimal isolation, shared resources allowed
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub isolation_level: IsolationLevel,
    pub default_namespace: String,
    pub allow_public_networks: bool,
    pub allow_cross_tenant_migration: bool,
    pub enforce_network_policies: bool,
    pub require_resource_labels: bool,
}

impl TenantConfig {
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            isolation_level: IsolationLevel::Strict,
            default_namespace: "default".to_string(),
            allow_public_networks: false,
            allow_cross_tenant_migration: false,
            enforce_network_policies: true,
            require_resource_labels: true,
        }
    }

    pub fn with_isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    pub fn with_default_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.default_namespace = namespace.into();
        self
    }
}

/// Tenant manager
pub struct TenantManager {
    tenants: HashMap<String, Tenant>,
    configs: HashMap<String, TenantConfig>,
    quotas: HashMap<String, ResourceQuota>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: HashMap::new(),
            configs: HashMap::new(),
            quotas: HashMap::new(),
        }
    }

    pub fn create_tenant(&mut self, tenant: Tenant) -> String {
        let tenant_id = tenant.id.clone();
        let config = TenantConfig::new(&tenant_id);

        self.tenants.insert(tenant_id.clone(), tenant);
        self.configs.insert(tenant_id.clone(), config);

        tenant_id
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Option<&Tenant> {
        self.tenants.get(tenant_id)
    }

    pub fn get_tenant_mut(&mut self, tenant_id: &str) -> Option<&mut Tenant> {
        self.tenants.get_mut(tenant_id)
    }

    pub fn delete_tenant(&mut self, tenant_id: &str) -> bool {
        // Clean up associated configs and quotas to prevent orphaned data
        self.configs.remove(tenant_id);
        self.quotas.remove(tenant_id);
        self.tenants.remove(tenant_id).is_some()
    }

    pub fn list_tenants(&self) -> Vec<&Tenant> {
        self.tenants.values().collect()
    }

    pub fn active_tenants(&self) -> Vec<&Tenant> {
        self.tenants.values().filter(|t| t.active).collect()
    }

    pub fn get_config(&self, tenant_id: &str) -> Option<&TenantConfig> {
        self.configs.get(tenant_id)
    }

    pub fn update_config(&mut self, tenant_id: &str, config: TenantConfig) {
        self.configs.insert(tenant_id.to_string(), config);
    }

    pub fn set_quota(&mut self, tenant_id: &str, quota: ResourceQuota) {
        self.quotas.insert(tenant_id.to_string(), quota);
    }

    pub fn get_quota(&self, tenant_id: &str) -> Option<&ResourceQuota> {
        self.quotas.get(tenant_id)
    }

    pub fn get_tenant_by_namespace(&self, namespace: &str) -> Option<&Tenant> {
        self.tenants.values().find(|t| t.has_namespace(namespace))
    }

    pub fn is_cross_tenant_allowed(&self, from_tenant: &str, to_tenant: &str) -> bool {
        if from_tenant == to_tenant {
            return true;
        }

        if let Some(config) = self.get_config(from_tenant) {
            match config.isolation_level {
                IsolationLevel::Strict => false,
                IsolationLevel::Moderate => config.allow_cross_tenant_migration,
                IsolationLevel::Relaxed => true,
            }
        } else {
            false
        }
    }

    pub fn tenant_count(&self) -> usize {
        self.tenants.len()
    }

    pub fn active_tenant_count(&self) -> usize {
        self.active_tenants().len()
    }

    pub fn namespace_count(&self) -> usize {
        self.tenants.values().map(|t| t.namespace_count()).sum()
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant isolation validator
pub struct IsolationValidator;

impl IsolationValidator {
    /// Check if operation is allowed based on isolation policy
    pub fn validate_operation(
        tenant_config: &TenantConfig,
        operation: &TenantOperation,
    ) -> Result<(), String> {
        match operation {
            TenantOperation::CrossTenantAccess { from, to } => {
                if from == to {
                    return Ok(());
                }

                match tenant_config.isolation_level {
                    IsolationLevel::Strict => {
                        Err("Cross-tenant access not allowed in strict isolation mode".to_string())
                    }
                    IsolationLevel::Moderate => {
                        if tenant_config.allow_cross_tenant_migration {
                            Ok(())
                        } else {
                            Err("Cross-tenant access requires explicit permission".to_string())
                        }
                    }
                    IsolationLevel::Relaxed => Ok(()),
                }
            }

            TenantOperation::PublicNetworkAccess => {
                if tenant_config.allow_public_networks {
                    Ok(())
                } else {
                    Err("Public network access not allowed for this tenant".to_string())
                }
            }

            TenantOperation::ResourceWithoutLabel => {
                if tenant_config.require_resource_labels {
                    Err("Resources must have tenant labels".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Tenant operation types for validation
#[derive(Debug, Clone)]
pub enum TenantOperation {
    CrossTenantAccess { from: String, to: String },
    PublicNetworkAccess,
    ResourceWithoutLabel,
}

#[cfg(test)]
mod tests {
    use super::super::quotas::ResourceLimits;
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Acme Corp", "user-123", "admin@acme.com")
            .with_description("Main corporate tenant");

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.owner_id, "user-123");
        assert_eq!(tenant.contact_email, "admin@acme.com");
        assert!(tenant.active);
    }

    #[test]
    fn test_tenant_namespaces() {
        let mut tenant = Tenant::new("Test", "user-1", "test@example.com");

        tenant.add_namespace("production");
        tenant.add_namespace("staging");

        assert_eq!(tenant.namespace_count(), 2);
        assert!(tenant.has_namespace("production"));
        assert!(tenant.has_namespace("staging"));

        tenant.remove_namespace("staging");
        assert_eq!(tenant.namespace_count(), 1);
        assert!(!tenant.has_namespace("staging"));
    }

    #[test]
    fn test_tenant_labels() {
        let mut tenant = Tenant::new("Test", "user-1", "test@example.com");

        tenant.add_label("env", "production");
        tenant.add_label("region", "us-west");

        assert_eq!(tenant.labels.get("env"), Some(&"production".to_string()));
        assert_eq!(tenant.labels.get("region"), Some(&"us-west".to_string()));
    }

    #[test]
    fn test_tenant_activation() {
        let mut tenant = Tenant::new("Test", "user-1", "test@example.com");

        assert!(tenant.active);

        tenant.deactivate();
        assert!(!tenant.active);

        tenant.activate();
        assert!(tenant.active);
    }

    #[test]
    fn test_isolation_levels() {
        assert!(matches!(IsolationLevel::Strict, IsolationLevel::Strict));
        assert!(matches!(IsolationLevel::Moderate, IsolationLevel::Moderate));
        assert!(matches!(IsolationLevel::Relaxed, IsolationLevel::Relaxed));
    }

    #[test]
    fn test_tenant_config() {
        let config = TenantConfig::new("tenant-1")
            .with_isolation(IsolationLevel::Moderate)
            .with_default_namespace("custom");

        assert_eq!(config.isolation_level, IsolationLevel::Moderate);
        assert_eq!(config.default_namespace, "custom");
    }

    #[test]
    fn test_tenant_manager() {
        let mut manager = TenantManager::new();

        let tenant = Tenant::new("Test Tenant", "user-1", "test@example.com");
        let tenant_id = manager.create_tenant(tenant);

        assert_eq!(manager.tenant_count(), 1);
        assert!(manager.get_tenant(&tenant_id).is_some());
    }

    #[test]
    fn test_tenant_manager_active() {
        let mut manager = TenantManager::new();

        let tenant1 = Tenant::new("Active", "user-1", "test1@example.com");
        let mut tenant2 = Tenant::new("Inactive", "user-2", "test2@example.com");
        tenant2.deactivate();

        manager.create_tenant(tenant1);
        manager.create_tenant(tenant2);

        assert_eq!(manager.tenant_count(), 2);
        assert_eq!(manager.active_tenant_count(), 1);
    }

    #[test]
    fn test_tenant_manager_by_namespace() {
        let mut manager = TenantManager::new();

        let mut tenant = Tenant::new("Test", "user-1", "test@example.com");
        tenant.add_namespace("production");

        manager.create_tenant(tenant);

        let found = manager.get_tenant_by_namespace("production");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test");
    }

    #[test]
    fn test_tenant_manager_quota() {
        let mut manager = TenantManager::new();

        let tenant = Tenant::new("Test", "user-1", "test@example.com");
        let tenant_id = manager.create_tenant(tenant);

        let quota =
            ResourceQuota::new("test-quota", "default").with_limits(ResourceLimits::small());

        manager.set_quota(&tenant_id, quota);

        assert!(manager.get_quota(&tenant_id).is_some());
    }

    #[test]
    fn test_cross_tenant_allowed() {
        let mut manager = TenantManager::new();

        let tenant = Tenant::new("Test", "user-1", "test@example.com");
        let tenant_id = manager.create_tenant(tenant);

        // Strict isolation - not allowed
        assert!(!manager.is_cross_tenant_allowed(&tenant_id, "other-tenant"));

        // Same tenant - allowed
        assert!(manager.is_cross_tenant_allowed(&tenant_id, &tenant_id));

        // Moderate with flag - allowed
        let config = TenantConfig::new(&tenant_id).with_isolation(IsolationLevel::Moderate);
        manager.update_config(&tenant_id, config);

        assert!(!manager.is_cross_tenant_allowed(&tenant_id, "other-tenant"));
    }

    #[test]
    fn test_isolation_validator_strict() {
        let config = TenantConfig::new("tenant-1").with_isolation(IsolationLevel::Strict);

        let operation = TenantOperation::CrossTenantAccess {
            from: "tenant-1".to_string(),
            to: "tenant-2".to_string(),
        };

        let result = IsolationValidator::validate_operation(&config, &operation);
        assert!(result.is_err());
    }

    #[test]
    fn test_isolation_validator_same_tenant() {
        let config = TenantConfig::new("tenant-1").with_isolation(IsolationLevel::Strict);

        let operation = TenantOperation::CrossTenantAccess {
            from: "tenant-1".to_string(),
            to: "tenant-1".to_string(),
        };

        let result = IsolationValidator::validate_operation(&config, &operation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_isolation_validator_public_network() {
        let config = TenantConfig::new("tenant-1");

        let operation = TenantOperation::PublicNetworkAccess;

        let result = IsolationValidator::validate_operation(&config, &operation);
        assert!(result.is_err()); // Not allowed by default

        let mut config2 = TenantConfig::new("tenant-1");
        config2.allow_public_networks = true;

        let result2 = IsolationValidator::validate_operation(&config2, &operation);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_isolation_validator_resource_labels() {
        let config = TenantConfig::new("tenant-1");

        let operation = TenantOperation::ResourceWithoutLabel;

        let result = IsolationValidator::validate_operation(&config, &operation);
        assert!(result.is_err()); // Required by default

        let mut config2 = TenantConfig::new("tenant-1");
        config2.require_resource_labels = false;

        let result2 = IsolationValidator::validate_operation(&config2, &operation);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_tenant_manager_delete() {
        let mut manager = TenantManager::new();

        let tenant = Tenant::new("Test", "user-1", "test@example.com");
        let tenant_id = manager.create_tenant(tenant);

        assert_eq!(manager.tenant_count(), 1);

        assert!(manager.delete_tenant(&tenant_id));
        assert_eq!(manager.tenant_count(), 0);
    }

    #[test]
    fn test_tenant_manager_namespace_count() {
        let mut manager = TenantManager::new();

        let mut tenant1 = Tenant::new("Tenant1", "user-1", "test1@example.com");
        tenant1.add_namespace("ns1");
        tenant1.add_namespace("ns2");

        let mut tenant2 = Tenant::new("Tenant2", "user-2", "test2@example.com");
        tenant2.add_namespace("ns3");

        manager.create_tenant(tenant1);
        manager.create_tenant(tenant2);

        assert_eq!(manager.namespace_count(), 3);
    }

    #[test]
    fn test_isolation_validator_relaxed() {
        let config = TenantConfig::new("tenant-1").with_isolation(IsolationLevel::Relaxed);

        let operation = TenantOperation::CrossTenantAccess {
            from: "tenant-1".to_string(),
            to: "tenant-2".to_string(),
        };

        let result = IsolationValidator::validate_operation(&config, &operation);
        assert!(result.is_ok()); // Relaxed allows cross-tenant
    }
}
