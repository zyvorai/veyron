// VM Migration - Live migration and high availability

pub mod assistant;
pub mod batch;
pub mod dryrun;
pub mod history;
pub mod rollback;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Migration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    pub vm_name: String,
    pub source_node: String,
    pub target_node: Option<String>,
    pub migration_type: MigrationType,
    pub policy: MigrationPolicy,
    pub requested_at: DateTime<Utc>,
}

impl MigrationRequest {
    pub fn new(vm_name: impl Into<String>, source_node: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            source_node: source_node.into(),
            target_node: None,
            migration_type: MigrationType::Live,
            policy: MigrationPolicy::default(),
            requested_at: Utc::now(),
        }
    }

    pub fn to_node(mut self, node: impl Into<String>) -> Self {
        self.target_node = Some(node.into());
        self
    }

    pub fn with_type(mut self, migration_type: MigrationType) -> Self {
        self.migration_type = migration_type;
        self
    }

    pub fn with_policy(mut self, policy: MigrationPolicy) -> Self {
        self.policy = policy;
        self
    }
}

/// Migration type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationType {
    Live,     // Live migration (no downtime)
    Offline,  // VM must be stopped
    PostCopy, // Post-copy live migration
}

impl MigrationType {
    pub fn as_str(&self) -> &str {
        match self {
            MigrationType::Live => "live",
            MigrationType::Offline => "offline",
            MigrationType::PostCopy => "post-copy",
        }
    }
}

/// Migration policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPolicy {
    pub allow_auto_converge: bool,
    pub allow_post_copy: bool,
    pub bandwidth_limit: Option<String>, // e.g., "100Mi"
    pub completion_timeout: u64,         // seconds
    pub parallelism: u32,
}

impl Default for MigrationPolicy {
    fn default() -> Self {
        Self {
            allow_auto_converge: true,
            allow_post_copy: false,
            bandwidth_limit: None,
            completion_timeout: 800,
            parallelism: 2,
        }
    }
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub vm_name: String,
    pub state: MigrationState,
    pub source_node: String,
    pub target_node: String,
    pub progress_percent: u8,
    pub phase: MigrationPhase,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl MigrationStatus {
    pub fn new(
        vm_name: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            vm_name: vm_name.into(),
            state: MigrationState::Pending,
            source_node: source.into(),
            target_node: target.into(),
            progress_percent: 0,
            phase: MigrationPhase::Preparing,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
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

    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            MigrationState::Succeeded | MigrationState::Failed
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, MigrationState::Running)
    }
}

/// Migration state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationState {
    Pending,
    Scheduling,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl std::fmt::Display for MigrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationState::Pending => write!(f, "Pending"),
            MigrationState::Scheduling => write!(f, "Scheduling"),
            MigrationState::Running => write!(f, "Running"),
            MigrationState::Succeeded => write!(f, "Succeeded"),
            MigrationState::Failed => write!(f, "Failed"),
            MigrationState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Migration phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationPhase {
    Preparing,
    PreparingTarget,
    MemoryTransfer,
    SyncingDisk,
    Completing,
    Succeeded,
    Failed,
}

impl std::fmt::Display for MigrationPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationPhase::Preparing => write!(f, "Preparing"),
            MigrationPhase::PreparingTarget => write!(f, "Preparing Target"),
            MigrationPhase::MemoryTransfer => write!(f, "Transferring Memory"),
            MigrationPhase::SyncingDisk => write!(f, "Syncing Disks"),
            MigrationPhase::Completing => write!(f, "Completing"),
            MigrationPhase::Succeeded => write!(f, "Succeeded"),
            MigrationPhase::Failed => write!(f, "Failed"),
        }
    }
}

pub mod evacuation;
pub mod ha;
pub mod strategy;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_request() {
        let request = MigrationRequest::new("my-vm", "node1")
            .to_node("node2")
            .with_type(MigrationType::Live);

        assert_eq!(request.vm_name, "my-vm");
        assert_eq!(request.source_node, "node1");
        assert_eq!(request.target_node, Some("node2".to_string()));
        assert_eq!(request.migration_type, MigrationType::Live);
    }

    #[test]
    fn test_migration_policy_default() {
        let policy = MigrationPolicy::default();
        assert!(policy.allow_auto_converge);
        assert!(!policy.allow_post_copy);
        assert_eq!(policy.completion_timeout, 800);
    }

    #[test]
    fn test_migration_status() {
        let mut status = MigrationStatus::new("test-vm", "node1", "node2");
        assert_eq!(status.state, MigrationState::Pending);
        assert!(!status.is_complete());

        status.state = MigrationState::Running;
        assert!(status.is_running());

        status.state = MigrationState::Succeeded;
        status.completed_at = Some(Utc::now());
        assert!(status.is_complete());
    }

    #[test]
    fn test_migration_type() {
        assert_eq!(MigrationType::Live.as_str(), "live");
        assert_eq!(MigrationType::Offline.as_str(), "offline");
        assert_eq!(MigrationType::PostCopy.as_str(), "post-copy");
    }

    #[test]
    fn test_migration_state_display() {
        assert_eq!(MigrationState::Pending.to_string(), "Pending");
        assert_eq!(MigrationState::Running.to_string(), "Running");
        assert_eq!(MigrationState::Succeeded.to_string(), "Succeeded");
    }

    #[test]
    fn test_migration_phase_display() {
        assert_eq!(MigrationPhase::Preparing.to_string(), "Preparing");
        assert_eq!(
            MigrationPhase::MemoryTransfer.to_string(),
            "Transferring Memory"
        );
        assert_eq!(MigrationPhase::Succeeded.to_string(), "Succeeded");
    }
}
