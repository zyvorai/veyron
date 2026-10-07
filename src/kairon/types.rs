// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Serde models of the `kairon.zyvor.dev/v1beta1` CRDs.
//!
//! Mirrors `../kairon/internal/model/*.go`. Every spec/status carries a
//! `#[serde(flatten)] extra` map so fields Veyron does not model survive a
//! get → modify → replace round-trip instead of being silently dropped.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

pub type Extra = BTreeMap<String, JsonValue>;

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "Machine",
    plural = "machines",
    shortname = "km",
    namespaced
)]
#[kube(status = "MachineStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_type_name: Option<String>,
    #[serde(default)]
    pub image: MachineImage,
    #[serde(default)]
    pub resources: MachineResources,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_init: Option<CloudInitSpec>,
    /// `Running` | `Stopped` | `Paused` | `Halted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<PlacementSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySpec>,
    /// `volumes[0]` is the boot disk; `volumes[1..]` are virtiofs shares.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<MachineVolume>,
    /// Hot-attachable PVC-backed SCSI data disks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disks: Vec<MachineDisk>,
    /// Read-only install media (ISO) attached as SATA CD-ROMs from first boot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cdroms: Vec<MachineCdrom>,
    /// DRA `ResourceClaim` names mapped to VFIO devices (GPU, SR-IOV).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_claims: Vec<DeviceClaimReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_agent: Option<GuestAgentSpec>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineImage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ImageSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    /// A cluster-scoped `MachineImage` (kind disk); kairon-controller pins its
    /// source and digest into this spec before the Machine is scheduled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_ref: Option<String>,
    /// Empty root disk of `disk_size`, the target of an ISO install.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub blank: bool,
    /// Root disk size at creation, e.g. `60Gi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_size: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageSource {
    #[serde(rename = "httpURL", default, skip_serializing_if = "Option::is_none")]
    pub http_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oci: Option<String>,
    /// `qcow2` | `raw` | `ova` | `vmdk` | `vhd` | `vhdx`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub repair: bool,
    /// Download without verifying the server certificate; the digest still
    /// guards the bytes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub insecure_skip_tls_verify: bool,
}

/// One `spec.cdroms` entry: a `MachineImage` of kind iso, or a source + digest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineCdrom {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ImageSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

// ---------------------------------------------------------------------------
// MachineImage (cluster-scoped image catalog)
// ---------------------------------------------------------------------------

