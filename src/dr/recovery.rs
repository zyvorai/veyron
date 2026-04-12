use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Recovery point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPoint {
    pub id: String,
    pub name: String,
    pub resource_id: String,
    pub snapshot_id: String,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    pub consistent: bool,
    pub metadata: HashMap<String, String>,
}

impl RecoveryPoint {
    pub fn new(
        name: impl Into<String>,
        resource_id: impl Into<String>,
        snapshot_id: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rp-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            resource_id: resource_id.into(),
            snapshot_id: snapshot_id.into(),
            timestamp: Utc::now(),
            size_bytes: 0,
            consistent: true,
            metadata: HashMap::new(),
        }
    }

    pub fn with_size(mut self, bytes: u64) -> Self {
        self.size_bytes = bytes;
        self
    }

    pub fn with_consistency(mut self, consistent: bool) -> Self {
        self.consistent = consistent;
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn age_hours(&self) -> i64 {
        (Utc::now() - self.timestamp).num_hours()
    }

    pub fn age_days(&self) -> i64 {
        (Utc::now() - self.timestamp).num_days()
    }
}

/// Recovery operation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    Pending,
    InProgress,
    Validating,
    Completed,
    Failed,
}

impl std::fmt::Display for RecoveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryStatus::Pending => write!(f, "Pending"),
            RecoveryStatus::InProgress => write!(f, "In Progress"),
            RecoveryStatus::Validating => write!(f, "Validating"),
            RecoveryStatus::Completed => write!(f, "Completed"),
            RecoveryStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOperation {
    pub id: String,
    pub name: String,
    pub recovery_point_id: String,
    pub target_resource: String,
    pub target_site: String,
    pub status: RecoveryStatus,
    pub initiated_by: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl RecoveryOperation {
    pub fn new(
        name: impl Into<String>,
        recovery_point_id: impl Into<String>,
        target: impl Into<String>,
        site: impl Into<String>,
        initiated_by: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rec-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            recovery_point_id: recovery_point_id.into(),
            target_resource: target.into(),
            target_site: site.into(),
            status: RecoveryStatus::Pending,
            initiated_by: initiated_by.into(),
            started_at: Utc::now(),
            completed_at: None,
            duration_seconds: None,
            success: false,
            error_message: None,
        }
    }

    pub fn start(&mut self) {
        self.status = RecoveryStatus::InProgress;
        self.started_at = Utc::now();
    }

    pub fn validate(&mut self) {
        self.status = RecoveryStatus::Validating;
    }

    pub fn complete(&mut self, success: bool, error: Option<String>) {
        self.status = if success {
            RecoveryStatus::Completed
        } else {
            RecoveryStatus::Failed
        };
        let now = Utc::now();
        self.completed_at = Some(now);
        let duration = (now - self.started_at).num_seconds();
        self.duration_seconds = Some(if duration >= 0 { duration as u64 } else { 0 });
        self.success = success;
        self.error_message = error;
    }

    pub fn is_in_progress(&self) -> bool {
        matches!(
            self.status,
            RecoveryStatus::InProgress | RecoveryStatus::Validating
        )
    }

    pub fn is_completed(&self) -> bool {
        self.status == RecoveryStatus::Completed
    }
}

