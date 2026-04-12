use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::DRSite;

/// Failover status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverStatus {
    Idle,
    Initiating,
    InProgress,
    Validating,
    Completed,
    Failed,
    RollingBack,
}

impl std::fmt::Display for FailoverStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailoverStatus::Idle => write!(f, "Idle"),
            FailoverStatus::Initiating => write!(f, "Initiating"),
            FailoverStatus::InProgress => write!(f, "In Progress"),
            FailoverStatus::Validating => write!(f, "Validating"),
            FailoverStatus::Completed => write!(f, "Completed"),
            FailoverStatus::Failed => write!(f, "Failed"),
            FailoverStatus::RollingBack => write!(f, "Rolling Back"),
        }
    }
}

/// Failover type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverType {
    Planned,
    Unplanned,
    Test,
}

/// Failover event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEvent {
    pub id: String,
    pub name: String,
    pub failover_type: FailoverType,
    pub source_site: DRSite,
    pub target_site: DRSite,
    pub status: FailoverStatus,
    pub triggered_by: String,
    pub reason: String,
    pub resources_affected: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub success: bool,
}

impl FailoverEvent {
    pub fn new(
        name: impl Into<String>,
        failover_type: FailoverType,
        source: DRSite,
        target: DRSite,
        triggered_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "failover-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            failover_type,
            source_site: source,
            target_site: target,
            status: FailoverStatus::Idle,
            triggered_by: triggered_by.into(),
            reason: reason.into(),
            resources_affected: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            duration_seconds: None,
            success: false,
        }
    }

    pub fn add_resource(&mut self, resource_id: impl Into<String>) {
        self.resources_affected.push(resource_id.into());
    }

    pub fn initiate(&mut self) {
        self.status = FailoverStatus::Initiating;
    }

    pub fn start(&mut self) {
        self.status = FailoverStatus::InProgress;
        self.started_at = Utc::now();
    }

    pub fn validate(&mut self) {
        self.status = FailoverStatus::Validating;
    }

    pub fn complete(&mut self, success: bool) {
        self.status = if success {
            FailoverStatus::Completed
        } else {
            FailoverStatus::Failed
        };
        let now = Utc::now();
        self.completed_at = Some(now);
        let duration = (now - self.started_at).num_seconds();
        self.duration_seconds = Some(if duration >= 0 { duration as u64 } else { 0 });
        self.success = success;
    }

    pub fn rollback(&mut self) {
        self.status = FailoverStatus::RollingBack;
    }

    pub fn is_in_progress(&self) -> bool {
        matches!(
            self.status,
            FailoverStatus::Initiating | FailoverStatus::InProgress | FailoverStatus::Validating
        )
    }

    pub fn is_completed(&self) -> bool {
        self.status == FailoverStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == FailoverStatus::Failed
    }

    pub fn is_test(&self) -> bool {
        self.failover_type == FailoverType::Test
    }
}

/// Failback event (reverse failover)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailbackEvent {
    pub id: String,
    pub original_failover_id: String,
    pub source_site: DRSite,
    pub target_site: DRSite,
    pub status: FailoverStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
}

impl FailbackEvent {
    pub fn new(failover_id: impl Into<String>, source: DRSite, target: DRSite) -> Self {
        let failover_str = failover_id.into();
        let id = format!("failback-{}-{}", failover_str, Utc::now().timestamp());

        Self {
            id,
            original_failover_id: failover_str,
            source_site: source,
            target_site: target,
            status: FailoverStatus::Idle,
            started_at: Utc::now(),
            completed_at: None,
            success: false,
        }
    }

    pub fn start(&mut self) {
        self.status = FailoverStatus::InProgress;
        self.started_at = Utc::now();
    }

    pub fn complete(&mut self, success: bool) {
        self.status = if success {
            FailoverStatus::Completed
        } else {
            FailoverStatus::Failed
        };
        self.completed_at = Some(Utc::now());
        self.success = success;
    }
}

/// Failover manager
pub struct FailoverManager {
    events: HashMap<String, FailoverEvent>,
    failbacks: HashMap<String, FailbackEvent>,
}

