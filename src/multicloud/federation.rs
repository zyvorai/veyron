// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::CloudProvider;

/// Federation type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationType {
    ActiveActive,
    ActivePassive,
    Distributed,
    Hierarchical,
}

/// Federation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationStatus {
    Initializing,
    Active,
    Degraded,
    Partitioned,
    Failed,
}

/// Cloud federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFederation {
    pub id: String,
    pub name: String,
    pub federation_type: FederationType,
    pub member_providers: Vec<CloudProvider>,
    pub primary_provider: CloudProvider,
    pub load_balancing_enabled: bool,
    pub auto_failover_enabled: bool,
    pub data_sync_enabled: bool,
    pub status: FederationStatus,
    pub created_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
}

impl CloudFederation {
    pub fn new(
        name: impl Into<String>,
        federation_type: FederationType,
        primary_provider: CloudProvider,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "fed-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            federation_type,
            member_providers: vec![primary_provider.clone()],
            primary_provider,
            load_balancing_enabled: false,
            auto_failover_enabled: false,
            data_sync_enabled: false,
            status: FederationStatus::Initializing,
            created_at: Utc::now(),
            last_sync: None,
        }
    }

    pub fn add_member(&mut self, provider: CloudProvider) {
        if !self.member_providers.contains(&provider) {
            self.member_providers.push(provider);
        }
    }

    pub fn enable_load_balancing(mut self) -> Self {
        self.load_balancing_enabled = true;
        self
    }

    pub fn enable_auto_failover(mut self) -> Self {
        self.auto_failover_enabled = true;
        self
    }

    pub fn enable_data_sync(mut self) -> Self {
        self.data_sync_enabled = true;
        self
    }

    pub fn set_status(&mut self, status: FederationStatus) {
        self.status = status;
    }

    pub fn record_sync(&mut self) {
        self.last_sync = Some(Utc::now());
    }

    pub fn is_active(&self) -> bool {
        self.status == FederationStatus::Active
    }

    pub fn is_multi_provider(&self) -> bool {
        self.member_providers.len() > 1
    }

    pub fn member_count(&self) -> usize {
        self.member_providers.len()
    }
}

/// Orchestration policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationPolicy {
    pub id: String,
    pub name: String,
    pub federation_id: String,
    pub policy_type: PolicyType,
    pub scheduling_strategy: SchedulingStrategy,
    pub priority: u32,
    pub enabled: bool,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Policy type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyType {
    Placement,
    Scaling,
    Migration,
    Failover,
    CostOptimization,
}

/// Scheduling strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulingStrategy {
    RoundRobin,
    LeastLoaded,
    CostOptimized,
    LatencyOptimized,
    AffinityBased,
    Custom,
}

impl OrchestrationPolicy {
    pub fn new(
        name: impl Into<String>,
        federation_id: impl Into<String>,
        policy_type: PolicyType,
        scheduling_strategy: SchedulingStrategy,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "policy-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            federation_id: federation_id.into(),
            policy_type,
            scheduling_strategy,
            priority: 100,
            enabled: true,
            conditions: Vec::new(),
            actions: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn add_condition(&mut self, condition: impl Into<String>) {
        self.conditions.push(condition.into());
    }

    pub fn add_action(&mut self, action: impl Into<String>) {
        self.actions.push(action.into());
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }
}

/// Federation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEvent {
    pub id: String,
    pub federation_id: String,
    pub event_type: EventType,
    pub provider: CloudProvider,
    pub description: String,
    pub severity: EventSeverity,
    pub timestamp: DateTime<Utc>,
}

/// Event type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    MemberJoined,
    MemberLeft,
    Failover,
    DataSync,
    StatusChange,
    Error,
}

