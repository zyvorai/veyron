use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod failover;
pub mod ha;
pub mod plans;
pub mod recovery;
pub mod replication;

/// Recovery Point Objective - maximum acceptable data loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPO {
    pub seconds: u64,
}

impl RPO {
    pub fn new(seconds: u64) -> Self {
        Self { seconds }
    }

    pub fn minutes(minutes: u64) -> Self {
        Self {
            seconds: minutes * 60,
        }
    }

    pub fn hours(hours: u64) -> Self {
        Self {
            seconds: hours * 3600,
        }
    }

    pub fn as_minutes(&self) -> u64 {
        self.seconds / 60
    }

    pub fn as_hours(&self) -> u64 {
        self.seconds / 3600
    }
}

/// Recovery Time Objective - maximum acceptable downtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTO {
    pub seconds: u64,
}

impl RTO {
    pub fn new(seconds: u64) -> Self {
        Self { seconds }
    }

    pub fn minutes(minutes: u64) -> Self {
        Self {
            seconds: minutes * 60,
        }
    }

    pub fn hours(hours: u64) -> Self {
        Self {
            seconds: hours * 3600,
        }
    }

    pub fn as_minutes(&self) -> u64 {
        self.seconds / 60
    }

    pub fn as_hours(&self) -> u64 {
        self.seconds / 3600
    }
}

/// Disaster recovery site
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DRSite {
    Primary,
    Secondary,
    Tertiary,
}

impl std::fmt::Display for DRSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DRSite::Primary => write!(f, "Primary"),
            DRSite::Secondary => write!(f, "Secondary"),
            DRSite::Tertiary => write!(f, "Tertiary"),
        }
    }
}

/// DR strategy type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DRStrategy {
    ActiveActive,
    ActivePassive,
    PilotLight,
    WarmStandby,
    ColdStandby,
    BackupRestore,
}

impl std::fmt::Display for DRStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DRStrategy::ActiveActive => write!(f, "Active-Active"),
            DRStrategy::ActivePassive => write!(f, "Active-Passive"),
            DRStrategy::PilotLight => write!(f, "Pilot Light"),
            DRStrategy::WarmStandby => write!(f, "Warm Standby"),
            DRStrategy::ColdStandby => write!(f, "Cold Standby"),
            DRStrategy::BackupRestore => write!(f, "Backup & Restore"),
        }
    }
}

/// DR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DRConfig {
    pub id: String,
    pub name: String,
    pub strategy: DRStrategy,
    pub primary_site: String,
    pub secondary_site: String,
    pub rpo: RPO,
    pub rto: RTO,
    pub auto_failover: bool,
    pub sync_interval_seconds: u64,
    pub created_at: DateTime<Utc>,
}

impl DRConfig {
    pub fn new(
        name: impl Into<String>,
        primary: impl Into<String>,
        secondary: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "dr-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            strategy: DRStrategy::ActivePassive,
            primary_site: primary.into(),
            secondary_site: secondary.into(),
            rpo: RPO::hours(1),
            rto: RTO::hours(4),
            auto_failover: false,
            sync_interval_seconds: 300,
            created_at: Utc::now(),
        }
    }

    pub fn with_strategy(mut self, strategy: DRStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_rpo(mut self, rpo: RPO) -> Self {
        self.rpo = rpo;
        self
    }

    pub fn with_rto(mut self, rto: RTO) -> Self {
        self.rto = rto;
        self
    }

    pub fn with_auto_failover(mut self, enabled: bool) -> Self {
        self.auto_failover = enabled;
        self
    }

    pub fn is_active_active(&self) -> bool {
        self.strategy == DRStrategy::ActiveActive
    }
}

/// Protected resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResource {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub namespace: String,
    pub dr_config_id: String,
    pub current_site: DRSite,
    pub protection_enabled: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ProtectedResource {
    pub fn new(
        name: impl Into<String>,
        resource_type: impl Into<String>,
        namespace: impl Into<String>,
        dr_config_id: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "res-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            resource_type: resource_type.into(),
            namespace: namespace.into(),
            dr_config_id: dr_config_id.into(),
            current_site: DRSite::Primary,
            protection_enabled: true,
            last_sync: None,
            created_at: Utc::now(),
        }
    }

    pub fn enable_protection(&mut self) {
        self.protection_enabled = true;
    }

    pub fn disable_protection(&mut self) {
        self.protection_enabled = false;
    }

    pub fn update_sync_time(&mut self) {
        self.last_sync = Some(Utc::now());
    }

    pub fn is_synced(&self, max_age_seconds: u64) -> bool {
        if let Some(last_sync) = self.last_sync {
            let age = (Utc::now() - last_sync).num_seconds();
            age >= 0 && (age as u64) <= max_age_seconds
        } else {
            false
        }
    }
}

/// DR manager
pub struct DRManager {
    configs: HashMap<String, DRConfig>,
    resources: HashMap<String, ProtectedResource>,
}

impl DRManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            resources: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, config: DRConfig) -> String {
        let id = config.id.clone();
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&DRConfig> {
        self.configs.get(id)
    }

