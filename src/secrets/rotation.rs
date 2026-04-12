use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rotation strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStrategy {
    Automatic,
    Manual,
    OnDemand,
}

/// Rotation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    pub id: String,
    pub name: String,
    pub strategy: RotationStrategy,
    pub rotation_interval_days: u32,
    pub secret_ids: Vec<String>,
    pub notification_enabled: bool,
    pub auto_delete_old: bool,
    pub retain_versions: u32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl RotationPolicy {
    pub fn new(name: impl Into<String>, strategy: RotationStrategy, interval_days: u32) -> Self {
        let name_str = name.into();
        let id = format!(
            "rotpolicy-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            strategy,
            rotation_interval_days: interval_days,
            secret_ids: Vec::new(),
            notification_enabled: true,
            auto_delete_old: false,
            retain_versions: 3,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn add_secret(&mut self, secret_id: impl Into<String>) {
        self.secret_ids.push(secret_id.into());
    }

    pub fn enable_auto_delete(mut self, retain_versions: u32) -> Self {
        self.auto_delete_old = true;
        self.retain_versions = retain_versions;
        self
    }

    pub fn disable_notifications(mut self) -> Self {
        self.notification_enabled = false;
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn secret_count(&self) -> usize {
        self.secret_ids.len()
    }
}

/// Rotation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationEvent {
    pub id: String,
    pub policy_id: String,
    pub secret_id: String,
    pub old_version: u32,
    pub new_version: u32,
    pub status: RotationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl RotationEvent {
    pub fn new(
        policy_id: impl Into<String>,
        secret_id: impl Into<String>,
        old_version: u32,
    ) -> Self {
        let policy_id_str = policy_id.into();
        let secret_id_str = secret_id.into();
        let id = format!(
            "rotevent-{}-{}",
            secret_id_str,
            Utc::now().timestamp_micros()
        );

        Self {
            id,
            policy_id: policy_id_str,
            secret_id: secret_id_str,
            old_version,
            new_version: old_version + 1,
            status: RotationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn set_status(&mut self, status: RotationStatus) {
        self.status = status;
        if matches!(
            self.status,
            RotationStatus::Completed | RotationStatus::Failed
        ) {
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
        self.set_status(RotationStatus::Failed);
    }

    pub fn is_complete(&self) -> bool {
        self.status == RotationStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == RotationStatus::Failed
    }
}

/// Rotation manager
pub struct RotationManager {
    policies: HashMap<String, RotationPolicy>,
    events: HashMap<String, RotationEvent>,
}

impl RotationManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            events: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: RotationPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&RotationPolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut RotationPolicy> {
        self.policies.get_mut(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn add_event(&mut self, event: RotationEvent) -> String {
        let id = event.id.clone();
        self.events.insert(id.clone(), event);
        id
    }

    pub fn get_event(&self, id: &str) -> Option<&RotationEvent> {
        self.events.get(id)
    }

    pub fn get_event_mut(&mut self, id: &str) -> Option<&mut RotationEvent> {
        self.events.get_mut(id)
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn enabled_policies(&self) -> Vec<&RotationPolicy> {
        self.policies.values().filter(|p| p.enabled).collect()
    }

    pub fn automatic_policies(&self) -> Vec<&RotationPolicy> {
        self.policies
            .values()
            .filter(|p| p.strategy == RotationStrategy::Automatic)
            .collect()
    }

    pub fn events_for_secret(&self, secret_id: &str) -> Vec<&RotationEvent> {
        self.events
            .values()
            .filter(|e| e.secret_id == secret_id)
            .collect()
    }

    pub fn completed_events(&self) -> Vec<&RotationEvent> {
        self.events.values().filter(|e| e.is_complete()).collect()
    }

    pub fn failed_events(&self) -> Vec<&RotationEvent> {
        self.events.values().filter(|e| e.is_failed()).collect()
    }
}

impl Default for RotationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_policy() {
        let policy = RotationPolicy::new("30-day-rotation", RotationStrategy::Automatic, 30);

        assert_eq!(policy.name, "30-day-rotation");
        assert_eq!(policy.strategy, RotationStrategy::Automatic);
        assert_eq!(policy.rotation_interval_days, 30);
        assert!(policy.enabled);
        assert!(policy.notification_enabled);
    }

    #[test]
    fn test_policy_add_secret() {
        let mut policy = RotationPolicy::new("test", RotationStrategy::Automatic, 30);

        policy.add_secret("secret-1");
        policy.add_secret("secret-2");

        assert_eq!(policy.secret_count(), 2);
    }

    #[test]
    fn test_policy_enable_auto_delete() {
        let policy =
            RotationPolicy::new("test", RotationStrategy::Automatic, 30).enable_auto_delete(5);

        assert!(policy.auto_delete_old);
        assert_eq!(policy.retain_versions, 5);
    }

    #[test]
    fn test_policy_disable_notifications() {
        let policy =
            RotationPolicy::new("test", RotationStrategy::Automatic, 30).disable_notifications();

        assert!(!policy.notification_enabled);
    }

    #[test]
    fn test_policy_disable_enable() {
        let mut policy = RotationPolicy::new("test", RotationStrategy::Automatic, 30);

        assert!(policy.enabled);

        policy.disable();
        assert!(!policy.enabled);

        policy.enable();
        assert!(policy.enabled);
    }

    #[test]
    fn test_rotation_event() {
        let event = RotationEvent::new("policy-1", "secret-123", 5);

        assert_eq!(event.policy_id, "policy-1");
        assert_eq!(event.secret_id, "secret-123");
        assert_eq!(event.old_version, 5);
        assert_eq!(event.new_version, 6);
        assert_eq!(event.status, RotationStatus::Pending);
    }

    #[test]
    fn test_event_set_status() {
        let mut event = RotationEvent::new("policy-1", "secret-1", 1);

        event.set_status(RotationStatus::InProgress);
        assert_eq!(event.status, RotationStatus::InProgress);
        assert!(event.completed_at.is_none());

        event.set_status(RotationStatus::Completed);
        assert_eq!(event.status, RotationStatus::Completed);
        assert!(event.completed_at.is_some());
    }

    #[test]
    fn test_event_set_error() {
        let mut event = RotationEvent::new("policy-1", "secret-1", 1);

        event.set_error("Rotation failed due to timeout");

        assert_eq!(event.status, RotationStatus::Failed);
        assert_eq!(
            event.error_message,
            Some("Rotation failed due to timeout".to_string())
        );
        assert!(event.completed_at.is_some());
    }

    #[test]
    fn test_event_is_complete() {
        let mut event = RotationEvent::new("policy-1", "secret-1", 1);

        assert!(!event.is_complete());

        event.set_status(RotationStatus::Completed);
        assert!(event.is_complete());
    }

    #[test]
    fn test_event_is_failed() {
        let mut event = RotationEvent::new("policy-1", "secret-1", 1);

        assert!(!event.is_failed());

        event.set_error("Test error");
        assert!(event.is_failed());
    }

    #[test]
    fn test_rotation_manager() {
        let mut manager = RotationManager::new();

        let policy = RotationPolicy::new("test", RotationStrategy::Automatic, 30);
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_add_event() {
        let mut manager = RotationManager::new();

        let event = RotationEvent::new("policy-1", "secret-1", 1);
        let id = manager.add_event(event);

        assert_eq!(manager.event_count(), 1);
        assert!(manager.get_event(&id).is_some());
    }

    #[test]
    fn test_manager_enabled_policies() {
        let mut manager = RotationManager::new();

        let policy1 = RotationPolicy::new("p1", RotationStrategy::Automatic, 30);
        let mut policy2 = RotationPolicy::new("p2", RotationStrategy::Manual, 60);
        policy2.disable();

        manager.add_policy(policy1);
        manager.add_policy(policy2);

        let enabled = manager.enabled_policies();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_automatic_policies() {
        let mut manager = RotationManager::new();

        manager.add_policy(RotationPolicy::new("p1", RotationStrategy::Automatic, 30));
        manager.add_policy(RotationPolicy::new("p2", RotationStrategy::Manual, 60));
        manager.add_policy(RotationPolicy::new("p3", RotationStrategy::Automatic, 90));

        let automatic = manager.automatic_policies();
        assert_eq!(automatic.len(), 2);
    }

    #[test]
    fn test_manager_events_for_secret() {
        let mut manager = RotationManager::new();

        manager.add_event(RotationEvent::new("policy-1", "secret-1", 1));
        manager.add_event(RotationEvent::new("policy-1", "secret-2", 1));
        manager.add_event(RotationEvent::new("policy-1", "secret-1", 2));

        let events = manager.events_for_secret("secret-1");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_manager_completed_events() {
        let mut manager = RotationManager::new();

        let mut event1 = RotationEvent::new("policy-1", "secret-1", 1);
        event1.set_status(RotationStatus::Completed);

        let event2 = RotationEvent::new("policy-1", "secret-2", 1);

        manager.add_event(event1);
        manager.add_event(event2);

        let completed = manager.completed_events();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_manager_failed_events() {
        let mut manager = RotationManager::new();

        let mut event1 = RotationEvent::new("policy-1", "secret-1", 1);
        event1.set_error("Failed");

        let event2 = RotationEvent::new("policy-1", "secret-2", 1);

        manager.add_event(event1);
        manager.add_event(event2);

        let failed = manager.failed_events();
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_rotation_strategy_equality() {
        assert_eq!(RotationStrategy::Automatic, RotationStrategy::Automatic);
        assert_ne!(RotationStrategy::Automatic, RotationStrategy::Manual);
    }

    #[test]
    fn test_rotation_status_equality() {
        assert_eq!(RotationStatus::Completed, RotationStatus::Completed);
        assert_ne!(RotationStatus::Completed, RotationStatus::Failed);
    }
}