/// Recovery manager
pub struct RecoveryManager {
    recovery_points: HashMap<String, RecoveryPoint>,
    operations: HashMap<String, RecoveryOperation>,
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            recovery_points: HashMap::new(),
            operations: HashMap::new(),
        }
    }

    pub fn add_recovery_point(&mut self, point: RecoveryPoint) -> String {
        let id = point.id.clone();
        self.recovery_points.insert(id.clone(), point);
        id
    }

    pub fn get_recovery_point(&self, id: &str) -> Option<&RecoveryPoint> {
        self.recovery_points.get(id)
    }

    pub fn remove_recovery_point(&mut self, id: &str) -> bool {
        self.recovery_points.remove(id).is_some()
    }

    pub fn recovery_point_count(&self) -> usize {
        self.recovery_points.len()
    }

    pub fn add_operation(&mut self, operation: RecoveryOperation) -> String {
        let id = operation.id.clone();
        self.operations.insert(id.clone(), operation);
        id
    }

    pub fn get_operation(&self, id: &str) -> Option<&RecoveryOperation> {
        self.operations.get(id)
    }

    pub fn get_operation_mut(&mut self, id: &str) -> Option<&mut RecoveryOperation> {
        self.operations.get_mut(id)
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    pub fn by_resource(&self, resource_id: &str) -> Vec<&RecoveryPoint> {
        self.recovery_points
            .values()
            .filter(|p| p.resource_id == resource_id)
            .collect()
    }

    pub fn consistent_points(&self) -> Vec<&RecoveryPoint> {
        self.recovery_points
            .values()
            .filter(|p| p.consistent)
            .collect()
    }

    pub fn active_operations(&self) -> Vec<&RecoveryOperation> {
        self.operations
            .values()
            .filter(|o| o.is_in_progress())
            .collect()
    }

    pub fn completed_operations(&self) -> Vec<&RecoveryOperation> {
        self.operations
            .values()
            .filter(|o| o.status == RecoveryStatus::Completed || o.status == RecoveryStatus::Failed)
            .collect()
    }

    pub fn old_recovery_points(&self, max_age_days: i64) -> Vec<&RecoveryPoint> {
        self.recovery_points
            .values()
            .filter(|p| p.age_days() > max_age_days)
            .collect()
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_point() {
        let point = RecoveryPoint::new("RP-1", "vm-123", "snap-456");

        assert_eq!(point.name, "RP-1");
        assert_eq!(point.resource_id, "vm-123");
        assert_eq!(point.snapshot_id, "snap-456");
        assert!(point.consistent);
        assert_eq!(point.size_bytes, 0);
    }

    #[test]
    fn test_recovery_point_builder() {
        let point = RecoveryPoint::new("RP", "vm", "snap")
            .with_size(1024000)
            .with_consistency(false);

        assert_eq!(point.size_bytes, 1024000);
        assert!(!point.consistent);
    }

    #[test]
    fn test_recovery_point_metadata() {
        let mut point = RecoveryPoint::new("RP", "vm", "snap");

        point.add_metadata("type", "full");
        point.add_metadata("source", "automated");

        assert_eq!(point.metadata.len(), 2);
        assert_eq!(point.metadata.get("type"), Some(&"full".to_string()));
    }

    #[test]
    fn test_recovery_point_age() {
        let point = RecoveryPoint::new("RP", "vm", "snap");

        assert_eq!(point.age_hours(), 0);
        assert_eq!(point.age_days(), 0);
    }

    #[test]
    fn test_recovery_status_display() {
        assert_eq!(RecoveryStatus::Pending.to_string(), "Pending");
        assert_eq!(RecoveryStatus::InProgress.to_string(), "In Progress");
        assert_eq!(RecoveryStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_recovery_operation() {
        let op = RecoveryOperation::new("Restore VM", "rp-123", "vm-new", "site-2", "admin");

        assert_eq!(op.name, "Restore VM");
        assert_eq!(op.recovery_point_id, "rp-123");
        assert_eq!(op.target_resource, "vm-new");
        assert_eq!(op.status, RecoveryStatus::Pending);
        assert!(!op.success);
    }

    #[test]
    fn test_operation_lifecycle() {
        let mut op = RecoveryOperation::new("Test", "rp-1", "vm", "site", "user");

        assert_eq!(op.status, RecoveryStatus::Pending);

        op.start();
        assert_eq!(op.status, RecoveryStatus::InProgress);
        assert!(op.is_in_progress());

        op.validate();
        assert_eq!(op.status, RecoveryStatus::Validating);
        assert!(op.is_in_progress());

        op.complete(true, None);
        assert_eq!(op.status, RecoveryStatus::Completed);
        assert!(op.is_completed());
        assert!(op.success);
        assert!(op.completed_at.is_some());
        assert!(op.duration_seconds.is_some());
    }

    #[test]
    fn test_operation_failed() {
        let mut op = RecoveryOperation::new("Test", "rp-1", "vm", "site", "user");

        op.complete(false, Some("Disk space error".to_string()));

        assert_eq!(op.status, RecoveryStatus::Failed);
        assert!(!op.success);
        assert_eq!(op.error_message, Some("Disk space error".to_string()));
    }

    #[test]
    fn test_recovery_manager() {
        let mut manager = RecoveryManager::new();

        let point = RecoveryPoint::new("RP", "vm", "snap");
        let id = manager.add_recovery_point(point);

        assert_eq!(manager.recovery_point_count(), 1);
        assert!(manager.get_recovery_point(&id).is_some());
    }

    #[test]
    fn test_manager_operations() {
        let mut manager = RecoveryManager::new();

        let op = RecoveryOperation::new("Test", "rp-1", "vm", "site", "user");
        let id = manager.add_operation(op);

        assert_eq!(manager.operation_count(), 1);
        assert!(manager.get_operation(&id).is_some());
    }

    #[test]
    fn test_manager_by_resource() {
        let mut manager = RecoveryManager::new();

        manager.add_recovery_point(RecoveryPoint::new("RP1", "vm-1", "s1"));
        manager.add_recovery_point(RecoveryPoint::new("RP2", "vm-1", "s2"));
        manager.add_recovery_point(RecoveryPoint::new("RP3", "vm-2", "s3"));

        let vm1_points = manager.by_resource("vm-1");
        assert_eq!(vm1_points.len(), 2);
    }

    #[test]
    fn test_manager_consistent_points() {
        let mut manager = RecoveryManager::new();

        manager.add_recovery_point(RecoveryPoint::new("RP1", "vm1", "s1"));
        manager.add_recovery_point(RecoveryPoint::new("RP2", "vm2", "s2").with_consistency(false));
        manager.add_recovery_point(RecoveryPoint::new("RP3", "vm3", "s3"));

        let consistent = manager.consistent_points();
        assert_eq!(consistent.len(), 2);
    }

    #[test]
    fn test_manager_active_operations() {
        let mut manager = RecoveryManager::new();

        let mut op1 = RecoveryOperation::new("Op1", "rp1", "vm1", "site", "user");
        op1.start();

        let mut op2 = RecoveryOperation::new("Op2", "rp2", "vm2", "site", "user");
        op2.complete(true, None);

        manager.add_operation(op1);
        manager.add_operation(op2);

        let active = manager.active_operations();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_completed_operations() {
        let mut manager = RecoveryManager::new();

        let mut op1 = RecoveryOperation::new("Op1", "rp1", "vm1", "site", "user");
        op1.complete(true, None);

        let mut op2 = RecoveryOperation::new("Op2", "rp2", "vm2", "site", "user");
        op2.complete(false, Some("error".to_string()));

        let op3 = RecoveryOperation::new("Op3", "rp3", "vm3", "site", "user");

        manager.add_operation(op1);
        manager.add_operation(op2);
        manager.add_operation(op3);

        let completed = manager.completed_operations();
        assert_eq!(completed.len(), 2);
    }

    #[test]
    fn test_manager_old_recovery_points() {
        let mut manager = RecoveryManager::new();

        let mut old_point = RecoveryPoint::new("RP-old", "vm", "snap");
        old_point.timestamp = Utc::now() - chrono::TimeDelta::days(100);

        let recent_point = RecoveryPoint::new("RP-recent", "vm", "snap");

        manager.add_recovery_point(old_point);
        manager.add_recovery_point(recent_point);

        let old = manager.old_recovery_points(90);
        assert_eq!(old.len(), 1);
    }

    #[test]
    fn test_manager_remove_recovery_point() {
        let mut manager = RecoveryManager::new();

        let point = RecoveryPoint::new("RP", "vm", "snap");
        let id = manager.add_recovery_point(point);

        assert!(manager.remove_recovery_point(&id));
        assert_eq!(manager.recovery_point_count(), 0);
    }

    #[test]
    fn test_manager_get_operation_mut() {
        let mut manager = RecoveryManager::new();

        let op = RecoveryOperation::new("Test", "rp", "vm", "site", "user");
        let id = manager.add_operation(op);

        if let Some(op_mut) = manager.get_operation_mut(&id) {
            op_mut.start();
        }

        let op = manager.get_operation(&id).unwrap();
        assert_eq!(op.status, RecoveryStatus::InProgress);
    }

    #[test]
    fn test_recovery_status_equality() {
        assert_eq!(RecoveryStatus::Completed, RecoveryStatus::Completed);
        assert_ne!(RecoveryStatus::Completed, RecoveryStatus::Failed);
    }
}