    pub fn remove_config(&mut self, id: &str) -> bool {
        self.configs.remove(id).is_some()
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn add_resource(&mut self, resource: ProtectedResource) -> String {
        let id = resource.id.clone();
        self.resources.insert(id.clone(), resource);
        id
    }

    pub fn get_resource(&self, id: &str) -> Option<&ProtectedResource> {
        self.resources.get(id)
    }

    pub fn get_resource_mut(&mut self, id: &str) -> Option<&mut ProtectedResource> {
        self.resources.get_mut(id)
    }

    pub fn remove_resource(&mut self, id: &str) -> bool {
        self.resources.remove(id).is_some()
    }

    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    pub fn protected_resources(&self) -> Vec<&ProtectedResource> {
        self.resources
            .values()
            .filter(|r| r.protection_enabled)
            .collect()
    }

    pub fn by_dr_config(&self, config_id: &str) -> Vec<&ProtectedResource> {
        self.resources
            .values()
            .filter(|r| r.dr_config_id == config_id)
            .collect()
    }

    pub fn by_site(&self, site: &DRSite) -> Vec<&ProtectedResource> {
        self.resources
            .values()
            .filter(|r| &r.current_site == site)
            .collect()
    }

    pub fn auto_failover_configs(&self) -> Vec<&DRConfig> {
        self.configs.values().filter(|c| c.auto_failover).collect()
    }

    pub fn active_active_configs(&self) -> Vec<&DRConfig> {
        self.configs
            .values()
            .filter(|c| c.is_active_active())
            .collect()
    }
}

impl Default for DRManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpo() {
        let rpo = RPO::new(3600);
        assert_eq!(rpo.seconds, 3600);
        assert_eq!(rpo.as_minutes(), 60);
        assert_eq!(rpo.as_hours(), 1);
    }

    #[test]
    fn test_rpo_minutes() {
        let rpo = RPO::minutes(30);
        assert_eq!(rpo.seconds, 1800);
        assert_eq!(rpo.as_minutes(), 30);
    }

    #[test]
    fn test_rpo_hours() {
        let rpo = RPO::hours(2);
        assert_eq!(rpo.seconds, 7200);
        assert_eq!(rpo.as_hours(), 2);
    }

    #[test]
    fn test_rto() {
        let rto = RTO::new(7200);
        assert_eq!(rto.seconds, 7200);
        assert_eq!(rto.as_minutes(), 120);
        assert_eq!(rto.as_hours(), 2);
    }

    #[test]
    fn test_rto_minutes() {
        let rto = RTO::minutes(45);
        assert_eq!(rto.seconds, 2700);
        assert_eq!(rto.as_minutes(), 45);
    }

    #[test]
    fn test_rto_hours() {
        let rto = RTO::hours(4);
        assert_eq!(rto.seconds, 14400);
        assert_eq!(rto.as_hours(), 4);
    }

    #[test]
    fn test_dr_site_display() {
        assert_eq!(DRSite::Primary.to_string(), "Primary");
        assert_eq!(DRSite::Secondary.to_string(), "Secondary");
        assert_eq!(DRSite::Tertiary.to_string(), "Tertiary");
    }

    #[test]
    fn test_dr_strategy_display() {
        assert_eq!(DRStrategy::ActiveActive.to_string(), "Active-Active");
        assert_eq!(DRStrategy::ActivePassive.to_string(), "Active-Passive");
        assert_eq!(DRStrategy::WarmStandby.to_string(), "Warm Standby");
    }

    #[test]
    fn test_dr_config() {
        let config = DRConfig::new("my-dr", "site-1", "site-2");

        assert_eq!(config.name, "my-dr");
        assert_eq!(config.primary_site, "site-1");
        assert_eq!(config.secondary_site, "site-2");
        assert_eq!(config.strategy, DRStrategy::ActivePassive);
        assert!(!config.auto_failover);
    }

    #[test]
    fn test_dr_config_builder() {
        let config = DRConfig::new("dr", "primary", "secondary")
            .with_strategy(DRStrategy::ActiveActive)
            .with_rpo(RPO::minutes(15))
            .with_rto(RTO::hours(1))
            .with_auto_failover(true);

        assert_eq!(config.strategy, DRStrategy::ActiveActive);
        assert_eq!(config.rpo.as_minutes(), 15);
        assert_eq!(config.rto.as_hours(), 1);
        assert!(config.auto_failover);
        assert!(config.is_active_active());
    }

    #[test]
    fn test_protected_resource() {
        let resource = ProtectedResource::new("my-vm", "VirtualMachine", "default", "dr-1");

        assert_eq!(resource.name, "my-vm");
        assert_eq!(resource.resource_type, "VirtualMachine");
        assert_eq!(resource.namespace, "default");
        assert_eq!(resource.dr_config_id, "dr-1");
        assert_eq!(resource.current_site, DRSite::Primary);
        assert!(resource.protection_enabled);
    }

    #[test]
    fn test_resource_protection() {
        let mut resource = ProtectedResource::new("vm", "VirtualMachine", "default", "dr-1");

        assert!(resource.protection_enabled);

        resource.disable_protection();
        assert!(!resource.protection_enabled);

        resource.enable_protection();
        assert!(resource.protection_enabled);
    }

