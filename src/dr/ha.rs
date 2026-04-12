use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HA configuration mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAMode {
    ActiveActive,
    ActivePassive,
    NPlus1,
    NWayRedundancy,
}

impl std::fmt::Display for HAMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HAMode::ActiveActive => write!(f, "Active-Active"),
            HAMode::ActivePassive => write!(f, "Active-Passive"),
            HAMode::NPlus1 => write!(f, "N+1"),
            HAMode::NWayRedundancy => write!(f, "N-Way Redundancy"),
        }
    }
}

/// Health check type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthCheckType {
    HTTP,
    TCP,
    ICMP,
    Custom,
}

/// HA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAConfig {
    pub id: String,
    pub name: String,
    pub mode: HAMode,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub health_check_type: HealthCheckType,
    pub health_check_interval_seconds: u64,
    pub health_check_timeout_seconds: u64,
    pub failure_threshold: u32,
    pub auto_healing: bool,
    pub created_at: DateTime<Utc>,
}

impl HAConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "ha-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            mode: HAMode::ActivePassive,
            min_replicas: 2,
            max_replicas: 3,
            health_check_type: HealthCheckType::HTTP,
            health_check_interval_seconds: 30,
            health_check_timeout_seconds: 5,
            failure_threshold: 3,
            auto_healing: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_mode(mut self, mode: HAMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_replicas(mut self, min: u32, max: u32) -> Self {
        // Ensure min <= max to prevent invalid configurations
        if min > max {
            log::warn!("HA config: min_replicas ({}) > max_replicas ({}), swapping values", min, max);
            self.min_replicas = max;
            self.max_replicas = min;
        } else {
            self.min_replicas = min;
            self.max_replicas = max;
        }
        self
    }

    pub fn with_health_check(
        mut self,
        check_type: HealthCheckType,
        interval: u64,
        timeout: u64,
    ) -> Self {
        self.health_check_type = check_type;
        self.health_check_interval_seconds = interval;
        self.health_check_timeout_seconds = timeout;
        self
    }

    pub fn with_auto_healing(mut self, enabled: bool) -> Self {
        self.auto_healing = enabled;
        self
    }
}

/// HA group member status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for MemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberStatus::Healthy => write!(f, "Healthy"),
            MemberStatus::Degraded => write!(f, "Degraded"),
            MemberStatus::Unhealthy => write!(f, "Unhealthy"),
            MemberStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// HA group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAMember {
    pub id: String,
    pub name: String,
    pub resource_id: String,
    pub is_primary: bool,
    pub status: MemberStatus,
    pub last_health_check: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub added_at: DateTime<Utc>,
}

impl HAMember {
    pub fn new(name: impl Into<String>, resource_id: impl Into<String>, is_primary: bool) -> Self {
        let name_str = name.into();
        let id = format!(
            "member-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            resource_id: resource_id.into(),
            is_primary,
            status: MemberStatus::Unknown,
            last_health_check: None,
            consecutive_failures: 0,
            added_at: Utc::now(),
        }
    }

    pub fn update_health(&mut self, healthy: bool) {
        self.last_health_check = Some(Utc::now());

        if healthy {
            self.consecutive_failures = 0;
            self.status = MemberStatus::Healthy;
        } else {
            self.consecutive_failures += 1;
            self.status = if self.consecutive_failures >= 3 {
                MemberStatus::Unhealthy
            } else {
                MemberStatus::Degraded
            };
        }
    }

    pub fn promote_to_primary(&mut self) {
        self.is_primary = true;
    }

    pub fn demote_from_primary(&mut self) {
        self.is_primary = false;
    }

    pub fn is_healthy(&self) -> bool {
        self.status == MemberStatus::Healthy
    }
}

/// HA group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAGroup {
    pub id: String,
    pub name: String,
    pub config: HAConfig,
    pub members: Vec<HAMember>,
    pub created_at: DateTime<Utc>,
}

