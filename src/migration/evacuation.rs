// Node Evacuation - Drain nodes for maintenance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Node evacuation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvacuationRequest {
    pub node_name: String,
    pub reason: String,
    pub strategy: EvacuationStrategy,
    pub timeout_seconds: u64,
    pub force: bool,
    pub requested_at: DateTime<Utc>,
}

impl EvacuationRequest {
    pub fn new(node_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            node_name: node_name.into(),
            reason: reason.into(),
            strategy: EvacuationStrategy::default(),
            timeout_seconds: 3600, // 1 hour default
            force: false,
            requested_at: Utc::now(),
        }
    }

    pub fn with_strategy(mut self, strategy: EvacuationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn force_evict(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Evacuation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvacuationStrategy {
    pub live_migrate: bool,
    pub max_parallel: u32,
    pub retry_failed: bool,
    pub delete_local_data: bool,
}

impl Default for EvacuationStrategy {
    fn default() -> Self {
        Self {
            live_migrate: true,
            max_parallel: 2,
            retry_failed: true,
            delete_local_data: false,
        }
    }
}

/// Evacuation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvacuationStatus {
    pub node_name: String,
    pub state: EvacuationState,
    pub total_vms: usize,
    pub migrated_vms: usize,
    pub failed_vms: usize,
    pub in_progress_vms: usize,
    pub vm_statuses: Vec<VMEvacuationStatus>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl EvacuationStatus {
    pub fn new(node_name: impl Into<String>, total_vms: usize) -> Self {
        Self {
            node_name: node_name.into(),
            state: EvacuationState::Pending,
            total_vms,
            migrated_vms: 0,
            failed_vms: 0,
            in_progress_vms: 0,
            vm_statuses: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn progress_percent(&self) -> u8 {
        if self.total_vms == 0 {
            return 100;
        }
        (((self.migrated_vms + self.failed_vms) as f64 / self.total_vms as f64) * 100.0) as u8
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            EvacuationState::Completed | EvacuationState::Failed
        )
    }

    pub fn duration_secs(&self) -> i64 {
        match self.completed_at {
            Some(completed) => completed
                .signed_duration_since(self.started_at)
                .num_seconds(),
            None => Utc::now()
                .signed_duration_since(self.started_at)
                .num_seconds(),
        }
    }

    pub fn add_vm_status(&mut self, status: VMEvacuationStatus) {
        match status.state {
            VMEvacuationState::InProgress => {
                self.in_progress_vms += 1;
            }
            VMEvacuationState::Migrated => {
                self.migrated_vms += 1;
                self.in_progress_vms = self.in_progress_vms.saturating_sub(1);
            }
            VMEvacuationState::Failed => {
                self.failed_vms += 1;
                self.in_progress_vms = self.in_progress_vms.saturating_sub(1);
            }
            _ => {}
        }
        self.vm_statuses.push(status);
    }
}

/// Evacuation state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvacuationState {
    Pending,
    Cordoning,
    Draining,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for EvacuationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvacuationState::Pending => write!(f, "Pending"),
            EvacuationState::Cordoning => write!(f, "Cordoning"),
            EvacuationState::Draining => write!(f, "Draining"),
            EvacuationState::InProgress => write!(f, "In Progress"),
            EvacuationState::Completed => write!(f, "Completed"),
            EvacuationState::Failed => write!(f, "Failed"),
            EvacuationState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// VM evacuation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMEvacuationStatus {
    pub vm_name: String,
    pub state: VMEvacuationState,
    pub target_node: Option<String>,
    pub error_message: Option<String>,
}

impl VMEvacuationStatus {
    pub fn new(vm_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            state: VMEvacuationState::Pending,
            target_node: None,
            error_message: None,
        }
    }

    pub fn migrating_to(mut self, node: impl Into<String>) -> Self {
        self.target_node = Some(node.into());
        self.state = VMEvacuationState::InProgress;
        self
    }

    pub fn migrated(mut self) -> Self {
        self.state = VMEvacuationState::Migrated;
        self
    }

    pub fn failed(mut self, error: impl Into<String>) -> Self {
        self.state = VMEvacuationState::Failed;
        self.error_message = Some(error.into());
        self
    }
}

/// VM evacuation state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VMEvacuationState {
    Pending,
    InProgress,
    Migrated,
    Failed,
    Skipped,
}

