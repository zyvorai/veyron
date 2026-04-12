// Reconciliation - Drift detection and automatic reconciliation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Drift status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftStatus {
    InSync,
    Drifted,
    Unknown,
}

/// Resource drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDrift {
    pub resource_name: String,
    pub kind: String,
    pub namespace: String,
    pub status: DriftStatus,
    pub differences: Vec<DriftDifference>,
    pub detected_at: DateTime<Utc>,
    pub git_revision: String,
    pub cluster_revision: String,
}

impl ResourceDrift {
    pub fn new(
        resource_name: impl Into<String>,
        kind: impl Into<String>,
        namespace: impl Into<String>,
        git_revision: impl Into<String>,
        cluster_revision: impl Into<String>,
    ) -> Self {
        Self {
            resource_name: resource_name.into(),
            kind: kind.into(),
            namespace: namespace.into(),
            status: DriftStatus::InSync,
            differences: Vec::new(),
            detected_at: Utc::now(),
            git_revision: git_revision.into(),
            cluster_revision: cluster_revision.into(),
        }
    }

    pub fn add_difference(&mut self, diff: DriftDifference) {
        self.differences.push(diff);
        self.status = DriftStatus::Drifted;
    }

    pub fn is_drifted(&self) -> bool {
        matches!(self.status, DriftStatus::Drifted)
    }

    pub fn difference_count(&self) -> usize {
        self.differences.len()
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}/{}", self.kind, self.namespace, self.resource_name)
    }
}

/// Drift difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDifference {
    pub field_path: String,
    pub git_value: Option<String>,
    pub cluster_value: Option<String>,
    pub diff_type: DriftType,
}

impl DriftDifference {
    pub fn added(field_path: impl Into<String>, cluster_value: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            git_value: None,
            cluster_value: Some(cluster_value.into()),
            diff_type: DriftType::Added,
        }
    }

    pub fn removed(field_path: impl Into<String>, git_value: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            git_value: Some(git_value.into()),
            cluster_value: None,
            diff_type: DriftType::Removed,
        }
    }

    pub fn modified(
        field_path: impl Into<String>,
        git_value: impl Into<String>,
        cluster_value: impl Into<String>,
    ) -> Self {
        Self {
            field_path: field_path.into(),
            git_value: Some(git_value.into()),
            cluster_value: Some(cluster_value.into()),
            diff_type: DriftType::Modified,
        }
    }
}

/// Drift type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftType {
    Added,    // Field exists in cluster but not in Git
    Removed,  // Field exists in Git but not in cluster
    Modified, // Field value differs between Git and cluster
}

/// Reconciliation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconciliationAction {
    Update {
        resource_name: String,
    },
    Recreate {
        resource_name: String,
    },
    Delete {
        resource_name: String,
    },
    Skip {
        resource_name: String,
        reason: String,
    },
}

/// Reconciliation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationPlan {
    pub actions: Vec<ReconciliationAction>,
    pub total_resources: usize,
    pub resources_to_update: usize,
    pub resources_to_recreate: usize,
    pub resources_to_delete: usize,
    pub created_at: DateTime<Utc>,
}

impl ReconciliationPlan {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            total_resources: 0,
            resources_to_update: 0,
            resources_to_recreate: 0,
            resources_to_delete: 0,
            created_at: Utc::now(),
        }
    }

    pub fn add_update(&mut self, resource_name: impl Into<String>) {
        self.actions.push(ReconciliationAction::Update {
            resource_name: resource_name.into(),
        });
        self.resources_to_update += 1;
        self.total_resources += 1;
    }

    pub fn add_recreate(&mut self, resource_name: impl Into<String>) {
        self.actions.push(ReconciliationAction::Recreate {
            resource_name: resource_name.into(),
        });
        self.resources_to_recreate += 1;
        self.total_resources += 1;
    }

    pub fn add_delete(&mut self, resource_name: impl Into<String>) {
        self.actions.push(ReconciliationAction::Delete {
            resource_name: resource_name.into(),
        });
        self.resources_to_delete += 1;
        self.total_resources += 1;
    }

    pub fn add_skip(&mut self, resource_name: impl Into<String>, reason: impl Into<String>) {
        self.actions.push(ReconciliationAction::Skip {
            resource_name: resource_name.into(),
            reason: reason.into(),
        });
    }

    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }
}

impl Default for ReconciliationPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Reconciliation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub success: bool,
    pub resources_reconciled: usize,
    pub resources_failed: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub errors: Vec<String>,
}

impl ReconciliationResult {
    pub fn new() -> Self {
        Self {
            success: false,
            resources_reconciled: 0,
            resources_failed: 0,
            started_at: Utc::now(),
            completed_at: None,
            errors: Vec::new(),
        }
    }