impl HAGroup {
    pub fn new(name: impl Into<String>, config: HAConfig) -> Self {
        let name_str = name.into();
        let id = format!(
            "group-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            config,
            members: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_member(&mut self, member: HAMember) {
        if member.is_primary {
            // Demote any existing primaries to prevent multiple primaries
            for m in &mut self.members {
                m.is_primary = false;
            }
        }
        self.members.push(member);
    }

    pub fn remove_member(&mut self, member_id: &str) -> bool {
        let initial_len = self.members.len();
        self.members.retain(|m| m.id != member_id);
        self.members.len() < initial_len
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn healthy_members(&self) -> Vec<&HAMember> {
        self.members.iter().filter(|m| m.is_healthy()).collect()
    }

    pub fn promote_to_primary(&mut self, member_id: &str) -> bool {
        // Demote all existing primaries first
        for m in &mut self.members {
            m.is_primary = false;
        }
        // Promote the target member
        if let Some(member) = self.members.iter_mut().find(|m| m.id == member_id) {
            member.is_primary = true;
            true
        } else {
            false
        }
    }

    pub fn primary_member(&self) -> Option<&HAMember> {
        self.members.iter().find(|m| m.is_primary)
    }

    pub fn is_healthy(&self) -> bool {
        let healthy_count = self.healthy_members().len();
        healthy_count >= self.config.min_replicas as usize
    }

    pub fn needs_healing(&self) -> bool {
        !self.is_healthy() && self.config.auto_healing
    }
}

/// HA manager
pub struct HAManager {
    configs: HashMap<String, HAConfig>,
    groups: HashMap<String, HAGroup>,
}

impl HAManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, config: HAConfig) -> String {
        let id = config.id.clone();
        self.configs.insert(id.clone(), config);
        id
    }

    pub fn get_config(&self, id: &str) -> Option<&HAConfig> {
        self.configs.get(id)
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    pub fn add_group(&mut self, group: HAGroup) -> String {
        let id = group.id.clone();
        self.groups.insert(id.clone(), group);
        id
    }

    pub fn get_group(&self, id: &str) -> Option<&HAGroup> {
        self.groups.get(id)
    }

    pub fn get_group_mut(&mut self, id: &str) -> Option<&mut HAGroup> {
        self.groups.get_mut(id)
    }

    pub fn remove_group(&mut self, id: &str) -> bool {
        self.groups.remove(id).is_some()
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn healthy_groups(&self) -> Vec<&HAGroup> {
        self.groups.values().filter(|g| g.is_healthy()).collect()
    }

    pub fn groups_needing_healing(&self) -> Vec<&HAGroup> {
        self.groups.values().filter(|g| g.needs_healing()).collect()
    }

    pub fn by_mode(&self, mode: &HAMode) -> Vec<&HAGroup> {
        self.groups
            .values()
            .filter(|g| &g.config.mode == mode)
            .collect()
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
    fn test_ha_mode_display() {
        assert_eq!(HAMode::ActiveActive.to_string(), "Active-Active");
        assert_eq!(HAMode::ActivePassive.to_string(), "Active-Passive");
        assert_eq!(HAMode::NPlus1.to_string(), "N+1");
    }

    #[test]
    fn test_ha_config() {
        let config = HAConfig::new("Database HA");

        assert_eq!(config.name, "Database HA");
        assert_eq!(config.mode, HAMode::ActivePassive);
        assert_eq!(config.min_replicas, 2);
        assert_eq!(config.max_replicas, 3);
        assert!(config.auto_healing);
    }

    #[test]
    fn test_ha_config_builder() {
        let config = HAConfig::new("HA")
            .with_mode(HAMode::ActiveActive)
            .with_replicas(3, 5)
            .with_health_check(HealthCheckType::TCP, 15, 3)
            .with_auto_healing(false);

        assert_eq!(config.mode, HAMode::ActiveActive);
        assert_eq!(config.min_replicas, 3);
        assert_eq!(config.max_replicas, 5);
        assert_eq!(config.health_check_type, HealthCheckType::TCP);
        assert_eq!(config.health_check_interval_seconds, 15);
        assert!(!config.auto_healing);
    }

    #[test]
    fn test_member_status_display() {
        assert_eq!(MemberStatus::Healthy.to_string(), "Healthy");
        assert_eq!(MemberStatus::Degraded.to_string(), "Degraded");
        assert_eq!(MemberStatus::Unhealthy.to_string(), "Unhealthy");
    }

    #[test]
    fn test_ha_member() {
        let member = HAMember::new("VM-1", "vm-123", true);

        assert_eq!(member.name, "VM-1");
        assert_eq!(member.resource_id, "vm-123");
        assert!(member.is_primary);
        assert_eq!(member.status, MemberStatus::Unknown);
        assert_eq!(member.consecutive_failures, 0);
    }

    #[test]
    fn test_member_health_updates() {
        let mut member = HAMember::new("VM-1", "vm-123", false);

        member.update_health(true);
        assert_eq!(member.status, MemberStatus::Healthy);
        assert!(member.is_healthy());
        assert_eq!(member.consecutive_failures, 0);

        member.update_health(false);
        assert_eq!(member.status, MemberStatus::Degraded);
        assert_eq!(member.consecutive_failures, 1);

        member.update_health(false);
        member.update_health(false);
        assert_eq!(member.status, MemberStatus::Unhealthy);
        assert!(!member.is_healthy());
        assert_eq!(member.consecutive_failures, 3);
    }

    #[test]
    fn test_member_promotion() {
        let mut member = HAMember::new("VM-1", "vm-123", false);

        assert!(!member.is_primary);

        member.promote_to_primary();
        assert!(member.is_primary);

        member.demote_from_primary();
        assert!(!member.is_primary);
    }

    #[test]
    fn test_ha_group() {
        let config = HAConfig::new("Test HA");
        let group = HAGroup::new("My Group", config);

        assert_eq!(group.name, "My Group");
        assert_eq!(group.member_count(), 0);
    }

    #[test]
    fn test_group_add_remove_members() {
        let config = HAConfig::new("Test HA");
        let mut group = HAGroup::new("Group", config);

        let member1 = HAMember::new("M1", "vm-1", true);
        let member2 = HAMember::new("M2", "vm-2", false);
        let member_id = member2.id.clone();

        group.add_member(member1);
        group.add_member(member2);

        assert_eq!(group.member_count(), 2);

        assert!(group.remove_member(&member_id));
        assert_eq!(group.member_count(), 1);
    }

    #[test]
    fn test_group_healthy_members() {
        let config = HAConfig::new("Test HA");
        let mut group = HAGroup::new("Group", config);

        let mut member1 = HAMember::new("M1", "vm-1", true);
        member1.update_health(true);

        let mut member2 = HAMember::new("M2", "vm-2", false);
        member2.update_health(false);
        member2.update_health(false);
        member2.update_health(false);

        group.add_member(member1);
        group.add_member(member2);

        let healthy = group.healthy_members();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_group_primary_member() {
        let config = HAConfig::new("Test HA");
        let mut group = HAGroup::new("Group", config);

        let member1 = HAMember::new("M1", "vm-1", false);
        let member2 = HAMember::new("M2", "vm-2", true);

        group.add_member(member1);
        group.add_member(member2);

        let primary = group.primary_member().unwrap();
        assert_eq!(primary.name, "M2");
    }

    #[test]
    fn test_group_health() {
        let config = HAConfig::new("Test HA").with_replicas(2, 4);
        let mut group = HAGroup::new("Group", config);

        let mut member1 = HAMember::new("M1", "vm-1", true);
        member1.update_health(true);

        let mut member2 = HAMember::new("M2", "vm-2", false);
        member2.update_health(true);

        group.add_member(member1);
        group.add_member(member2);

        assert!(group.is_healthy());
    }

    #[test]
    fn test_group_needs_healing() {
        let config = HAConfig::new("Test HA")
            .with_replicas(2, 4)
            .with_auto_healing(true);
        let mut group = HAGroup::new("Group", config);

        let mut member1 = HAMember::new("M1", "vm-1", true);
        member1.update_health(false);
        member1.update_health(false);
        member1.update_health(false);

        group.add_member(member1);

        assert!(group.needs_healing());
    }

    #[test]
    fn test_ha_manager() {
        let mut manager = HAManager::new();

        let config = HAConfig::new("Test HA");
        let id = manager.add_config(config);

        assert_eq!(manager.config_count(), 1);
        assert!(manager.get_config(&id).is_some());
    }

    #[test]
    fn test_manager_groups() {
        let mut manager = HAManager::new();

        let config = HAConfig::new("Test HA");
        let group = HAGroup::new("Group", config);
        let id = manager.add_group(group);

        assert_eq!(manager.group_count(), 1);
        assert!(manager.get_group(&id).is_some());
    }

    #[test]
    fn test_manager_healthy_groups() {
        let mut manager = HAManager::new();

        let config1 = HAConfig::new("HA1").with_replicas(2, 4);
        let mut group1 = HAGroup::new("G1", config1);
        let mut m1 = HAMember::new("M1", "vm-1", true);
        m1.update_health(true);
        let mut m2 = HAMember::new("M2", "vm-2", false);
        m2.update_health(true);
        group1.add_member(m1);
        group1.add_member(m2);

        let config2 = HAConfig::new("HA2").with_replicas(2, 4);
        let mut group2 = HAGroup::new("G2", config2);
        let mut m3 = HAMember::new("M3", "vm-3", true);
        m3.update_health(false);
        m3.update_health(false);
        m3.update_health(false);
        group2.add_member(m3);

        manager.add_group(group1);
        manager.add_group(group2);

        let healthy = manager.healthy_groups();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_manager_groups_needing_healing() {
        let mut manager = HAManager::new();

        let config1 = HAConfig::new("HA1")
            .with_replicas(2, 4)
            .with_auto_healing(true);
        let mut group1 = HAGroup::new("G1", config1);
        let mut m1 = HAMember::new("M1", "vm-1", true);
        m1.update_health(false);
        m1.update_health(false);
        m1.update_health(false);
        group1.add_member(m1);

        let config2 = HAConfig::new("HA2").with_replicas(2, 4);
        let mut group2 = HAGroup::new("G2", config2);
        let mut m2 = HAMember::new("M2", "vm-2", true);
        m2.update_health(true);
        let mut m3 = HAMember::new("M3", "vm-3", false);
        m3.update_health(true);
        group2.add_member(m2);
        group2.add_member(m3);

        manager.add_group(group1);
        manager.add_group(group2);

        let healing = manager.groups_needing_healing();
        assert_eq!(healing.len(), 1);
    }

    #[test]
    fn test_manager_by_mode() {
        let mut manager = HAManager::new();

        let config1 = HAConfig::new("HA1").with_mode(HAMode::ActiveActive);
        let config2 = HAConfig::new("HA2").with_mode(HAMode::ActivePassive);
        let config3 = HAConfig::new("HA3").with_mode(HAMode::ActiveActive);

        manager.add_group(HAGroup::new("G1", config1));
        manager.add_group(HAGroup::new("G2", config2));
        manager.add_group(HAGroup::new("G3", config3));

        let active_active = manager.by_mode(&HAMode::ActiveActive);
        assert_eq!(active_active.len(), 2);
    }

    #[test]
    fn test_manager_remove_group() {
        let mut manager = HAManager::new();

        let config = HAConfig::new("HA");
        let group = HAGroup::new("Group", config);
        let id = manager.add_group(group);

        assert!(manager.remove_group(&id));
        assert_eq!(manager.group_count(), 0);
    }

    #[test]
    fn test_manager_get_group_mut() {
        let mut manager = HAManager::new();

        let config = HAConfig::new("HA");
        let group = HAGroup::new("Group", config);
        let id = manager.add_group(group);

        if let Some(group_mut) = manager.get_group_mut(&id) {
            group_mut.add_member(HAMember::new("M1", "vm-1", true));
        }

        let group = manager.get_group(&id).unwrap();
        assert_eq!(group.member_count(), 1);
    }

    #[test]
    fn test_ha_mode_equality() {
        assert_eq!(HAMode::ActiveActive, HAMode::ActiveActive);
        assert_ne!(HAMode::ActiveActive, HAMode::ActivePassive);
    }

    #[test]
    fn test_member_status_equality() {
        assert_eq!(MemberStatus::Healthy, MemberStatus::Healthy);
        assert_ne!(MemberStatus::Healthy, MemberStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_type_equality() {
        assert_eq!(HealthCheckType::HTTP, HealthCheckType::HTTP);
        assert_ne!(HealthCheckType::HTTP, HealthCheckType::TCP);
    }
}