/// Event severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl FederationEvent {
    pub fn new(
        federation_id: impl Into<String>,
        event_type: EventType,
        provider: CloudProvider,
        description: impl Into<String>,
        severity: EventSeverity,
    ) -> Self {
        let federation_id_str = federation_id.into();
        let id = format!(
            "event-{}-{}",
            federation_id_str,
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            federation_id: federation_id_str,
            event_type,
            provider,
            description: description.into(),
            severity,
            timestamp: Utc::now(),
        }
    }

    pub fn is_critical(&self) -> bool {
        self.severity == EventSeverity::Critical
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self.severity,
            EventSeverity::Error | EventSeverity::Critical
        )
    }
}

/// Federation manager
pub struct FederationManager {
    federations: HashMap<String, CloudFederation>,
    policies: HashMap<String, OrchestrationPolicy>,
    events: Vec<FederationEvent>,
}

impl FederationManager {
    pub fn new() -> Self {
        Self {
            federations: HashMap::new(),
            policies: HashMap::new(),
            events: Vec::new(),
        }
    }

    pub fn add_federation(&mut self, federation: CloudFederation) -> String {
        let id = federation.id.clone();
        self.federations.insert(id.clone(), federation);
        id
    }

    pub fn get_federation(&self, id: &str) -> Option<&CloudFederation> {
        self.federations.get(id)
    }

    pub fn get_federation_mut(&mut self, id: &str) -> Option<&mut CloudFederation> {
        self.federations.get_mut(id)
    }

    pub fn federation_count(&self) -> usize {
        self.federations.len()
    }

