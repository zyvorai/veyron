// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

/// VirtualMachine custom resource for KubeVirt
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kubevirt.io",
    version = "v1",
    kind = "VirtualMachine",
    plural = "virtualmachines",
    shortname = "vm",
    shortname = "vms",
    namespaced
)]
#[kube(status = "VirtualMachineStatus")]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineSpec {
    pub running: Option<bool>,
    pub run_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instancetype: Option<InstancetypeMatcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preference: Option<InstancetypeMatcher>,
    /// Per-VM DataVolumes, created and garbage-collected with the VM. This is how a
    /// VM gets its own disk cloned from a golden image, rather than referencing a
    /// shared PVC by name (which would have two VMs writing the same disk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_volume_templates: Option<Vec<DataVolumeTemplate>>,
    pub template: VirtualMachineInstanceTemplateSpec,
}

/// A CDI DataVolume owned by the VM (`spec.dataVolumeTemplates[]`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "metadata_schema")]
    pub metadata: Option<ObjectMeta>,
    pub spec: DataVolumeSpec,
}

/// CDI `DataVolume.spec`. Either `source` (inline) or `sourceRef` (a DataSource
/// pointer) is set — `sourceRef` is preferred for a golden-image catalog because
/// the DataSource can be repointed at a new image version without touching VMs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DataVolumeSourceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<DataVolumeSourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<DataVolumeStorage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSourceSpec {
    /// Clone from an existing PVC (the golden disk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pvc: Option<DataVolumeSourcePVC>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<DataVolumeSourceHTTP>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<DataVolumeSourceRegistry>,
    /// An empty disk of the requested size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blank: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSourcePVC {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSourceHTTP {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSourceRegistry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Points at a CDI `DataSource` — the stable catalog handle for a golden image.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSourceRef {
    /// Always `DataSource` today.
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_modes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<DataVolumeResources>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<BTreeMap<String, String>>,
}

/// KubeVirt `VirtualMachineSpec.instancetype` / `preference` reference.
///
/// `name` OR `inferFromVolume` — the latter tells KubeVirt to read the matcher from a
/// label on the named volume's PVC/DataSource (how a VM auto-sizes off a golden image).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstancetypeMatcher {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infer_from_volume: Option<String>,
    /// `Reject` (default in KubeVirt) or `Ignore` when the label is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infer_from_volume_failure_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceTemplateSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "metadata_schema")]
    pub metadata: Option<ObjectMeta>,
    pub spec: VirtualMachineInstanceSpec,
}

fn metadata_schema(_generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::Schema::Bool(true)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_config: Option<JsonValue>,
    pub domain: DomainSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<Network>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_grace_period_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eviction_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affinity: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerations: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_spread_constraints: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_credentials: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainSpec {
    pub resources: ResourceRequirements,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CPU>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<Memory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Devices>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystems: Option<Vec<Filesystem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Features>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<Clock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<Firmware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_threads_policy: Option<String>,
}

/// Virtio-FS filesystem definition (`spec.domain.filesystems`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Filesystem {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtiofs: Option<VirtiofsSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct VirtiofsSource {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CPU {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_cpu_placement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolate_emulator_thread: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa: Option<NUMA>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realtime: Option<Realtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<Hugepages>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_guest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct Devices {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disks: Option<Vec<Disk>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<Interface>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tpm: Option<TPMDevice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rng: Option<RNGDevice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<InputDevice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_devices: Option<Vec<HostDevice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpus: Option<Vec<Gpu>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<WatchdogDevice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoattach_graphics_device: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoattach_mem_balloon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_interface_multiqueue: Option<bool>,
    /// KubeVirt `devices.video` — API may return a single object or an array of devices.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_devices_video"
    )]
    pub video: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<Channel>>,
}

/// Virtio-serial channel (e.g. QEMU guest agent at `org.qemu.guest_agent.0`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub name: String,
    pub target: ChannelTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChannelTarget {
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: String,
}