    #[test]
    fn test_resource_sync() {
        let mut resource = ProtectedResource::new("vm", "VirtualMachine", "default", "dr-1");

        assert!(!resource.is_synced(300));

        resource.update_sync_time();
        assert!(resource.is_synced(300));
    }

    #[test]
    fn test_dr_manager() {
        let mut manager = DRManager::new();

        let config = DRConfig::new("dr", "site-1", "site-2");
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_resources() {
        let mut manager = DRManager::new();

        let resource = ProtectedResource::new("vm", "VirtualMachine", "default", "dr-1");
        let id = manager.add_resource(resource);

        assert_eq!(manager.resource_count(), 1);
        assert!(manager.get_resource(&id).is_some());
    }

    #[test]
    fn test_manager_protected_resources() {
        let mut manager = DRManager::new();

        let resource1 = ProtectedResource::new("vm-1", "VirtualMachine", "default", "dr-1");
        let mut resource2 = ProtectedResource::new("vm-2", "VirtualMachine", "default", "dr-1");
        resource2.disable_protection();

        manager.add_resource(resource1);
        manager.add_resource(resource2);

        let protected = manager.protected_resources();
        assert_eq!(protected.len(), 1);
    }

    #[test]
    fn test_manager_by_dr_config() {
        let mut manager = DRManager::new();

        manager.add_resource(ProtectedResource::new("vm-1", "VM", "ns1", "dr-1"));
        manager.add_resource(ProtectedResource::new("vm-2", "VM", "ns2", "dr-1"));
        manager.add_resource(ProtectedResource::new("vm-3", "VM", "ns3", "dr-2"));

        let dr1_resources = manager.by_dr_config("dr-1");
        assert_eq!(dr1_resources.len(), 2);
    }

    #[test]
    fn test_manager_by_site() {
        let mut manager = DRManager::new();

        let mut resource1 = ProtectedResource::new("vm-1", "VM", "ns1", "dr-1");
        resource1.current_site = DRSite::Primary;

        let mut resource2 = ProtectedResource::new("vm-2", "VM", "ns2", "dr-1");
        resource2.current_site = DRSite::Secondary;

        let mut resource3 = ProtectedResource::new("vm-3", "VM", "ns3", "dr-1");
        resource3.current_site = DRSite::Primary;

        manager.add_resource(resource1);
        manager.add_resource(resource2);
        manager.add_resource(resource3);

        let primary = manager.by_site(&DRSite::Primary);
        assert_eq!(primary.len(), 2);
    }

    #[test]
    fn test_manager_auto_failover_configs() {
        let mut manager = DRManager::new();

        manager.add_config(DRConfig::new("dr-1", "s1", "s2").with_auto_failover(true));
        manager.add_config(DRConfig::new("dr-2", "s1", "s2"));
        manager.add_config(DRConfig::new("dr-3", "s1", "s2").with_auto_failover(true));

        let auto = manager.auto_failover_configs();
        assert_eq!(auto.len(), 2);
    }

    #[test]
    fn test_manager_active_active_configs() {
        let mut manager = DRManager::new();

        manager
            .add_config(DRConfig::new("dr-1", "s1", "s2").with_strategy(DRStrategy::ActiveActive));
        manager
            .add_config(DRConfig::new("dr-2", "s1", "s2").with_strategy(DRStrategy::ActivePassive));
        manager
            .add_config(DRConfig::new("dr-3", "s1", "s2").with_strategy(DRStrategy::ActiveActive));

        let active = manager.active_active_configs();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_manager_remove_config() {
        let mut manager = DRManager::new();

        let config = DRConfig::new("dr", "site-1", "site-2");
        let id = manager.add_config(config);

        assert!(manager.remove_config(&id));
        assert_eq!(manager.config_count(), 0);
    }

    #[test]
    fn test_manager_remove_resource() {
        let mut manager = DRManager::new();

        let resource = ProtectedResource::new("vm", "VirtualMachine", "default", "dr-1");
        let id = manager.add_resource(resource);

        assert!(manager.remove_resource(&id));
        assert_eq!(manager.resource_count(), 0);
    }

    #[test]
    fn test_manager_get_resource_mut() {
        let mut manager = DRManager::new();

        let resource = ProtectedResource::new("vm", "VirtualMachine", "default", "dr-1");
        let id = manager.add_resource(resource);

        if let Some(resource_mut) = manager.get_resource_mut(&id) {
            resource_mut.update_sync_time();
        }

        let resource = manager.get_resource(&id).unwrap();
        assert!(resource.last_sync.is_some());
    }

    #[test]
    fn test_dr_site_equality() {
        assert_eq!(DRSite::Primary, DRSite::Primary);
        assert_ne!(DRSite::Primary, DRSite::Secondary);
    }

    #[test]
    fn test_dr_strategy_equality() {
        assert_eq!(DRStrategy::ActiveActive, DRStrategy::ActiveActive);
        assert_ne!(DRStrategy::ActiveActive, DRStrategy::ActivePassive);
    }
}
