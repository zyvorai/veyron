// Disaster Recovery - VM restore and recovery operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Recovery plan for disaster recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub name: String,
    pub description: String,
    pub vms: Vec<VMRecoveryConfig>,
    pub recovery_order: Vec<RecoveryPhase>,
    pub rto_minutes: u32, // Recovery Time Objective
    pub rpo_minutes: u32, // Recovery Point Objective
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RecoveryPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            vms: Vec::new(),
            recovery_order: Vec::new(),
            rto_minutes: 60, // Default 1 hour
            rpo_minutes: 15, // Default 15 minutes
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_objectives(mut self, rto_minutes: u32, rpo_minutes: u32) -> Self {
        self.rto_minutes = rto_minutes;
        self.rpo_minutes = rpo_minutes;
        self
    }

    pub fn add_vm(mut self, vm_config: VMRecoveryConfig) -> Self {
        self.vms.push(vm_config);
        self
    }

    pub fn add_phase(mut self, phase: RecoveryPhase) -> Self {
        self.recovery_order.push(phase);
        self
    }

    /// Estimate total recovery time
    pub fn estimated_recovery_time(&self) -> u32 {
        self.recovery_order
            .iter()
            .map(|p| p.estimated_minutes)
            .sum()
    }
}

/// VM recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMRecoveryConfig {
    pub vm_name: String,
    pub priority: RecoveryPriority,
    pub backup_location: String,
    pub target_node: Option<String>,
    pub dependencies: Vec<String>,
}

impl VMRecoveryConfig {
    pub fn new(vm_name: impl Into<String>, priority: RecoveryPriority) -> Self {
        Self {
            vm_name: vm_name.into(),
            priority,
            backup_location: String::new(),
            target_node: None,
            dependencies: Vec::new(),
        }
    }

    pub fn with_backup(mut self, location: impl Into<String>) -> Self {
        self.backup_location = location.into();
        self
    }

    pub fn depends_on(mut self, vm: impl Into<String>) -> Self {
        self.dependencies.push(vm.into());
        self
    }
}

/// Recovery priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
}

/// Recovery phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPhase {
    pub phase_number: u32,
    pub name: String,
    pub vms: Vec<String>,
    pub estimated_minutes: u32,
}

impl RecoveryPhase {
    pub fn new(phase_number: u32, name: impl Into<String>) -> Self {
        Self {
            phase_number,
            name: name.into(),
            vms: Vec::new(),
            estimated_minutes: 0,
        }
    }

    pub fn add_vm(mut self, vm: impl Into<String>) -> Self {
        self.vms.push(vm.into());
        self
    }

    pub fn with_duration(mut self, minutes: u32) -> Self {
        self.estimated_minutes = minutes;
        self
    }
}

/// Restore operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOperation {
    pub restore_id: String,
    pub vm_name: String,
    pub backup_name: String,
    pub target_name: String,
    pub state: RestoreState,
    pub progress_percent: u8,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl RestoreOperation {
    pub fn new(
        restore_id: impl Into<String>,
        vm_name: impl Into<String>,
        backup_name: impl Into<String>,
    ) -> Self {
        let vm = vm_name.into();
        Self {
            restore_id: restore_id.into(),
            target_name: vm.clone(),
            vm_name: vm,
            backup_name: backup_name.into(),
            state: RestoreState::Pending,
            progress_percent: 0,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn to_new_vm(mut self, target: impl Into<String>) -> Self {
        self.target_name = target.into();
        self
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
        matches!(self.state, RestoreState::Completed | RestoreState::Failed)
    }
}

/// Restore state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestoreState {
    Pending,
    Downloading,
    Decompressing,
    Restoring,
    Completed,
    Failed,
}

impl std::fmt::Display for RestoreState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreState::Pending => write!(f, "Pending"),
            RestoreState::Downloading => write!(f, "Downloading"),
            RestoreState::Decompressing => write!(f, "Decompressing"),
            RestoreState::Restoring => write!(f, "Restoring"),
            RestoreState::Completed => write!(f, "Completed"),
            RestoreState::Failed => write!(f, "Failed"),
        }
    }
}

/// Point-in-time recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointInTimeRecovery {
    pub vm_name: String,
    pub target_time: DateTime<Utc>,
    pub available_backups: Vec<BackupInfo>,
}