fn deserialize_devices_video<'de, D>(deserializer: D) -> Result<Option<JsonValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<JsonValue>::deserialize(deserializer)?;
    Ok(v.filter(|j| !j.is_null()))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<DiskTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lun: Option<LUNTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom: Option<CDROMTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_order: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_io_thread: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiskTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CDROMTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LUNTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Interface {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masquerade: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sriov: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<InterfacePort>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_order: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_disk: Option<ContainerDiskSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_volume: Option<DataVolumeSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_init_no_cloud: Option<CloudInitNoCloudSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_init_config_drive: Option<CloudInitConfigDriveSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_disk: Option<EmptyDiskSource>,
    /// Windows unattended-setup answer file, surfaced to the guest as a CD-ROM.
    /// KubeVirt reads `autounattend.xml` from the referenced Secret/ConfigMap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysprep: Option<SysprepSource>,
}

/// KubeVirt `sysprep` volume — mounts a Windows `autounattend.xml` as install media.
/// Exactly one of `secret` / `config_map` is set; a Secret is correct whenever the
/// answer file carries credentials (it almost always does).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SysprepSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<LocalObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<LocalObjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalObjectRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDiskSource {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimVolumeSource {
    pub claim_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSource {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitNoCloudSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitConfigDriveSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data_secret_ref: Option<UserDataSecretRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDataSecretRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmptyDiskSource {
    pub capacity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multus: Option<MultusNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultusNetwork {
    pub network_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InterfacePort {
    pub port: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

// ============================================================================
// Features
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acpi: Option<FeatureEnabled>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apic: Option<FeatureEnabled>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperv: Option<HyperVFeatures>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm: Option<KVMFeatures>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smm: Option<SMMFeatures>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeatureEnabled {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeatureState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HyperVFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relaxed: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vapic: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinlocks: Option<SpinlockRetries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpindex: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synic: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stimer: Option<STimer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequencies: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reenlightenment: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlbflush: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipi: Option<FeatureState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evmcs: Option<FeatureState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpinlockRetries {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinlocks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct STimer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct: Option<FeatureState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KVMFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SMMFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

// ============================================================================
// Clock
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc: Option<UTCClock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timer: Option<ClockTimer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UTCClock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClockTimer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hpet: Option<TimerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit: Option<TimerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc: Option<RTCTimer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperv: Option<TimerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm: Option<TimerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RTCTimer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
}

// ============================================================================
// Firmware
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootloader: Option<Bootloader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Bootloader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios: Option<BIOSBootloader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efi: Option<EFIBootloader>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BIOSBootloader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_serial: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EFIBootloader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_boot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
}

// ============================================================================
// Machine
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,
}

// ============================================================================
// CPU extras
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NUMA {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest_mapping_passthrough: Option<NUMAGuestMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NUMAGuestMapping {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Realtime {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
}

// ============================================================================
// Memory extras
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Hugepages {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<String>,
}

// ============================================================================
// Device types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TPMDevice {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RNGDevice {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    #[serde(rename = "type")]
    pub input_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WatchdogDevice {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// GPU assignment (`spec.domain.devices.gpus`) — whole-GPU passthrough or a
/// vGPU mediated device. `deviceName` is the device-plugin resource name.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub device_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_gpu_options: Option<VirtualGpuOptions>,
}

/// KubeVirt `virtualGPUOptions` (vGPU display head / RAM framebuffer).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct VirtualGpuOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<VgpuDisplayOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct VgpuDisplayOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "ramFB", skip_serializing_if = "Option::is_none")]
    pub ram_fb: Option<VgpuRamFb>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct VgpuRamFb {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// GPU / mediated device assignment (`spec.domain.devices.hostDevices`).
/// KubeVirt's schema calls the device-plugin resource `deviceName`; the
/// `resourceName` alias keeps previously exported Veyron JSON deserializable.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostDevice {
    pub name: String,
    #[serde(rename = "deviceName", alias = "resourceName")]
    pub resource_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printable_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type")]
    pub type_: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<String>,
}

/// VirtualMachineInstance - represents a running VM instance in KubeVirt
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "kubevirt.io",
    version = "v1",
    kind = "VirtualMachineInstance",
    plural = "virtualmachineinstances",
    shortname = "vmi",
    shortname = "vmis",
    namespaced
)]
#[kube(status = "VirtualMachineInstanceStatus")]
#[serde(rename_all = "camelCase")]
pub struct VmiSpec {
    // VMI spec mirrors the template spec - we mainly care about status
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiCondition {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<VmiInterface>,
    #[serde(
        rename = "guestOSInfo",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub guest_os_info: Option<GuestOsInfo>,
    #[serde(default)]
    pub conditions: Vec<VmiCondition>,
    /// Every other status field (volumeStatus, migrationState,
    /// qemuGuestAgentVersionInfo, …) that isn't modeled above. Without this,
    /// any such field is silently dropped whenever a VMI round-trips through
    /// this typed struct — reproduced live: GET .../volumes/status always
    /// returned `[]` despite `.status.volumeStatus` genuinely holding a
    /// hotplugged volume's real AttachedToNode state. Mirrors the same
    /// flatten pattern already used for `VmiSpec` above.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiInterface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub ip_addresses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestOsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_release: Option<String>,
}

/// Typed spec for KubeVirt cluster-scoped `MigrationPolicy`
/// (`migrations.kubevirt.io/v1alpha1`). Used to validate API payloads before
/// the untyped passthrough create/replace, and as the target of
/// [`crate::migration::MigrationPolicy::to_kubevirt_policy`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPolicySpec {
    pub selectors: MigrationPolicySelectors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_auto_converge: Option<bool>,
    /// Quantity string, e.g. `"100Mi"` per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_per_migration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_timeout_per_gib: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_post_copy: Option<bool>,
}

/// A MigrationPolicy must select *something* — namespaces and/or VMIs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPolicySelectors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_selector: Option<BTreeMap<String, String>>,
    #[serde(
        rename = "virtualMachineInstanceSelector",
        skip_serializing_if = "Option::is_none"
    )]
    pub vmi_selector: Option<BTreeMap<String, String>>,
}