/// A named, versioned boot disk (`kind: disk`) or ISO (`kind: iso`). Generated
/// as `CatalogImage` because `MachineImage` is already `spec.image`.
#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineImage",
    root = "CatalogImage",
    plural = "machineimages",
    shortname = "mimg"
)]
#[serde(rename_all = "camelCase")]
pub struct CatalogImageSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `disk` (default) | `iso`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default)]
    pub source: ImageSource,
    #[serde(default)]
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<CatalogImageDefaults>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogImageDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_size: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineResources {
    #[serde(default)]
    pub cpu: String,
    #[serde(default)]
    pub memory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cpu: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_memory: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hugepages: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cpu_pinning: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSpec {
    /// `auto` | `qemu` | `cloud-hypervisor` | `firecracker` | `flux-vm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSpec {
    /// `user` | `tap` | `macvtap`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forwards: Vec<PortForward>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_interfaces: Vec<ExtraInterface>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cilium_attach: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub anti_spoof: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortForward {
    pub host_port: u16,
    pub guest_port: u16,
    /// `tcp` (default) | `udp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtraInterface {
    pub name: String,
    pub bridge: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ssh_authorized_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_cmd: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_files: Vec<CloudInitFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitFile {
    pub path: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlacementSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub node_selector: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<JsonValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topology_spread_constraints: Vec<JsonValue>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySpec {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secure_boot: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tpm: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineVolume {
    pub name: String,
    #[serde(default)]
    pub claim_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_path: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atlas: Option<JsonValue>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineDisk {
    pub name: String,
    pub claim_name: String,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DeviceClaimReference {
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestAgentSpec {
    /// QEMU guest agent (QGA) — needed for exec, fsfreeze, firewall.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub enabled: bool,
    /// FluxVM vsock agent console.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub console: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    #[serde(rename = "runtimeID", default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(rename = "guestIP", default, skip_serializing_if = "Option::is_none")]
    pub guest_ip: Option<String>,
    #[serde(rename = "guestIPs", default, skip_serializing_if = "Vec::is_empty")]
    pub guest_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<MachineCondition>,
    #[serde(
        rename = "appliedVCPUs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub applied_vcpus: Option<u32>,
    #[serde(
        rename = "appliedMemoryMiB",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub applied_memory_mib: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<ResourceUsage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_disks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineCondition {
    #[serde(rename = "type")]
    pub type_: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    #[serde(default)]
    pub cpu_percent: f64,
    #[serde(default)]
    pub memory_bytes: u64,
    #[serde(default)]
    pub disk_read_bytes: u64,
    #[serde(default)]
    pub disk_write_bytes: u64,
}

// ---------------------------------------------------------------------------
// MachineMigration
// ---------------------------------------------------------------------------

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineMigration",
    plural = "machinemigrations",
    namespaced
)]
#[kube(status = "MachineMigrationStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineMigrationSpec {
    pub machine_name: String,
    /// `auto` | `live` | `cold`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<String>,
    /// `pre-copy` | `post-copy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_downtime_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cancel: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineMigrationStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_strategy: Option<String>,
    #[serde(
        rename = "ramTransferred",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ram_transferred: Option<u64>,
    #[serde(rename = "ramTotal", default, skip_serializing_if = "Option::is_none")]
    pub ram_total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_time_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downtime_ms: Option<u64>,
    #[serde(flatten)]
    pub extra: Extra,
}

// ---------------------------------------------------------------------------
// Snapshots / restores / schedules
// ---------------------------------------------------------------------------

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineSnapshot",
    plural = "machinesnapshots",
    namespaced
)]
#[kube(status = "MachineSnapshotStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshotSpec {
    pub machine_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_snapshot_class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_names: Vec<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshotStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub ready_to_use: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_snapshots: Vec<VolumeSnapshotReference>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSnapshotReference {
    pub volume_name: String,
    #[serde(default)]
    pub volume_snapshot_name: String,
    #[serde(default)]
    pub ready_to_use: bool,
    #[serde(
        rename = "atlasSnapshotID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub atlas_snapshot_id: Option<String>,
}

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineSnapshotRestore",
    plural = "machinesnapshotrestores",
    namespaced
)]
#[kube(status = "MachineSnapshotRestoreStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshotRestoreSpec {
    pub snapshot_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_name: Option<String>,
    pub target_claim_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshotRestoreStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_claim_name: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineSnapshotSchedule",
    plural = "machinesnapshotschedules",
    namespaced
)]
#[kube(status = "GenericStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshotScheduleSpec {
    #[serde(default)]
    pub selector: BTreeMap<String, String>,
    pub interval_seconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_snapshot_class_name: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suspend: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_last: Option<i64>,
    #[serde(flatten)]
    pub extra: Extra,
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineBackup",
    plural = "machinebackups",
    namespaced
)]
#[kube(status = "GenericStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineBackupSpec {
    pub machine_name: String,
    /// `""` (best effort) | `required` | `none`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiesce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atlas: Option<JsonValue>,
}

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineBackupRestore",
    plural = "machinebackuprestores",
    namespaced
)]
#[kube(status = "GenericStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineBackupRestoreSpec {
    pub backup_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,
}

/// Status shape shared by Kairon CRDs whose status Veyron only reports.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenericStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

