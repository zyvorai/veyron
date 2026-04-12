// High Availability - VM HA configuration and failover

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High Availability configuration for a VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAConfig {
    pub vm_name: String,
    pub enabled: bool,
    pub priority: HAPriority,
    pub failover_policy: FailoverPolicy,
    pub eviction_strategy: EvictionStrategy,
    pub max_unavailable: u32,
}

impl HAConfig {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            enabled: true,
            priority: HAPriority::Normal,
            failover_policy: FailoverPolicy::default(),
            eviction_strategy: EvictionStrategy::LiveMigrate,
            max_unavailable: 1,
        }
    }

    pub fn with_priority(mut self, priority: HAPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_eviction_strategy(mut self, strategy: EvictionStrategy) -> Self {
        self.eviction_strategy = strategy;
        self
    }

    pub fn with_failover_policy(mut self, policy: FailoverPolicy) -> Self {
        self.failover_policy = policy;
        self
    }
}

/// HA priority level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HAPriority {
    Critical, // Highest priority
    High,
    Normal,
    Low,
}

impl std::fmt::Display for HAPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HAPriority::Critical => write!(f, "Critical"),
            HAPriority::High => write!(f, "High"),
            HAPriority::Normal => write!(f, "Normal"),
            HAPriority::Low => write!(f, "Low"),
        }
    }
}

impl HAPriority {
    pub fn as_u8(&self) -> u8 {
        match self {
            HAPriority::Critical => 4,
            HAPriority::High => 3,
            HAPriority::Normal => 2,
            HAPriority::Low => 1,
        }
    }
}

impl PartialOrd for HAPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HAPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u8().cmp(&other.as_u8())
    }
}

/// Failover policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverPolicy {
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_seconds: u64,
    pub require_quorum: bool,
}

impl Default for FailoverPolicy {
    fn default() -> Self {
        Self {
            auto_restart: true,
            max_restart_attempts: 3,
            restart_delay_seconds: 30,
            require_quorum: true,
        }
    }
}

/// Eviction strategy when node is drained
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvictionStrategy {
    LiveMigrate, // Attempt live migration
    Shutdown,    // Graceful shutdown
    None,        // Do not evict (risk downtime)
}

impl std::fmt::Display for EvictionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvictionStrategy::LiveMigrate => write!(f, "Live Migrate"),
            EvictionStrategy::Shutdown => write!(f, "Shutdown"),
            EvictionStrategy::None => write!(f, "None"),
        }
    }
}

/// Failover event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEvent {
    pub vm_name: String,
    pub event_type: FailoverEventType,
    pub source_node: String,
    pub target_node: Option<String>,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub successful: bool,
}

impl FailoverEvent {
    pub fn new(
        vm_name: impl Into<String>,
        event_type: FailoverEventType,
        source_node: impl Into<String>,
    ) -> Self {
        Self {
            vm_name: vm_name.into(),
            event_type,
            source_node: source_node.into(),
            target_node: None,
            reason: String::new(),
            timestamp: Utc::now(),
            successful: false,
        }
    }

    pub fn with_target(mut self, node: impl Into<String>) -> Self {
        self.target_node = Some(node.into());
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    pub fn mark_successful(mut self) -> Self {
        self.successful = true;
        self
    }
}

/// Failover event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailoverEventType {
    NodeFailure,
    NodeMaintenance,
    VMCrash,
    HealthCheckFailure,
    ManualTrigger,
}

impl std::fmt::Display for FailoverEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailoverEventType::NodeFailure => write!(f, "Node Failure"),
            FailoverEventType::NodeMaintenance => write!(f, "Node Maintenance"),
            FailoverEventType::VMCrash => write!(f, "VM Crash"),
            FailoverEventType::HealthCheckFailure => write!(f, "Health Check Failure"),
            FailoverEventType::ManualTrigger => write!(f, "Manual Trigger"),
        }
    }
}

/// HA manager for managing failover
pub struct HAManager {
    configs: Vec<HAConfig>,
}