    pub fn add_policy(&mut self, policy: OrchestrationPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&OrchestrationPolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut OrchestrationPolicy> {
        self.policies.get_mut(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn add_event(&mut self, event: FederationEvent) {
        const MAX_FEDERATION_EVENTS: usize = 10_000;
        if self.events.len() >= MAX_FEDERATION_EVENTS {
            self.events.drain(..MAX_FEDERATION_EVENTS / 10);
        }
        self.events.push(event);
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn active_federations(&self) -> Vec<&CloudFederation> {
        self.federations
            .values()
            .filter(|f| f.is_active())
            .collect()
    }

    pub fn multi_provider_federations(&self) -> Vec<&CloudFederation> {
        self.federations
            .values()
            .filter(|f| f.is_multi_provider())
            .collect()
    }

    pub fn federations_with_failover(&self) -> Vec<&CloudFederation> {
        self.federations
            .values()
            .filter(|f| f.auto_failover_enabled)
            .collect()
    }

    pub fn policies_for_federation(&self, federation_id: &str) -> Vec<&OrchestrationPolicy> {
        self.policies
            .values()
            .filter(|p| p.federation_id == federation_id)
            .collect()
    }

    pub fn enabled_policies(&self) -> Vec<&OrchestrationPolicy> {
        self.policies.values().filter(|p| p.enabled).collect()
    }

    pub fn events_for_federation(&self, federation_id: &str) -> Vec<&FederationEvent> {
        self.events
            .iter()
            .filter(|e| e.federation_id == federation_id)
            .collect()
    }

    pub fn critical_events(&self) -> Vec<&FederationEvent> {
        self.events.iter().filter(|e| e.is_critical()).collect()
    }

    pub fn error_events(&self) -> Vec<&FederationEvent> {
        self.events.iter().filter(|e| e.is_error()).collect()
    }
}

impl Default for FederationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_federation() {
        let federation = CloudFederation::new(
            "global-federation",
            FederationType::ActiveActive,
            CloudProvider::AWS,
        );

        assert_eq!(federation.name, "global-federation");
        assert_eq!(federation.federation_type, FederationType::ActiveActive);
        assert_eq!(federation.primary_provider, CloudProvider::AWS);
        assert_eq!(federation.status, FederationStatus::Initializing);
        assert!(!federation.load_balancing_enabled);
    }

    #[test]
    fn test_federation_add_member() {
        let mut federation =
            CloudFederation::new("fed", FederationType::Distributed, CloudProvider::AWS);

        federation.add_member(CloudProvider::Azure);
        federation.add_member(CloudProvider::GCP);
        federation.add_member(CloudProvider::Azure); // Duplicate

        assert_eq!(federation.member_count(), 3); // AWS (initial) + Azure + GCP
    }

    #[test]
    fn test_federation_enable_load_balancing() {
        let federation =
            CloudFederation::new("fed", FederationType::ActiveActive, CloudProvider::AWS)
                .enable_load_balancing();

        assert!(federation.load_balancing_enabled);
    }

    #[test]
    fn test_federation_enable_auto_failover() {
        let federation =
            CloudFederation::new("fed", FederationType::ActivePassive, CloudProvider::Azure)
                .enable_auto_failover();

        assert!(federation.auto_failover_enabled);
    }

    #[test]
    fn test_federation_enable_data_sync() {
        let federation =
            CloudFederation::new("fed", FederationType::Hierarchical, CloudProvider::GCP)
                .enable_data_sync();

        assert!(federation.data_sync_enabled);
    }

    #[test]
    fn test_federation_set_status() {
        let mut federation =
            CloudFederation::new("fed", FederationType::ActiveActive, CloudProvider::AWS);

        federation.set_status(FederationStatus::Active);
        assert_eq!(federation.status, FederationStatus::Active);
    }

    #[test]
    fn test_federation_record_sync() {
        let mut federation =
            CloudFederation::new("fed", FederationType::Distributed, CloudProvider::AWS);

        federation.record_sync();
        assert!(federation.last_sync.is_some());
    }

    #[test]
    fn test_federation_is_active() {
        let mut federation =
            CloudFederation::new("fed", FederationType::ActiveActive, CloudProvider::Azure);

        assert!(!federation.is_active());

        federation.set_status(FederationStatus::Active);
        assert!(federation.is_active());
    }

    #[test]
    fn test_federation_is_multi_provider() {
        let mut federation =
            CloudFederation::new("fed", FederationType::ActiveActive, CloudProvider::AWS);

        assert!(!federation.is_multi_provider());

        federation.add_member(CloudProvider::Azure);
        assert!(federation.is_multi_provider());
    }

    #[test]
    fn test_orchestration_policy() {
        let policy = OrchestrationPolicy::new(
            "placement-policy",
            "fed-123",
            PolicyType::Placement,
            SchedulingStrategy::LeastLoaded,
        );

        assert_eq!(policy.name, "placement-policy");
        assert_eq!(policy.federation_id, "fed-123");
        assert_eq!(policy.policy_type, PolicyType::Placement);
        assert_eq!(policy.scheduling_strategy, SchedulingStrategy::LeastLoaded);
        assert!(policy.enabled);
        assert_eq!(policy.priority, 100);
    }

    #[test]
    fn test_policy_with_priority() {
        let policy = OrchestrationPolicy::new(
            "policy",
            "fed-1",
            PolicyType::Scaling,
            SchedulingStrategy::CostOptimized,
        )
        .with_priority(200);

        assert_eq!(policy.priority, 200);
    }

    #[test]
    fn test_policy_add_condition() {
        let mut policy = OrchestrationPolicy::new(
            "policy",
            "fed-1",
            PolicyType::Migration,
            SchedulingStrategy::AffinityBased,
        );

        policy.add_condition("cpu_usage > 80");
        policy.add_condition("memory_usage > 90");

        assert_eq!(policy.condition_count(), 2);
    }

    #[test]
    fn test_policy_add_action() {
        let mut policy = OrchestrationPolicy::new(
            "policy",
            "fed-1",
            PolicyType::Failover,
            SchedulingStrategy::Custom,
        );

        policy.add_action("migrate_workload");
        policy.add_action("send_notification");

        assert_eq!(policy.action_count(), 2);
    }

    #[test]
    fn test_policy_enable_disable() {
        let mut policy = OrchestrationPolicy::new(
            "policy",
            "fed-1",
            PolicyType::CostOptimization,
            SchedulingStrategy::RoundRobin,
        );

        assert!(policy.enabled);

        policy.disable();
        assert!(!policy.enabled);

        policy.enable();
        assert!(policy.enabled);
    }

    #[test]
    fn test_federation_event() {
        let event = FederationEvent::new(
            "fed-123",
            EventType::MemberJoined,
            CloudProvider::AWS,
            "New member joined the federation",
            EventSeverity::Info,
        );

        assert_eq!(event.federation_id, "fed-123");
        assert_eq!(event.event_type, EventType::MemberJoined);
        assert_eq!(event.provider, CloudProvider::AWS);
        assert_eq!(event.severity, EventSeverity::Info);
    }

    #[test]
    fn test_event_is_critical() {
        let event1 = FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::AWS,
            "Critical error",
            EventSeverity::Critical,
        );
        assert!(event1.is_critical());

        let event2 = FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::Azure,
            "Minor error",
            EventSeverity::Error,
        );
        assert!(!event2.is_critical());
    }

    #[test]
    fn test_event_is_error() {
        let event1 = FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::AWS,
            "Error occurred",
            EventSeverity::Error,
        );
        assert!(event1.is_error());

        let event2 = FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::Azure,
            "Critical error",
            EventSeverity::Critical,
        );
        assert!(event2.is_error());