// ---------------------------------------------------------------------------
// Catalog / fleet
// ---------------------------------------------------------------------------

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineInstanceType",
    plural = "machineinstancetypes",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct MachineInstanceTypeSpec {
    pub resources: MachineResources,
}

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachineSet",
    plural = "machinesets",
    namespaced
)]
#[kube(status = "GenericStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachineSetSpec {
    pub replicas: i32,
    pub template: MachineTemplate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kairon.zyvor.dev",
    version = "v1beta1",
    kind = "MachinePool",
    plural = "machinepools",
    namespaced
)]
#[kube(status = "GenericStatus")]
#[serde(rename_all = "camelCase")]
pub struct MachinePoolSpec {
    pub replicas: i32,
    pub template: MachineTemplate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineTemplate {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    pub spec: MachineSpec,
}

macro_rules! opaque_kairon_crd {
    ($spec:ident, $kind:literal, $plural:literal) => {
        #[derive(CustomResource, Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
        #[kube(group = "kairon.zyvor.dev", version = "v1beta1", kind = $kind, plural = $plural, namespaced)]
        #[kube(status = "GenericStatus")]
        pub struct $spec {
            #[serde(flatten)]
            pub fields: Extra,
        }
    };
}

opaque_kairon_crd!(MachineClaimSpec, "MachineClaim", "machineclaims");
opaque_kairon_crd!(MachineQuotaSpec, "MachineQuota", "machinequotas");
opaque_kairon_crd!(
    MachineDisruptionBudgetSpec,
    "MachineDisruptionBudget",
    "machinedisruptionbudgets"
);
opaque_kairon_crd!(MigrationPolicySpec, "MigrationPolicy", "migrationpolicies");
opaque_kairon_crd!(
    MachineNetworkPolicySpec,
    "MachineNetworkPolicy",
    "machinenetworkpolicies"
);
opaque_kairon_crd!(
    NetworkSecurityGroupSpec,
    "NetworkSecurityGroup",
    "networksecuritygroups"
);

/// Every Kairon CRD plural, for RBAC/capability probes.
pub const ALL_PLURALS: &[&str] = &[
    "machines",
    "machinemigrations",
    "machinesnapshots",
    "machinesnapshotrestores",
    "machinesnapshotschedules",
    "machinebackups",
    "machinebackuprestores",
    "machineinstancetypes",
    "machinesets",
    "machinepools",
    "machineclaims",
    "machinequotas",
    "machinedisruptionbudgets",
    "migrationpolicies",
    "machinenetworkpolicies",
    "networksecuritygroups",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_status_maps_kairon_field_names() {
        let json = serde_json::json!({
            "apiVersion": "kairon.zyvor.dev/v1beta1",
            "kind": "Machine",
            "metadata": {"name": "web-1", "namespace": "default"},
            "spec": {
                "image": {"source": {"httpURL": "https://img/x.qcow2", "format": "qcow2"}, "digest": "sha256:ab"},
                "resources": {"cpu": "2", "memory": "4Gi"},
                "powerState": "Running",
                "futureField": {"kept": true}
            },
            "status": {
                "phase": "Running",
                "nodeName": "n1",
                "runtimeID": "rt-1",
                "guestIP": "10.0.0.5",
                "guestIPs": ["10.0.0.5"],
                "appliedVCPUs": 2,
                "appliedMemoryMiB": 4096,
                "resourceUsage": {"cpuPercent": 12.5, "memoryBytes": 1024, "diskReadBytes": 1, "diskWriteBytes": 2},
                "message": ""
            }
        });
        let m: Machine = serde_json::from_value(json).unwrap();
        let st = m.status.as_ref().unwrap();
        assert_eq!(st.runtime_id.as_deref(), Some("rt-1"));
        assert_eq!(st.guest_ip.as_deref(), Some("10.0.0.5"));
        assert_eq!(st.applied_vcpus, Some(2));
        assert_eq!(
            m.spec.image.source.as_ref().unwrap().http_url.as_deref(),
            Some("https://img/x.qcow2")
        );
        let back = serde_json::to_value(&m).unwrap();
        assert_eq!(back["spec"]["futureField"]["kept"], true);
        assert_eq!(
            back["spec"]["image"]["source"]["httpURL"],
            "https://img/x.qcow2"
        );
        assert_eq!(back["status"]["runtimeID"], "rt-1");
    }
}