/// Evacuation planner
pub struct EvacuationPlanner {
    max_parallel: u32,
}

impl EvacuationPlanner {
    pub fn new(max_parallel: u32) -> Self {
        if max_parallel == 0 {
            log::warn!("max_parallel cannot be 0, defaulting to 1");
        }
        Self { max_parallel: max_parallel.max(1) }
    }

    /// Plan evacuation order (by priority)
    pub fn plan_evacuation(&self, vms: Vec<(String, u8)>) -> Vec<Vec<String>> {
        // Sort VMs by priority (higher first)
        let mut sorted_vms = vms;
        sorted_vms.sort_by(|a, b| b.1.cmp(&a.1));

        // Group into batches for parallel migration
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();

        for (vm_name, _priority) in sorted_vms {
            current_batch.push(vm_name);

            if current_batch.len() >= self.max_parallel as usize {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        batches
    }

    /// Calculate estimated evacuation time
    pub fn estimate_duration(&self, vm_count: usize, avg_migration_time_secs: u64) -> u64 {
        let batches = (vm_count as f64 / self.max_parallel as f64).ceil() as u64;
        batches * avg_migration_time_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evacuation_request() {
        let request = EvacuationRequest::new("node1", "Maintenance")
            .with_timeout(7200)
            .force_evict();

        assert_eq!(request.node_name, "node1");
        assert_eq!(request.reason, "Maintenance");
        assert_eq!(request.timeout_seconds, 7200);
        assert!(request.force);
    }

    #[test]
    fn test_evacuation_strategy() {
        let strategy = EvacuationStrategy::default();
        assert!(strategy.live_migrate);
        assert_eq!(strategy.max_parallel, 2);
        assert!(strategy.retry_failed);
    }

    #[test]
    fn test_evacuation_status() {
        let mut status = EvacuationStatus::new("node1", 10);
        assert_eq!(status.progress_percent(), 0);

        status.add_vm_status(VMEvacuationStatus::new("vm1").migrated());
        status.add_vm_status(VMEvacuationStatus::new("vm2").migrated());

        assert_eq!(status.migrated_vms, 2);
        assert_eq!(status.progress_percent(), 20);
    }

    #[test]
    fn test_vm_evacuation_status() {
        let status = VMEvacuationStatus::new("test-vm").migrating_to("node2");

        assert_eq!(status.vm_name, "test-vm");
        assert_eq!(status.state, VMEvacuationState::InProgress);
        assert_eq!(status.target_node, Some("node2".to_string()));

        let failed_status = VMEvacuationStatus::new("failed-vm").failed("No suitable nodes");

        assert_eq!(failed_status.state, VMEvacuationState::Failed);
        assert!(failed_status.error_message.is_some());
    }

    #[test]
    fn test_evacuation_planner() {
        let planner = EvacuationPlanner::new(2);

        let vms = vec![
            ("vm1".to_string(), 10),
            ("vm2".to_string(), 20),
            ("vm3".to_string(), 15),
            ("vm4".to_string(), 30),
            ("vm5".to_string(), 5),
        ];

        let batches = planner.plan_evacuation(vms);

        // Should create 3 batches (2+2+1) sorted by priority
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[2].len(), 1);

        // First batch should have highest priority VMs
        assert_eq!(batches[0][0], "vm4"); // priority 30
        assert_eq!(batches[0][1], "vm2"); // priority 20
    }

    #[test]
    fn test_estimate_duration() {
        let planner = EvacuationPlanner::new(2);

        // 10 VMs, 2 parallel, 120 seconds per migration
        // = 5 batches * 120 seconds = 600 seconds
        let duration = planner.estimate_duration(10, 120);
        assert_eq!(duration, 600);

        // 5 VMs, 2 parallel, 120 seconds
        // = 3 batches * 120 seconds = 360 seconds
        let duration = planner.estimate_duration(5, 120);
        assert_eq!(duration, 360);
    }

    #[test]
    fn test_evacuation_state_display() {
        assert_eq!(EvacuationState::Pending.to_string(), "Pending");
        assert_eq!(EvacuationState::Draining.to_string(), "Draining");
        assert_eq!(EvacuationState::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_evacuation_complete() {
        let mut status = EvacuationStatus::new("node1", 2);
        assert!(!status.is_complete());

        status.state = EvacuationState::Completed;
        status.completed_at = Some(Utc::now());
        assert!(status.is_complete());
    }
}