impl FailoverManager {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            failbacks: HashMap::new(),
        }
    }

    pub fn add_event(&mut self, event: FailoverEvent) -> String {
        let id = event.id.clone();
        self.events.insert(id.clone(), event);
        id
    }

    pub fn get_event(&self, id: &str) -> Option<&FailoverEvent> {
        self.events.get(id)
    }

    pub fn get_event_mut(&mut self, id: &str) -> Option<&mut FailoverEvent> {
        self.events.get_mut(id)
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn add_failback(&mut self, failback: FailbackEvent) -> String {
        let id = failback.id.clone();
        self.failbacks.insert(id.clone(), failback);
        id
    }

    pub fn get_failback(&self, id: &str) -> Option<&FailbackEvent> {
        self.failbacks.get(id)
    }

    pub fn get_failback_mut(&mut self, id: &str) -> Option<&mut FailbackEvent> {
        self.failbacks.get_mut(id)
    }

    pub fn failback_count(&self) -> usize {
        self.failbacks.len()
    }

    pub fn active_failovers(&self) -> Vec<&FailoverEvent> {
        self.events
            .values()
            .filter(|e| e.is_in_progress())
            .collect()
    }

    pub fn completed_failovers(&self) -> Vec<&FailoverEvent> {
        self.events.values().filter(|e| e.is_completed()).collect()
    }

    pub fn failed_failovers(&self) -> Vec<&FailoverEvent> {
        self.events.values().filter(|e| e.is_failed()).collect()
    }

    pub fn test_failovers(&self) -> Vec<&FailoverEvent> {
        self.events.values().filter(|e| e.is_test()).collect()
    }

    pub fn by_target_site(&self, site: &DRSite) -> Vec<&FailoverEvent> {
        self.events
            .values()
            .filter(|e| &e.target_site == site)
            .collect()
    }
}

impl Default for FailoverManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_status_display() {
        assert_eq!(FailoverStatus::Idle.to_string(), "Idle");
        assert_eq!(FailoverStatus::InProgress.to_string(), "In Progress");
        assert_eq!(FailoverStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_failover_event() {
        let event = FailoverEvent::new(
            "Production Failover",
            FailoverType::Unplanned,
            DRSite::Primary,
            DRSite::Secondary,
            "admin@example.com",
            "Primary site outage",
        );

        assert_eq!(event.name, "Production Failover");
        assert_eq!(event.source_site, DRSite::Primary);
        assert_eq!(event.target_site, DRSite::Secondary);
        assert_eq!(event.status, FailoverStatus::Idle);
        assert!(!event.success);
    }

    #[test]
    fn test_failover_lifecycle() {
        let mut event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "testing",
        );

        assert_eq!(event.status, FailoverStatus::Idle);

        event.initiate();
        assert_eq!(event.status, FailoverStatus::Initiating);
        assert!(event.is_in_progress());

        event.start();
        assert_eq!(event.status, FailoverStatus::InProgress);

        event.validate();
        assert_eq!(event.status, FailoverStatus::Validating);

        event.complete(true);
        assert_eq!(event.status, FailoverStatus::Completed);
        assert!(event.is_completed());
        assert!(event.success);
        assert!(event.completed_at.is_some());
        assert!(event.duration_seconds.is_some());
    }

    #[test]
    fn test_failover_failed() {
        let mut event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );

        event.complete(false);
        assert_eq!(event.status, FailoverStatus::Failed);
        assert!(event.is_failed());
        assert!(!event.success);
    }

    #[test]
    fn test_failover_rollback() {
        let mut event = FailoverEvent::new(
            "Test",
            FailoverType::Planned,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );

        event.rollback();
        assert_eq!(event.status, FailoverStatus::RollingBack);
    }

    #[test]
    fn test_failover_add_resources() {
        let mut event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );

        event.add_resource("vm-1");
        event.add_resource("vm-2");
        event.add_resource("vm-3");

        assert_eq!(event.resources_affected.len(), 3);
    }

    #[test]
    fn test_failover_is_test() {
        let test_event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );

        let planned_event = FailoverEvent::new(
            "Planned",
            FailoverType::Planned,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "planned",
        );

        assert!(test_event.is_test());
        assert!(!planned_event.is_test());
    }

    #[test]
    fn test_failback_event() {
        let failback = FailbackEvent::new("failover-123", DRSite::Secondary, DRSite::Primary);

        assert_eq!(failback.original_failover_id, "failover-123");
        assert_eq!(failback.source_site, DRSite::Secondary);
        assert_eq!(failback.target_site, DRSite::Primary);
        assert_eq!(failback.status, FailoverStatus::Idle);
        assert!(!failback.success);
    }

    #[test]
    fn test_failback_lifecycle() {
        let mut failback = FailbackEvent::new("failover-123", DRSite::Secondary, DRSite::Primary);

        failback.start();
        assert_eq!(failback.status, FailoverStatus::InProgress);

        failback.complete(true);
        assert_eq!(failback.status, FailoverStatus::Completed);
        assert!(failback.success);
        assert!(failback.completed_at.is_some());
    }

    #[test]
    fn test_failover_manager() {
        let mut manager = FailoverManager::new();

        let event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );
        let id = manager.add_event(event);

        assert_eq!(manager.event_count(), 1);
        assert!(manager.get_event(&id).is_some());
    }

    #[test]
    fn test_manager_failbacks() {
        let mut manager = FailoverManager::new();

        let failback = FailbackEvent::new("failover-123", DRSite::Secondary, DRSite::Primary);
        let id = manager.add_failback(failback);

        assert_eq!(manager.failback_count(), 1);
        assert!(manager.get_failback(&id).is_some());
    }

    #[test]
    fn test_manager_active_failovers() {
        let mut manager = FailoverManager::new();

        let mut event1 = FailoverEvent::new(
            "E1",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event1.start();

        let mut event2 = FailoverEvent::new(
            "E2",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event2.complete(true);

        manager.add_event(event1);
        manager.add_event(event2);

        let active = manager.active_failovers();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_completed_failovers() {
        let mut manager = FailoverManager::new();

        let mut event1 = FailoverEvent::new(
            "E1",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event1.complete(true);

        let mut event2 = FailoverEvent::new(
            "E2",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event2.complete(true);

        let event3 = FailoverEvent::new(
            "E3",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );

        manager.add_event(event1);
        manager.add_event(event2);
        manager.add_event(event3);

        let completed = manager.completed_failovers();
        assert_eq!(completed.len(), 2);
    }

    #[test]
    fn test_manager_failed_failovers() {
        let mut manager = FailoverManager::new();

        let mut event1 = FailoverEvent::new(
            "E1",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event1.complete(false);

        let mut event2 = FailoverEvent::new(
            "E2",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        );
        event2.complete(true);

        manager.add_event(event1);
        manager.add_event(event2);

        let failed = manager.failed_failovers();
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_manager_test_failovers() {
        let mut manager = FailoverManager::new();

        manager.add_event(FailoverEvent::new(
            "E1",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        ));
        manager.add_event(FailoverEvent::new(
            "E2",
            FailoverType::Planned,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "p",
        ));
        manager.add_event(FailoverEvent::new(
            "E3",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        ));

        let tests = manager.test_failovers();
        assert_eq!(tests.len(), 2);
    }

    #[test]
    fn test_manager_by_target_site() {
        let mut manager = FailoverManager::new();

        manager.add_event(FailoverEvent::new(
            "E1",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        ));
        manager.add_event(FailoverEvent::new(
            "E2",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Tertiary,
            "u",
            "t",
        ));
        manager.add_event(FailoverEvent::new(
            "E3",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "u",
            "t",
        ));

        let secondary = manager.by_target_site(&DRSite::Secondary);
        assert_eq!(secondary.len(), 2);
    }

    #[test]
    fn test_manager_get_event_mut() {
        let mut manager = FailoverManager::new();

        let event = FailoverEvent::new(
            "Test",
            FailoverType::Test,
            DRSite::Primary,
            DRSite::Secondary,
            "user",
            "test",
        );
        let id = manager.add_event(event);

        if let Some(event_mut) = manager.get_event_mut(&id) {
            event_mut.start();
        }

        let event = manager.get_event(&id).unwrap();
        assert_eq!(event.status, FailoverStatus::InProgress);
    }

    #[test]
    fn test_manager_get_failback_mut() {
        let mut manager = FailoverManager::new();

        let failback = FailbackEvent::new("failover-123", DRSite::Secondary, DRSite::Primary);
        let id = manager.add_failback(failback);

        if let Some(failback_mut) = manager.get_failback_mut(&id) {
            failback_mut.start();
        }

        let failback = manager.get_failback(&id).unwrap();
        assert_eq!(failback.status, FailoverStatus::InProgress);
    }

    #[test]
    fn test_failover_status_equality() {
        assert_eq!(FailoverStatus::Completed, FailoverStatus::Completed);
        assert_ne!(FailoverStatus::Completed, FailoverStatus::Failed);
    }

    #[test]
    fn test_failover_type_equality() {
        assert_eq!(FailoverType::Planned, FailoverType::Planned);
        assert_ne!(FailoverType::Planned, FailoverType::Unplanned);
    }
}