/// VirtualMachineInstanceMigration CRD for KubeVirt
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[kube(
    group = "kubevirt.io",
    version = "v1",
    kind = "VirtualMachineInstanceMigration",
    plural = "virtualmachineinstancemigrations",
    shortname = "vmim",
    namespaced
)]
#[kube(status = "VirtualMachineInstanceMigrationStatus")]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceMigrationSpec {
    /// Name of the VMI to migrate
    pub vmi_name: Option<String>,
    /// Restrict migration targets (e.g. `kubernetes.io/hostname: worker-2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_node_selector: Option<BTreeMap<String, String>>,
    /// `system-critical`, `user-triggered`, or `system-maintenance`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceMigrationStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_state: Option<MigrationStateInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStateInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_address: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmi_status_deserialization() {
        let json = serde_json::json!({
            "phase": "Running",
            "nodeName": "worker-1",
            "interfaces": [
                {
                    "name": "default",
                    "ipAddress": "10.244.1.5",
                    "ipAddresses": ["10.244.1.5", "fd00::5"],
                    "mac": "52:54:00:12:34:56",
                    "interfaceName": "eth0"
                }
            ],
            "guestOSInfo": {
                "name": "Fedora Linux",
                "id": "fedora",
                "version": "39",
                "kernelRelease": "6.5.0-0.rc7"
            }
        });

        let status: VirtualMachineInstanceStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status.phase.as_deref(), Some("Running"));
        assert_eq!(status.node_name.as_deref(), Some("worker-1"));
        assert_eq!(status.interfaces.len(), 1);
        assert_eq!(
            status.interfaces[0].ip_address.as_deref(),
            Some("10.244.1.5")
        );
        assert_eq!(status.interfaces[0].ip_addresses.len(), 2);
        assert_eq!(
            status.interfaces[0].mac.as_deref(),
            Some("52:54:00:12:34:56")
        );
        assert_eq!(status.interfaces[0].interface_name.as_deref(), Some("eth0"));
        let guest_os = status.guest_os_info.as_ref().unwrap();
        assert_eq!(guest_os.name.as_deref(), Some("Fedora Linux"));
        assert_eq!(guest_os.id.as_deref(), Some("fedora"));
        assert_eq!(guest_os.version.as_deref(), Some("39"));
    }

    #[test]
    fn test_vmi_status_deserialization_minimal() {
        let json = serde_json::json!({
            "phase": "Scheduling",
            "interfaces": []
        });

        let status: VirtualMachineInstanceStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status.phase.as_deref(), Some("Scheduling"));
        assert!(status.node_name.is_none());
        assert!(status.interfaces.is_empty());
        assert!(status.guest_os_info.is_none());
    }

    #[test]
    fn test_vmi_status_serialization_roundtrip() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Running".to_string()),
            node_name: Some("node-1".to_string()),
            interfaces: vec![VmiInterface {
                name: Some("default".to_string()),
                ip_address: Some("192.168.1.10".to_string()),
                ip_addresses: vec!["192.168.1.10".to_string(), "fd00::a".to_string()],
                mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
                interface_name: Some("eth0".to_string()),
            }],
            guest_os_info: Some(GuestOsInfo {
                name: Some("Ubuntu".to_string()),
                id: Some("ubuntu".to_string()),
                version: Some("22.04".to_string()),
                kernel_release: Some("5.15.0".to_string()),
            }),
            conditions: vec![],
            extra: Default::default(),
        };

        let json = serde_json::to_value(&status).unwrap();
        let deserialized: VirtualMachineInstanceStatus = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.phase, status.phase);
        assert_eq!(deserialized.node_name, status.node_name);
        assert_eq!(deserialized.interfaces.len(), 1);
        assert_eq!(
            deserialized.interfaces[0].ip_address,
            status.interfaces[0].ip_address
        );
    }

    /// Regression: a real KubeVirt VMI status carries fields (volumeStatus,
    /// migrationState, …) that VirtualMachineInstanceStatus doesn't model
    /// explicitly. Without #[serde(flatten)] extra, these are silently
    /// dropped on deserialize — reproduced live via GET .../volumes/status
    /// always returning [] despite a real hotplugged volume's status
    /// existing on the cluster.
    #[test]
    fn test_vmi_status_preserves_unmodeled_fields() {
        let json = serde_json::json!({
            "phase": "Running",
            "volumeStatus": [
                {
                    "name": "hotvol1",
                    "target": "",
                    "phase": "AttachedToNode",
                    "hotplugVolume": {"attachPodName": "hp-volume-abc123"}
                }
            ],
            "migrationState": {"completed": true}
        });
        let status: VirtualMachineInstanceStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status.phase.as_deref(), Some("Running"));

        let round_tripped = serde_json::to_value(&status).unwrap();
        let volume_status = round_tripped
            .get("volumeStatus")
            .and_then(|v| v.as_array())
            .expect("volumeStatus must survive the round-trip");
        assert_eq!(volume_status.len(), 1);
        assert_eq!(
            volume_status[0].get("phase").and_then(|p| p.as_str()),
            Some("AttachedToNode")
        );
        assert!(round_tripped.get("migrationState").is_some());
    }

    /// Regression: KubeVirt returns `devices.video` as a single object, not an array.
    #[test]
    fn test_vm_deserialize_video_object_not_array() {
        let json = serde_json::json!({
            "apiVersion": "kubevirt.io/v1",
            "kind": "VirtualMachine",
            "metadata": { "name": "zeus-ubuntu-desktop-01", "namespace": "ragnarok-vms" },
            "spec": {
                "runStrategy": "Always",
                "template": {
                    "spec": {
                        "architecture": "amd64",
                        "domain": {
                            "devices": {
                                "disks": [{ "disk": { "bus": "virtio" }, "name": "rootdisk" }],
                                "interfaces": [{ "masquerade": {}, "model": "virtio", "name": "default" }],
                                "rng": {},
                                "video": { "type": "virtio" }
                            },
                            "resources": { "requests": { "memory": "4Gi" } }
                        },
                        "networks": [{ "name": "default", "pod": {} }],
                        "volumes": [{ "containerDisk": { "image": "quay.io/containerdisks/ubuntu:24.04" }, "name": "rootdisk" }]
                    }
                }
            },
            "status": { "printableStatus": "Running", "ready": true }
        });
        let vm: VirtualMachine = serde_json::from_value(json).unwrap();
        let video = vm
            .spec
            .template
            .spec
            .domain
            .devices
            .as_ref()
            .and_then(|d| d.video.as_ref())
            .expect("video");
        assert_eq!(video.get("type").and_then(|v| v.as_str()), Some("virtio"));
    }

    #[test]
    fn test_vmi_interface_empty_fields() {
        let json = serde_json::json!({});

        let iface: VmiInterface = serde_json::from_value(json).unwrap();
        assert!(iface.name.is_none());
        assert!(iface.ip_address.is_none());
        assert!(iface.ip_addresses.is_empty());
        assert!(iface.mac.is_none());
        assert!(iface.interface_name.is_none());
    }

    #[test]
    fn test_vmi_interface_multiple_ips() {
        let json = serde_json::json!({
            "name": "net1",
            "ipAddress": "10.0.0.1",
            "ipAddresses": ["10.0.0.1", "10.0.0.2", "fd00::1"],
            "mac": "52:54:00:ab:cd:ef",
            "interfaceName": "net1"
        });

        let iface: VmiInterface = serde_json::from_value(json).unwrap();
        assert_eq!(iface.ip_addresses.len(), 3);
        assert_eq!(iface.ip_addresses[0], "10.0.0.1");
        assert_eq!(iface.ip_addresses[2], "fd00::1");
    }

    #[test]
    fn test_guest_os_info_partial() {
        let json = serde_json::json!({
            "name": "CentOS Stream",
            "id": "centos"
        });

        let info: GuestOsInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.name.as_deref(), Some("CentOS Stream"));
        assert_eq!(info.id.as_deref(), Some("centos"));
        assert!(info.version.is_none());
        assert!(info.kernel_release.is_none());
    }

    #[test]
    fn test_vmi_status_skips_none_on_serialize() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Running".to_string()),
            node_name: None,
            interfaces: vec![],
            guest_os_info: None,
            conditions: vec![],
            extra: Default::default(),
        };

        let json = serde_json::to_value(&status).unwrap();
        assert!(json.get("phase").is_some());
        assert!(json.get("nodeName").is_none());
        assert!(json.get("guestOSInfo").is_none());
    }

    /// Helper to extract the first IP from a VirtualMachineInstanceStatus,
    /// mirroring the logic in KubeClient::get_vm_ip.
    fn extract_first_ip(status: &VirtualMachineInstanceStatus) -> Option<String> {
        for iface in &status.interfaces {
            if let Some(ip) = &iface.ip_address {
                if !ip.is_empty() {
                    return Some(ip.clone());
                }
            }
        }
        None
    }

    #[test]
    fn test_extract_ip_from_interfaces() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Running".to_string()),
            node_name: None,
            interfaces: vec![
                VmiInterface {
                    name: Some("default".to_string()),
                    ip_address: Some("10.244.1.5".to_string()),
                    ip_addresses: vec!["10.244.1.5".to_string()],
                    mac: None,
                    interface_name: None,
                },
                VmiInterface {
                    name: Some("net1".to_string()),
                    ip_address: Some("192.168.1.100".to_string()),
                    ip_addresses: vec!["192.168.1.100".to_string()],
                    mac: None,
                    interface_name: None,
                },
            ],
            guest_os_info: None,
            conditions: vec![],
            extra: Default::default(),
        };

        // Should return the first interface's IP
        assert_eq!(extract_first_ip(&status), Some("10.244.1.5".to_string()));
    }

    #[test]
    fn test_extract_ip_skips_empty() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Running".to_string()),
            node_name: None,
            interfaces: vec![
                VmiInterface {
                    name: Some("default".to_string()),
                    ip_address: Some(String::new()),
                    ip_addresses: vec![],
                    mac: None,
                    interface_name: None,
                },
                VmiInterface {
                    name: Some("net1".to_string()),
                    ip_address: Some("10.0.0.1".to_string()),
                    ip_addresses: vec!["10.0.0.1".to_string()],
                    mac: None,
                    interface_name: None,
                },
            ],
            guest_os_info: None,
            conditions: vec![],
            extra: Default::default(),
        };

        // Should skip empty string and return the second IP
        assert_eq!(extract_first_ip(&status), Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_ip_no_interfaces() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Scheduling".to_string()),
            node_name: None,
            interfaces: vec![],
            guest_os_info: None,
            conditions: vec![],
            extra: Default::default(),
        };

        assert_eq!(extract_first_ip(&status), None);
    }

    #[test]
    fn test_extract_ip_none_ip_address() {
        let status = VirtualMachineInstanceStatus {
            phase: Some("Running".to_string()),
            node_name: None,
            interfaces: vec![VmiInterface {
                name: Some("default".to_string()),
                ip_address: None,
                ip_addresses: vec![],
                mac: None,
                interface_name: None,
            }],
            guest_os_info: None,
            conditions: vec![],
            extra: Default::default(),
        };

        assert_eq!(extract_first_ip(&status), None);
    }
}
