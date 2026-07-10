// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// VM Snapshots & Backup System - Production-grade snapshot management
// This is a critical feature for disaster recovery and VM lifecycle management

pub mod crds;
pub mod manager;
pub mod restore;
pub mod retention;
pub mod scheduler_lease;
pub mod types;
pub mod vm_snapshot_schedule;

pub use crds::{VirtualMachineRestore, VirtualMachineSnapshot};
pub use manager::SnapshotManager;
pub use restore::RestoreManager;
pub use retention::RetentionEnforcer;
pub use types::{RestoreInfo, RestoreStatus, SnapshotInfo, SnapshotStatus};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub max_snapshots: Option<u32>,
    pub max_age_days: Option<u32>,
    pub keep_last_n: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_snapshots: Some(10),
            max_age_days: Some(30),
            keep_last_n: Some(5),
        }
    }
}

/// Snapshot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub vm_name: String,
    pub snapshot_name: String,
    pub description: Option<String>,
    pub labels: HashMap<String, String>,
    pub retention: RetentionPolicy,
    /// Request an application-consistent (quiesced) snapshot. For a running VM
    /// with the guest agent connected, KubeVirt's snapshot controller freezes
    /// the guest filesystems around the online snapshot; this flag records the
    /// intent and tags the snapshot's consistency level.
    pub app_consistent: bool,
}

impl SnapshotConfig {
    pub fn new(vm_name: impl Into<String>, snapshot_name: impl Into<String>) -> Self {
        Self {
            vm_name: vm_name.into(),
            snapshot_name: snapshot_name.into(),
            description: None,
            labels: HashMap::new(),
            retention: RetentionPolicy::default(),
            app_consistent: false,
        }
    }

    /// Request an application-consistent (guest-quiesced) snapshot.
    pub fn with_app_consistent(mut self, app_consistent: bool) -> Self {
        self.app_consistent = app_consistent;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_config() {
        let config = SnapshotConfig::new("my-vm", "snap-1")
            .with_description("Test snapshot")
            .with_label("env", "production");

        assert_eq!(config.vm_name, "my-vm");
        assert_eq!(config.snapshot_name, "snap-1");
        assert_eq!(config.description, Some("Test snapshot".to_string()));
        assert_eq!(config.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_default_retention_policy() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.max_snapshots, Some(10));
        assert_eq!(policy.max_age_days, Some(30));
        assert_eq!(policy.keep_last_n, Some(5));
    }
}
