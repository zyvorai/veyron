// KubeVirt Snapshot CRD Types
// Defines VirtualMachineSnapshot and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Snapshot status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotStatus {
    InProgress,
    Succeeded,
    Failed,
    Unknown,
}

impl std::fmt::Display for SnapshotStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotStatus::InProgress => write!(f, "InProgress"),
            SnapshotStatus::Succeeded => write!(f, "Succeeded"),
            SnapshotStatus::Failed => write!(f, "Failed"),
            SnapshotStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Snapshot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub vm_name: String,
    pub namespace: String,
    pub status: SnapshotStatus,
    pub created_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub size: Option<String>,
    pub labels: HashMap<String, String>,
    pub ready_to_use: bool,
    pub error: Option<String>,
}

impl SnapshotInfo {
    pub fn new(
        name: impl Into<String>,
        vm_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            vm_name: vm_name.into(),
            namespace: namespace.into(),
            status: SnapshotStatus::Unknown,
            created_at: None,
            completed_at: None,
            description: None,
            size: None,
            labels: HashMap::new(),
            ready_to_use: false,
            error: None,
        }
    }

    /// Get snapshot age in a human-readable format
    pub fn age(&self) -> String {
        if let Some(created) = self.created_at {
            let duration = Utc::now().signed_duration_since(created);
            let days = duration.num_days();
            let hours = duration.num_hours() % 24;
            let minutes = duration.num_minutes() % 60;

            if days > 0 {
                format!("{}d{}h", days, hours)
            } else if hours > 0 {
                format!("{}h{}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Get duration to completion
    pub fn duration(&self) -> Option<String> {
        if let (Some(created), Some(completed)) = (self.created_at, self.completed_at) {
            let duration = completed.signed_duration_since(created);
            let seconds = duration.num_seconds();

            if seconds < 60 {
                Some(format!("{}s", seconds))
            } else if seconds < 3600 {
                Some(format!("{}m{}s", seconds / 60, seconds % 60))
            } else {
                Some(format!("{}h{}m", seconds / 3600, (seconds % 3600) / 60))
            }
        } else {
            None
        }
    }
}

/// Restore information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreInfo {
    pub name: String,
    pub snapshot_name: String,
    pub target_vm_name: String,
    pub namespace: String,
    pub status: RestoreStatus,
    pub created_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Restore status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestoreStatus {
    InProgress,
    Succeeded,
    Failed,
    Unknown,
}

impl std::fmt::Display for RestoreStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreStatus::InProgress => write!(f, "InProgress"),
            RestoreStatus::Succeeded => write!(f, "Succeeded"),
            RestoreStatus::Failed => write!(f, "Failed"),
            RestoreStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

impl RestoreInfo {
    pub fn new(
        name: impl Into<String>,
        snapshot_name: impl Into<String>,
        target_vm_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            snapshot_name: snapshot_name.into(),
            target_vm_name: target_vm_name.into(),
            namespace: namespace.into(),
            status: RestoreStatus::Unknown,
            created_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Get restore duration
    pub fn duration(&self) -> Option<String> {
        if let (Some(created), Some(completed)) = (self.created_at, self.completed_at) {
            let duration = completed.signed_duration_since(created);
            let seconds = duration.num_seconds();

            if seconds < 60 {
                Some(format!("{}s", seconds))
            } else {
                Some(format!("{}m{}s", seconds / 60, seconds % 60))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_info_creation() {
        let info = SnapshotInfo::new("snap-1", "my-vm", "default");
        assert_eq!(info.name, "snap-1");
        assert_eq!(info.vm_name, "my-vm");
        assert_eq!(info.namespace, "default");
        assert_eq!(info.status, SnapshotStatus::Unknown);
        assert!(!info.ready_to_use);
    }

    #[test]
    fn test_restore_info_creation() {
        let info = RestoreInfo::new("restore-1", "snap-1", "restored-vm", "default");
        assert_eq!(info.name, "restore-1");
        assert_eq!(info.snapshot_name, "snap-1");
        assert_eq!(info.target_vm_name, "restored-vm");
        assert_eq!(info.status, RestoreStatus::Unknown);
    }

    #[test]
    fn test_snapshot_status_display() {
        assert_eq!(format!("{}", SnapshotStatus::InProgress), "InProgress");
        assert_eq!(format!("{}", SnapshotStatus::Succeeded), "Succeeded");
        assert_eq!(format!("{}", SnapshotStatus::Failed), "Failed");
    }
}
