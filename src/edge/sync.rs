use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sync direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    EdgeToCloud,
    CloudToEdge,
    Bidirectional,
}

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Paused,
}

/// Data sync policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSyncPolicy {
    pub id: String,
    pub name: String,
    pub direction: SyncDirection,
    pub source: String,
    pub destination: String,
    pub sync_interval_seconds: u32,
    pub priority: u32,
    pub bandwidth_limit_mbps: Option<u32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl DataSyncPolicy {
    pub fn new(
        name: impl Into<String>,
        direction: SyncDirection,
        source: impl Into<String>,
        destination: impl Into<String>,
        interval_seconds: u32,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "sync-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            direction,
            source: source.into(),
            destination: destination.into(),
            sync_interval_seconds: interval_seconds,
            priority: 5,
            bandwidth_limit_mbps: None,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority.min(10);
        self
    }

    pub fn with_bandwidth_limit(mut self, mbps: u32) -> Self {
        self.bandwidth_limit_mbps = Some(mbps);
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn is_high_priority(&self) -> bool {
        self.priority >= 8
    }
}

/// Sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    pub id: String,
    pub policy_id: String,
    pub status: SyncStatus,
    pub bytes_transferred: u64,
    pub bytes_total: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl SyncOperation {
    pub fn new(policy_id: impl Into<String>, bytes_total: u64) -> Self {
        let policy_id_str = policy_id.into();
        let id = format!("syncop-{}-{}", policy_id_str, Utc::now().timestamp_micros());

        Self {
            id,
            policy_id: policy_id_str,
            status: SyncStatus::Pending,
            bytes_transferred: 0,
            bytes_total,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn set_status(&mut self, status: SyncStatus) {
        self.status = status;
        if matches!(self.status, SyncStatus::Completed | SyncStatus::Failed) {
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn update_progress(&mut self, bytes_transferred: u64) {
        self.bytes_transferred = bytes_transferred.min(self.bytes_total);
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
        self.set_status(SyncStatus::Failed);
    }

    pub fn progress_percent(&self) -> f64 {
        if self.bytes_total == 0 {
            return 0.0;
        }
        (self.bytes_transferred as f64 / self.bytes_total as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.status == SyncStatus::Completed
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SyncStatus::InProgress | SyncStatus::Pending)
    }
}

/// Sync manager
pub struct SyncManager {
    policies: HashMap<String, DataSyncPolicy>,
    operations: HashMap<String, SyncOperation>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            operations: HashMap::new(),
        }
    }

    pub fn add_policy(&mut self, policy: DataSyncPolicy) -> String {
        let id = policy.id.clone();
        self.policies.insert(id.clone(), policy);
        id
    }

    pub fn get_policy(&self, id: &str) -> Option<&DataSyncPolicy> {
        self.policies.get(id)
    }

    pub fn get_policy_mut(&mut self, id: &str) -> Option<&mut DataSyncPolicy> {
        self.policies.get_mut(id)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn add_operation(&mut self, operation: SyncOperation) -> String {
        let id = operation.id.clone();
        self.operations.insert(id.clone(), operation);
        id
    }

    pub fn get_operation(&self, id: &str) -> Option<&SyncOperation> {
        self.operations.get(id)
    }

    pub fn get_operation_mut(&mut self, id: &str) -> Option<&mut SyncOperation> {
        self.operations.get_mut(id)
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    pub fn enabled_policies(&self) -> Vec<&DataSyncPolicy> {
        self.policies.values().filter(|p| p.enabled).collect()
    }

    pub fn high_priority_policies(&self) -> Vec<&DataSyncPolicy> {
        self.policies
            .values()
            .filter(|p| p.is_high_priority())
            .collect()
    }

    pub fn active_operations(&self) -> Vec<&SyncOperation> {
        self.operations.values().filter(|o| o.is_active()).collect()
    }

    pub fn completed_operations(&self) -> Vec<&SyncOperation> {
        self.operations
            .values()
            .filter(|o| o.is_complete())
            .collect()
    }

    pub fn total_bytes_synced(&self) -> u64 {
        self.operations
            .values()
            .filter(|o| o.is_complete())
            .map(|o| o.bytes_total)
            .sum()
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_sync_policy() {
        let policy = DataSyncPolicy::new(
            "EdgeToCloud",
            SyncDirection::EdgeToCloud,
            "edge-storage",
            "cloud-storage",
            3600,
        );

        assert_eq!(policy.name, "EdgeToCloud");
        assert_eq!(policy.direction, SyncDirection::EdgeToCloud);
        assert_eq!(policy.sync_interval_seconds, 3600);
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_with_priority() {
        let policy = DataSyncPolicy::new("Test", SyncDirection::Bidirectional, "src", "dst", 60)
            .with_priority(9);

        assert_eq!(policy.priority, 9);
        assert!(policy.is_high_priority());
    }

    #[test]
    fn test_policy_priority_clamping() {
        let policy = DataSyncPolicy::new("Test", SyncDirection::Bidirectional, "src", "dst", 60)
            .with_priority(15);

        assert_eq!(policy.priority, 10);
    }

    #[test]
    fn test_policy_with_bandwidth_limit() {
        let policy = DataSyncPolicy::new("Test", SyncDirection::EdgeToCloud, "src", "dst", 60)
            .with_bandwidth_limit(100);

        assert_eq!(policy.bandwidth_limit_mbps, Some(100));
    }

    #[test]
    fn test_policy_disable_enable() {
        let mut policy = DataSyncPolicy::new("Test", SyncDirection::CloudToEdge, "src", "dst", 60);

        assert!(policy.enabled);

        policy.disable();
        assert!(!policy.enabled);

        policy.enable();
        assert!(policy.enabled);
    }

    #[test]
    fn test_sync_operation() {
        let operation = SyncOperation::new("policy-1", 1000000);

        assert_eq!(operation.policy_id, "policy-1");
        assert_eq!(operation.bytes_total, 1000000);
        assert_eq!(operation.bytes_transferred, 0);
        assert_eq!(operation.status, SyncStatus::Pending);
    }

    #[test]
    fn test_operation_set_status() {
        let mut operation = SyncOperation::new("policy-1", 1000000);

        operation.set_status(SyncStatus::InProgress);
        assert_eq!(operation.status, SyncStatus::InProgress);
        assert!(operation.completed_at.is_none());

        operation.set_status(SyncStatus::Completed);
        assert_eq!(operation.status, SyncStatus::Completed);
        assert!(operation.completed_at.is_some());
    }

    #[test]
    fn test_operation_update_progress() {
        let mut operation = SyncOperation::new("policy-1", 1000000);

        operation.update_progress(500000);
        assert_eq!(operation.bytes_transferred, 500000);
        assert_eq!(operation.progress_percent(), 50.0);

        operation.update_progress(1000000);
        assert_eq!(operation.progress_percent(), 100.0);
    }

    #[test]
    fn test_operation_set_error() {
        let mut operation = SyncOperation::new("policy-1", 1000000);

        operation.set_error("Network timeout");
        assert_eq!(operation.status, SyncStatus::Failed);
        assert_eq!(operation.error_message, Some("Network timeout".to_string()));
        assert!(operation.completed_at.is_some());
    }

    #[test]
    fn test_operation_is_complete() {
        let mut operation = SyncOperation::new("policy-1", 1000000);

        assert!(!operation.is_complete());

        operation.set_status(SyncStatus::Completed);
        assert!(operation.is_complete());
    }

    #[test]
    fn test_operation_is_active() {
        let mut operation = SyncOperation::new("policy-1", 1000000);

        assert!(operation.is_active()); // Pending

        operation.set_status(SyncStatus::InProgress);
        assert!(operation.is_active());

        operation.set_status(SyncStatus::Completed);
        assert!(!operation.is_active());
    }

    #[test]
    fn test_sync_manager() {
        let mut manager = SyncManager::new();

        let policy = DataSyncPolicy::new("Test", SyncDirection::EdgeToCloud, "src", "dst", 60);
        let id = manager.add_policy(policy);

        assert_eq!(manager.policy_count(), 1);
        assert!(manager.get_policy(&id).is_some());
    }

    #[test]
    fn test_manager_add_operation() {
        let mut manager = SyncManager::new();

        let operation = SyncOperation::new("policy-1", 1000000);
        let id = manager.add_operation(operation);

        assert_eq!(manager.operation_count(), 1);
        assert!(manager.get_operation(&id).is_some());
    }

    #[test]
    fn test_manager_enabled_policies() {
        let mut manager = SyncManager::new();

        let policy1 = DataSyncPolicy::new("P1", SyncDirection::EdgeToCloud, "s", "d", 60);
        let mut policy2 = DataSyncPolicy::new("P2", SyncDirection::CloudToEdge, "s", "d", 60);
        policy2.disable();

        manager.add_policy(policy1);
        manager.add_policy(policy2);

        let enabled = manager.enabled_policies();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_manager_high_priority_policies() {
        let mut manager = SyncManager::new();

        manager.add_policy(
            DataSyncPolicy::new("P1", SyncDirection::EdgeToCloud, "s", "d", 60).with_priority(9),
        );
        manager.add_policy(
            DataSyncPolicy::new("P2", SyncDirection::CloudToEdge, "s", "d", 60).with_priority(5),
        );

        let high_priority = manager.high_priority_policies();
        assert_eq!(high_priority.len(), 1);
    }

    #[test]
    fn test_manager_active_operations() {
        let mut manager = SyncManager::new();

        let mut op1 = SyncOperation::new("p1", 1000);
        op1.set_status(SyncStatus::InProgress);

        let mut op2 = SyncOperation::new("p2", 1000);
        op2.set_status(SyncStatus::Completed);

        manager.add_operation(op1);
        manager.add_operation(op2);

        let active = manager.active_operations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_completed_operations() {
        let mut manager = SyncManager::new();

        let mut op1 = SyncOperation::new("p1", 1000);
        op1.set_status(SyncStatus::Completed);

        let op2 = SyncOperation::new("p2", 1000);

        manager.add_operation(op1);
        manager.add_operation(op2);

        let completed = manager.completed_operations();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_manager_total_bytes_synced() {
        let mut manager = SyncManager::new();

        let mut op1 = SyncOperation::new("p1", 1000000);
        op1.set_status(SyncStatus::Completed);

        let mut op2 = SyncOperation::new("p2", 500000);
        op2.set_status(SyncStatus::Completed);

        let op3 = SyncOperation::new("p3", 300000); // Not completed

        manager.add_operation(op1);
        manager.add_operation(op2);
        manager.add_operation(op3);

        assert_eq!(manager.total_bytes_synced(), 1500000);
    }

    #[test]
    fn test_sync_direction_equality() {
        assert_eq!(SyncDirection::EdgeToCloud, SyncDirection::EdgeToCloud);
        assert_ne!(SyncDirection::EdgeToCloud, SyncDirection::CloudToEdge);
    }

    #[test]
    fn test_sync_status_equality() {
        assert_eq!(SyncStatus::InProgress, SyncStatus::InProgress);
        assert_ne!(SyncStatus::InProgress, SyncStatus::Completed);
    }
}