impl HAManager {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
        }
    }

    pub fn add_config(&mut self, config: HAConfig) {
        // Remove any existing config for the same VM to prevent duplicates
        self.configs.retain(|c| c.vm_name != config.vm_name);
        self.configs.push(config);
    }

    pub fn get_config(&self, vm_name: &str) -> Option<&HAConfig> {
        self.configs.iter().find(|c| c.vm_name == vm_name)
    }

    pub fn remove_config(&mut self, vm_name: &str) -> bool {
        if let Some(pos) = self.configs.iter().position(|c| c.vm_name == vm_name) {
            self.configs.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get VMs sorted by priority
    pub fn get_by_priority(&self) -> Vec<&HAConfig> {
        let mut configs: Vec<&HAConfig> = self.configs.iter().collect();
        configs.sort_by(|a, b| b.priority.cmp(&a.priority));
        configs
    }

    /// Check if VM should be migrated on node failure
    pub fn should_migrate_on_failure(&self, vm_name: &str) -> bool {
        self.get_config(vm_name)
            .map(|c| c.enabled && c.eviction_strategy == EvictionStrategy::LiveMigrate)
            .unwrap_or(false)
    }

    /// Check if VM should be restarted after failure
    pub fn should_restart(&self, vm_name: &str, attempt_count: u32) -> bool {
        self.get_config(vm_name)
            .map(|c| {
                c.enabled
                    && c.failover_policy.auto_restart
                    && attempt_count < c.failover_policy.max_restart_attempts
            })
            .unwrap_or(false)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }
}

impl Default for HAManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ha_config() {
        let config = HAConfig::new("critical-vm")
            .with_priority(HAPriority::Critical)
            .with_eviction_strategy(EvictionStrategy::LiveMigrate);

        assert_eq!(config.vm_name, "critical-vm");
        assert_eq!(config.priority, HAPriority::Critical);
        assert_eq!(config.eviction_strategy, EvictionStrategy::LiveMigrate);
    }

    #[test]
    fn test_ha_priority_ordering() {
        assert!(HAPriority::Critical > HAPriority::High);
        assert!(HAPriority::High > HAPriority::Normal);
        assert!(HAPriority::Normal > HAPriority::Low);

        assert_eq!(HAPriority::Critical.as_u8(), 4);
        assert_eq!(HAPriority::Low.as_u8(), 1);
    }

    #[test]
    fn test_failover_policy_default() {
        let policy = FailoverPolicy::default();
        assert!(policy.auto_restart);
        assert_eq!(policy.max_restart_attempts, 3);
        assert_eq!(policy.restart_delay_seconds, 30);
    }

    #[test]
    fn test_failover_event() {
        let event = FailoverEvent::new("db-vm", FailoverEventType::NodeFailure, "node1")
            .with_target("node2")
            .with_reason("Node unresponsive")
            .mark_successful();

        assert_eq!(event.vm_name, "db-vm");
        assert_eq!(event.event_type, FailoverEventType::NodeFailure);
        assert_eq!(event.target_node, Some("node2".to_string()));
        assert!(event.successful);
    }

    #[test]
    fn test_ha_manager() {
        let mut manager = HAManager::new();

        let config1 = HAConfig::new("vm1").with_priority(HAPriority::Critical);
        let config2 = HAConfig::new("vm2").with_priority(HAPriority::Normal);

        manager.add_config(config1);
        manager.add_config(config2);

        assert_eq!(manager.config_count(), 2);
        assert!(manager.get_config("vm1").is_some());

        assert!(manager.should_migrate_on_failure("vm1"));
        assert!(manager.should_restart("vm1", 0));
        assert!(!manager.should_restart("vm1", 5)); // Exceeds max attempts

        assert!(manager.remove_config("vm1"));
        assert_eq!(manager.config_count(), 1);
    }

    #[test]
    fn test_priority_sorting() {
        let mut manager = HAManager::new();

        manager.add_config(HAConfig::new("low-vm").with_priority(HAPriority::Low));
        manager.add_config(HAConfig::new("critical-vm").with_priority(HAPriority::Critical));
        manager.add_config(HAConfig::new("normal-vm").with_priority(HAPriority::Normal));

        let sorted = manager.get_by_priority();
        assert_eq!(sorted[0].vm_name, "critical-vm");
        assert_eq!(sorted[1].vm_name, "normal-vm");
        assert_eq!(sorted[2].vm_name, "low-vm");
    }

    #[test]
    fn test_eviction_strategy_display() {
        assert_eq!(EvictionStrategy::LiveMigrate.to_string(), "Live Migrate");
        assert_eq!(EvictionStrategy::Shutdown.to_string(), "Shutdown");
        assert_eq!(EvictionStrategy::None.to_string(), "None");
    }

    #[test]
    fn test_failover_event_type_display() {
        assert_eq!(FailoverEventType::NodeFailure.to_string(), "Node Failure");
        assert_eq!(FailoverEventType::VMCrash.to_string(), "VM Crash");
    }
}