        let event3 = FederationEvent::new(
            "fed-1",
            EventType::DataSync,
            CloudProvider::GCP,
            "Data synced",
            EventSeverity::Info,
        );
        assert!(!event3.is_error());
    }

    #[test]
    fn test_federation_manager() {
        let mut manager = FederationManager::new();

        let federation =
            CloudFederation::new("fed", FederationType::ActiveActive, CloudProvider::AWS);
        let id = manager.add_federation(federation);

        assert_eq!(manager.federation_count(), 1);
        assert!(manager.get_federation(&id).is_some());
    }

    #[test]
    fn test_manager_add_policy() {
        let mut manager = FederationManager::new();

        let policy = OrchestrationPolicy::new(
            "policy",
            "fed-1",
            PolicyType::Placement,
            SchedulingStrategy::LeastLoaded,
        );
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_add_event() {
        let mut manager = FederationManager::new();

        let event = FederationEvent::new(
            "fed-1",
            EventType::MemberJoined,
            CloudProvider::AWS,
            "Member joined",
            EventSeverity::Info,
        );
        manager.add_event(event);

        assert_eq!(manager.event_count(), 1);
    }

    #[test]
    fn test_manager_active_federations() {
        let mut manager = FederationManager::new();

        let mut federation1 =
            CloudFederation::new("f1", FederationType::ActiveActive, CloudProvider::AWS);
        federation1.set_status(FederationStatus::Active);

        let federation2 =
            CloudFederation::new("f2", FederationType::ActivePassive, CloudProvider::Azure);

        manager.add_federation(federation1);
        manager.add_federation(federation2);

        let active = manager.active_federations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_multi_provider_federations() {
        let mut manager = FederationManager::new();

        let mut federation1 =
            CloudFederation::new("f1", FederationType::Distributed, CloudProvider::AWS);
        federation1.add_member(CloudProvider::Azure);

        let federation2 =
            CloudFederation::new("f2", FederationType::Hierarchical, CloudProvider::GCP);

        manager.add_federation(federation1);
        manager.add_federation(federation2);

        let multi = manager.multi_provider_federations();
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn test_manager_federations_with_failover() {
        let mut manager = FederationManager::new();

        let federation1 =
            CloudFederation::new("f1", FederationType::ActivePassive, CloudProvider::AWS)
                .enable_auto_failover();
        let federation2 =
            CloudFederation::new("f2", FederationType::ActiveActive, CloudProvider::Azure);

        manager.add_federation(federation1);
        manager.add_federation(federation2);

        let with_failover = manager.federations_with_failover();
        assert_eq!(with_failover.len(), 1);
    }

    #[test]
    fn test_manager_policies_for_federation() {
        let mut manager = FederationManager::new();

        manager.add_policy(OrchestrationPolicy::new(
            "p1",
            "fed-1",
            PolicyType::Placement,
            SchedulingStrategy::LeastLoaded,
        ));
        manager.add_policy(OrchestrationPolicy::new(
            "p2",
            "fed-2",
            PolicyType::Scaling,
            SchedulingStrategy::CostOptimized,
        ));
        manager.add_policy(OrchestrationPolicy::new(
            "p3",
            "fed-1",
            PolicyType::Migration,
            SchedulingStrategy::AffinityBased,
        ));

        let policies = manager.policies_for_federation("fed-1");
        assert_eq!(policies.len(), 2);
    }

    #[test]
    fn test_manager_enabled_policies() {
        let mut manager = FederationManager::new();

        let policy1 = OrchestrationPolicy::new(
            "p1",
            "fed-1",
            PolicyType::Placement,
            SchedulingStrategy::LeastLoaded,
        );
        let mut policy2 = OrchestrationPolicy::new(
            "p2",
            "fed-1",
            PolicyType::Scaling,
            SchedulingStrategy::CostOptimized,
        );
        policy2.disable();

        manager.add_policy(policy1);
        manager.add_policy(policy2);

        let enabled = manager.enabled_policies();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_events_for_federation() {
        let mut manager = FederationManager::new();

        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::MemberJoined,
            CloudProvider::AWS,
            "Joined",
            EventSeverity::Info,
        ));
        manager.add_event(FederationEvent::new(
            "fed-2",
            EventType::DataSync,
            CloudProvider::Azure,
            "Synced",
            EventSeverity::Info,
        ));
        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::Failover,
            CloudProvider::GCP,
            "Failover",
            EventSeverity::Warning,
        ));

        let events = manager.events_for_federation("fed-1");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_manager_critical_events() {
        let mut manager = FederationManager::new();

        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::AWS,
            "Critical error",
            EventSeverity::Critical,
        ));
        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::Azure,
            "Minor error",
            EventSeverity::Error,
        ));
        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::DataSync,
            CloudProvider::GCP,
            "Synced",
            EventSeverity::Info,
        ));

        let critical = manager.critical_events();
        assert_eq!(critical.len(), 1);
    }

    #[test]
    fn test_manager_error_events() {
        let mut manager = FederationManager::new();

        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::AWS,
            "Critical error",
            EventSeverity::Critical,
        ));
        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::Error,
            CloudProvider::Azure,
            "Error",
            EventSeverity::Error,
        ));
        manager.add_event(FederationEvent::new(
            "fed-1",
            EventType::DataSync,
            CloudProvider::GCP,
            "Synced",
            EventSeverity::Info,
        ));

        let errors = manager.error_events();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_federation_type_equality() {
        assert_eq!(FederationType::ActiveActive, FederationType::ActiveActive);
        assert_ne!(FederationType::ActiveActive, FederationType::ActivePassive);
    }

    #[test]
    fn test_federation_status_equality() {
        assert_eq!(FederationStatus::Active, FederationStatus::Active);
        assert_ne!(FederationStatus::Active, FederationStatus::Failed);
    }

    #[test]
    fn test_policy_type_equality() {
        assert_eq!(PolicyType::Placement, PolicyType::Placement);
        assert_ne!(PolicyType::Placement, PolicyType::Scaling);
    }

    #[test]
    fn test_scheduling_strategy_equality() {
        assert_eq!(
            SchedulingStrategy::RoundRobin,
            SchedulingStrategy::RoundRobin
        );
        assert_ne!(
            SchedulingStrategy::RoundRobin,
            SchedulingStrategy::LeastLoaded
        );
    }

    #[test]
    fn test_event_type_equality() {
        assert_eq!(EventType::MemberJoined, EventType::MemberJoined);
        assert_ne!(EventType::MemberJoined, EventType::MemberLeft);
    }

    #[test]
    fn test_event_severity_equality() {
        assert_eq!(EventSeverity::Critical, EventSeverity::Critical);
        assert_ne!(EventSeverity::Critical, EventSeverity::Info);
    }
}