impl PointInTimeRecovery {
    pub fn new(vm_name: impl Into<String>, target_time: DateTime<Utc>) -> Self {
        Self {
            vm_name: vm_name.into(),
            target_time,
            available_backups: Vec::new(),
        }
    }

    /// Find the absolute closest backup to the target time (including after)
    pub fn find_closest_backup(&self) -> Option<&BackupInfo> {
        self.available_backups
            .iter()
            .min_by_key(|b| {
                let diff = b.created_at.signed_duration_since(self.target_time);
                diff.num_seconds().unsigned_abs()
            })
    }

    /// Find backup immediately before target time
    pub fn find_backup_before(&self) -> Option<&BackupInfo> {
        self.available_backups
            .iter()
            .filter(|b| b.created_at <= self.target_time)
            .max_by_key(|b| b.created_at)
    }
}

/// Backup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub backup_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_recovery_plan() {
        let plan = RecoveryPlan::new("disaster-recovery")
            .with_description("Primary site recovery")
            .with_objectives(30, 5);

        assert_eq!(plan.name, "disaster-recovery");
        assert_eq!(plan.rto_minutes, 30);
        assert_eq!(plan.rpo_minutes, 5);
    }

    #[test]
    fn test_vm_recovery_config() {
        let config = VMRecoveryConfig::new("database-vm", RecoveryPriority::Critical)
            .with_backup("s3://backups/database")
            .depends_on("storage-vm");

        assert_eq!(config.vm_name, "database-vm");
        assert_eq!(config.priority, RecoveryPriority::Critical);
        assert_eq!(config.dependencies.len(), 1);
    }

    #[test]
    fn test_recovery_priority() {
        assert!(RecoveryPriority::Critical > RecoveryPriority::High);
        assert!(RecoveryPriority::High > RecoveryPriority::Normal);
        assert!(RecoveryPriority::Normal > RecoveryPriority::Low);
    }

    #[test]
    fn test_recovery_phase() {
        let phase = RecoveryPhase::new(1, "Infrastructure")
            .add_vm("network-vm")
            .add_vm("storage-vm")
            .with_duration(15);

        assert_eq!(phase.phase_number, 1);
        assert_eq!(phase.vms.len(), 2);
        assert_eq!(phase.estimated_minutes, 15);
    }

    #[test]
    fn test_estimated_recovery_time() {
        let plan = RecoveryPlan::new("test")
            .add_phase(RecoveryPhase::new(1, "Phase 1").with_duration(10))
            .add_phase(RecoveryPhase::new(2, "Phase 2").with_duration(20))
            .add_phase(RecoveryPhase::new(3, "Phase 3").with_duration(15));

        assert_eq!(plan.estimated_recovery_time(), 45);
    }

    #[test]
    fn test_restore_operation() {
        let mut restore = RestoreOperation::new("restore-001", "my-vm", "backup-2024-01-01")
            .to_new_vm("my-vm-restored");

        assert_eq!(restore.target_name, "my-vm-restored");
        assert!(!restore.is_complete());

        restore.state = RestoreState::Completed;
        restore.completed_at = Some(Utc::now());
        assert!(restore.is_complete());
    }

    #[test]
    fn test_restore_state_display() {
        assert_eq!(RestoreState::Pending.to_string(), "Pending");
        assert_eq!(RestoreState::Downloading.to_string(), "Downloading");
        assert_eq!(RestoreState::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_point_in_time_recovery() {
        let target_time = Utc::now();
        let mut pitr = PointInTimeRecovery::new("my-vm", target_time);

        pitr.available_backups.push(BackupInfo {
            name: "backup-1".to_string(),
            created_at: target_time - Duration::hours(2),
            size_bytes: 1000,
            backup_type: "full".to_string(),
        });

        pitr.available_backups.push(BackupInfo {
            name: "backup-2".to_string(),
            created_at: target_time - Duration::hours(1),
            size_bytes: 1000,
            backup_type: "full".to_string(),
        });

        pitr.available_backups.push(BackupInfo {
            name: "backup-3".to_string(),
            created_at: target_time + Duration::hours(1),
            size_bytes: 1000,
            backup_type: "full".to_string(),
        });

        let closest = pitr.find_closest_backup().unwrap();
        assert_eq!(closest.name, "backup-2");

        let before = pitr.find_backup_before().unwrap();
        assert_eq!(before.name, "backup-2");
    }
}