    pub fn add_success(&mut self) {
        self.resources_reconciled += 1;
    }

    pub fn add_failure(&mut self, error: impl Into<String>) {
        self.resources_failed += 1;
        self.errors.push(error.into());
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.success = self.resources_failed == 0;
    }

    pub fn duration_seconds(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.started_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.started_at)
                .num_seconds(),
        }
    }
}

impl Default for ReconciliationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Drift detector
pub struct DriftDetector {
    detected_drifts: Vec<ResourceDrift>,
}

impl DriftDetector {
    pub fn new() -> Self {
        Self {
            detected_drifts: Vec::new(),
        }
    }

    pub fn detect_drift(&mut self, drift: ResourceDrift) {
        self.detected_drifts.push(drift);
    }

    pub fn get_drifts(&self) -> Vec<&ResourceDrift> {
        self.detected_drifts.iter().collect()
    }

    pub fn get_drifted_resources(&self) -> Vec<&ResourceDrift> {
        self.detected_drifts
            .iter()
            .filter(|d| d.is_drifted())
            .collect()
    }

    pub fn drift_count(&self) -> usize {
        self.detected_drifts.len()
    }

    pub fn drifted_count(&self) -> usize {
        self.get_drifted_resources().len()
    }

    pub fn clear(&mut self) {
        self.detected_drifts.clear();
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Reconciler
pub struct Reconciler {
    auto_reconcile: bool,
    reconciliation_history: Vec<ReconciliationResult>,
}

impl Reconciler {
    pub fn new(auto_reconcile: bool) -> Self {
        Self {
            auto_reconcile,
            reconciliation_history: Vec::new(),
        }
    }

    pub fn is_auto_reconcile(&self) -> bool {
        self.auto_reconcile
    }

    pub fn create_plan(&self, drifts: &[&ResourceDrift]) -> ReconciliationPlan {
        let mut plan = ReconciliationPlan::new();

        for drift in drifts {
            if drift.is_drifted() {
                plan.add_update(&drift.resource_name);
            }
        }

        plan
    }

    pub fn execute_plan(&mut self, plan: &ReconciliationPlan) -> ReconciliationResult {
        let mut result = ReconciliationResult::new();

        for action in &plan.actions {
            match action {
                ReconciliationAction::Update { resource_name: _ } => {
                    // Simulated reconciliation
                    result.add_success();
                }
                ReconciliationAction::Recreate { resource_name: _ } => {
                    result.add_success();
                }
                ReconciliationAction::Delete { resource_name: _ } => {
                    result.add_success();
                }
                ReconciliationAction::Skip { .. } => {
                    // Skipped actions don't count
                }
            }
        }

        result.complete();
        self.reconciliation_history.push(result.clone());
        result
    }

    pub fn get_history(&self) -> &[ReconciliationResult] {
        &self.reconciliation_history
    }

    pub fn history_count(&self) -> usize {
        self.reconciliation_history.len()
    }

    pub fn last_result(&self) -> Option<&ReconciliationResult> {
        self.reconciliation_history.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_status() {
        assert_eq!(DriftStatus::InSync, DriftStatus::InSync);
        assert_ne!(DriftStatus::InSync, DriftStatus::Drifted);
    }

    #[test]
    fn test_resource_drift() {
        let drift = ResourceDrift::new("test-vm", "VirtualMachine", "default", "abc123", "def456");

        assert_eq!(drift.resource_name, "test-vm");
        assert_eq!(drift.status, DriftStatus::InSync);
        assert!(!drift.is_drifted());
    }

    #[test]
    fn test_drift_add_difference() {
        let mut drift = ResourceDrift::new("test-vm", "VM", "default", "abc", "def");

        let diff = DriftDifference::modified("spec.replicas", "2", "3");
        drift.add_difference(diff);

        assert!(drift.is_drifted());
        assert_eq!(drift.difference_count(), 1);
    }

    #[test]
    fn test_drift_difference_types() {
        let added = DriftDifference::added("spec.newField", "value");
        assert_eq!(added.diff_type, DriftType::Added);
        assert!(added.git_value.is_none());
        assert!(added.cluster_value.is_some());

        let removed = DriftDifference::removed("spec.oldField", "value");
        assert_eq!(removed.diff_type, DriftType::Removed);

        let modified = DriftDifference::modified("spec.field", "old", "new");
        assert_eq!(modified.diff_type, DriftType::Modified);
        assert_eq!(modified.git_value, Some("old".to_string()));
        assert_eq!(modified.cluster_value, Some("new".to_string()));
    }

    #[test]
    fn test_reconciliation_plan() {
        let mut plan = ReconciliationPlan::new();

        plan.add_update("vm-1");
        plan.add_recreate("vm-2");
        plan.add_delete("vm-3");

        assert!(plan.has_actions());
        assert_eq!(plan.action_count(), 3);
        assert_eq!(plan.resources_to_update, 1);
        assert_eq!(plan.resources_to_recreate, 1);
        assert_eq!(plan.resources_to_delete, 1);
    }

    #[test]
    fn test_plan_skip_action() {
        let mut plan = ReconciliationPlan::new();

        plan.add_skip("vm-1", "Manual intervention required");
        plan.add_update("vm-2");

        assert_eq!(plan.action_count(), 2);
        assert_eq!(plan.total_resources, 1); // Skip doesn't count
    }

    #[test]
    fn test_reconciliation_result() {
        let mut result = ReconciliationResult::new();

        result.add_success();
        result.add_success();
        result.add_failure("Error syncing vm-3");

        assert_eq!(result.resources_reconciled, 2);
        assert_eq!(result.resources_failed, 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_result_completion() {
        let mut result = ReconciliationResult::new();

        result.add_success();
        result.complete();

        assert!(result.success);
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_result_failure() {
        let mut result = ReconciliationResult::new();

        result.add_failure("Error");
        result.complete();

        assert!(!result.success);
    }

    #[test]
    fn test_result_duration() {
        let result = ReconciliationResult::new();
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(result.duration_seconds() >= 0);
    }

    #[test]
    fn test_drift_detector() {
        let mut detector = DriftDetector::new();

        let drift = ResourceDrift::new("test-vm", "VM", "default", "abc", "def");
        detector.detect_drift(drift);

        assert_eq!(detector.drift_count(), 1);
    }

    #[test]
    fn test_detector_drifted_resources() {
        let mut detector = DriftDetector::new();

        let mut drifted = ResourceDrift::new("vm-1", "VM", "default", "abc", "def");
        drifted.add_difference(DriftDifference::modified("field", "old", "new"));

        let in_sync = ResourceDrift::new("vm-2", "VM", "default", "abc", "abc");

        detector.detect_drift(drifted);
        detector.detect_drift(in_sync);

        assert_eq!(detector.drifted_count(), 1);
    }

    #[test]
    fn test_detector_clear() {
        let mut detector = DriftDetector::new();

        detector.detect_drift(ResourceDrift::new("vm-1", "VM", "default", "abc", "def"));
        assert_eq!(detector.drift_count(), 1);

        detector.clear();
        assert_eq!(detector.drift_count(), 0);
    }

    #[test]
    fn test_reconciler() {
        let reconciler = Reconciler::new(true);

        assert!(reconciler.is_auto_reconcile());
        assert_eq!(reconciler.history_count(), 0);
    }

    #[test]
    fn test_reconciler_create_plan() {
        let reconciler = Reconciler::new(false);

        let mut drift = ResourceDrift::new("vm-1", "VM", "default", "abc", "def");
        drift.add_difference(DriftDifference::modified("spec.replicas", "2", "3"));

        let drifts = vec![&drift];
        let plan = reconciler.create_plan(&drifts);

        assert!(plan.has_actions());
        assert_eq!(plan.resources_to_update, 1);
    }

    #[test]
    fn test_reconciler_execute_plan() {
        let mut reconciler = Reconciler::new(true);

        let mut plan = ReconciliationPlan::new();
        plan.add_update("vm-1");
        plan.add_update("vm-2");

        let result = reconciler.execute_plan(&plan);

        assert!(result.success);
        assert_eq!(result.resources_reconciled, 2);
        assert_eq!(reconciler.history_count(), 1);
    }

    #[test]
    fn test_reconciler_history() {
        let mut reconciler = Reconciler::new(true);

        let mut plan1 = ReconciliationPlan::new();
        plan1.add_update("vm-1");

        let mut plan2 = ReconciliationPlan::new();
        plan2.add_update("vm-2");

        reconciler.execute_plan(&plan1);
        reconciler.execute_plan(&plan2);

        assert_eq!(reconciler.history_count(), 2);

        let last = reconciler.last_result();
        assert!(last.is_some());
    }

    #[test]
    fn test_drift_type() {
        assert_eq!(DriftType::Added, DriftType::Added);
        assert_ne!(DriftType::Added, DriftType::Modified);
    }

    #[test]
    fn test_resource_drift_full_name() {
        let drift = ResourceDrift::new("test-vm", "VirtualMachine", "production", "abc", "def");

        assert_eq!(drift.full_name(), "VirtualMachine/production/test-vm");
    }
}
