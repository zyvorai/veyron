// KubeVirt Snapshot CRD Definitions
// VirtualMachineSnapshot and VirtualMachineRestore custom resources

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// VirtualMachineSnapshot CRD
/// Represents a snapshot of a KubeVirt virtual machine
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "snapshot.kubevirt.io",
    version = "v1alpha1",
    kind = "VirtualMachineSnapshot",
    namespaced
)]
#[kube(status = "VirtualMachineSnapshotStatus")]
pub struct VirtualMachineSnapshotSpec {
    /// Source VM to snapshot
    pub source: SnapshotSource,

    /// Deletion policy for snapshot content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_policy: Option<String>,

    /// Failure deadline in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_deadline: Option<i64>,
}

/// Source specification for snapshot
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SnapshotSource {
    /// API group of the source
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "apiGroup")]
    pub api_group: Option<String>,

    /// Kind of the source (usually VirtualMachine)
    pub kind: String,

    /// Name of the VM to snapshot
    pub name: String,
}

/// Status of a VirtualMachineSnapshot
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VirtualMachineSnapshotStatus {
    /// Indicates if snapshot is ready to use
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "readyToUse")]
    pub ready_to_use: Option<bool>,

    /// Creation time of the snapshot (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "creationTime")]
    pub creation_time: Option<String>,

    /// Current phase of the snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,

    /// Error information if snapshot failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SnapshotError>,

    /// Name of the VirtualMachineSnapshotContent
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "virtualMachineSnapshotContentName")]
    pub content_name: Option<String>,

    /// Indications of snapshot progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indications: Option<Vec<String>>,

    /// Conditions of the snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<SnapshotCondition>>,
}

/// Snapshot error information
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SnapshotError {
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Error time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/// Snapshot condition
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SnapshotCondition {
    /// Type of condition
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status of the condition
    pub status: String,

    /// Last probe time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastProbeTime")]
    pub last_probe_time: Option<String>,

    /// Last transition time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastTransitionTime")]
    pub last_transition_time: Option<String>,

    /// Reason for the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// VirtualMachineRestore CRD
/// Represents a restore operation from a snapshot
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "snapshot.kubevirt.io",
    version = "v1alpha1",
    kind = "VirtualMachineRestore",
    namespaced
)]
#[kube(status = "VirtualMachineRestoreStatus")]
pub struct VirtualMachineRestoreSpec {
    /// Target VM to restore to
    pub target: RestoreTarget,

    /// Name of the VirtualMachineSnapshot to restore from
    #[serde(rename = "virtualMachineSnapshotName")]
    pub snapshot_name: String,

    /// Patches to apply to the restored VM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patches: Option<Vec<String>>,
}

/// Target specification for restore
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RestoreTarget {
    /// API group of the target
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "apiGroup")]
    pub api_group: Option<String>,

    /// Kind of the target (usually VirtualMachine)
    pub kind: String,

    /// Name of the target VM
    pub name: String,
}

/// Status of a VirtualMachineRestore
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VirtualMachineRestoreStatus {
    /// Indicates if restore is complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete: Option<bool>,

    /// Restore start time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "restoreTime")]
    pub restore_time: Option<String>,

    /// Deleted data volumes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "deletedDataVolumes")]
    pub deleted_data_volumes: Option<Vec<String>>,

    /// Restored data volumes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restores: Option<Vec<DataVolumeRestore>>,

    /// Error information if restore failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RestoreError>,

    /// Conditions of the restore
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<RestoreCondition>>,
}

/// Data volume restore information
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct DataVolumeRestore {
    /// Name of the data volume
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dataVolumeName")]
    pub data_volume_name: Option<String>,

    /// PersistentVolumeClaim name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "persistentVolumeClaim")]
    pub pvc_name: Option<String>,

    /// VolumeSnapshot name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "volumeSnapshotName")]
    pub volume_snapshot_name: Option<String>,
}

/// Restore error information
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RestoreError {
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Error time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/// Restore condition
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct RestoreCondition {
    /// Type of condition
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status of the condition
    pub status: String,

    /// Last probe time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastProbeTime")]
    pub last_probe_time: Option<String>,

    /// Last transition time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastTransitionTime")]
    pub last_transition_time: Option<String>,

    /// Reason for the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_source_creation() {
        let source = SnapshotSource {
            api_group: Some("kubevirt.io".to_string()),
            kind: "VirtualMachine".to_string(),
            name: "test-vm".to_string(),
        };

        assert_eq!(source.kind, "VirtualMachine");
        assert_eq!(source.name, "test-vm");
    }

    #[test]
    fn test_snapshot_spec_creation() {
        let spec = VirtualMachineSnapshotSpec {
            source: SnapshotSource {
                api_group: Some("kubevirt.io".to_string()),
                kind: "VirtualMachine".to_string(),
                name: "my-vm".to_string(),
            },
            deletion_policy: Some("Delete".to_string()),
            failure_deadline: Some(3600),
        };

        assert_eq!(spec.source.name, "my-vm");
        assert_eq!(spec.deletion_policy, Some("Delete".to_string()));
    }

    #[test]
    fn test_restore_target_creation() {
        let target = RestoreTarget {
            api_group: Some("kubevirt.io".to_string()),
            kind: "VirtualMachine".to_string(),
            name: "restored-vm".to_string(),
        };

        assert_eq!(target.kind, "VirtualMachine");
        assert_eq!(target.name, "restored-vm");
    }
}
