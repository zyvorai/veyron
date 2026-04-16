// VMRogue Operator CRD Definitions
//
// Rust-side mirror of the Go operator CRDs (vmrogue.io/v1alpha1).
// These types allow the Rust API to create/read/update/delete VMRogue CRDs
// via kube-rs, enabling the Rust API server and CLI to interact with the
// Go operator's custom resources.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// VMRogueVM CRD
// =============================================================================

/// VMRogueVM is a higher-level VM abstraction that the Go operator
/// translates into KubeVirt VirtualMachine resources.
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "vmrogue.io",
    version = "v1alpha1",
    kind = "VMRogueVM",
    namespaced
)]
#[kube(status = "VMRogueVMStatus")]
#[kube(shortname = "vrvm")]
pub struct VMRogueVMSpec {
    /// Template name from VMRogue template library.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// Resource profile name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// CPU configuration.
    pub cpu: CRDCPUSpec,

    /// Memory configuration.
    pub memory: CRDMemorySpec,

    /// Disk definitions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disks: Vec<CRDDiskSpec>,

    /// Network interfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<CRDInterfaceSpec>,

    /// Cloud-init configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cloudInit")]
    pub cloud_init: Option<CRDCloudInitSpec>,

    /// VM features.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<CRDFeaturesSpec>,

    /// Firmware configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<CRDFirmwareSpec>,

    /// Clock configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<CRDClockSpec>,

    /// Eviction strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "evictionStrategy")]
    pub eviction_strategy: Option<String>,

    /// Termination grace period in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "terminationGracePeriod")]
    pub termination_grace_period: Option<i64>,

    /// Enable TPM 2.0.
    #[serde(default)]
    #[serde(rename = "enableTpm")]
    pub enable_tpm: bool,

    /// Enable virtio-rng.
    #[serde(default)]
    #[serde(rename = "enableRng")]
    pub enable_rng: bool,

    /// Machine type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "machineType")]
    pub machine_type: Option<String>,

    /// Running state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,

    /// Labels for the KubeVirt VM.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,

    /// Annotations for the KubeVirt VM.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDCPUSpec {
    pub cores: u32,
    #[serde(default = "default_one")]
    pub sockets: u32,
    #[serde(default = "default_one")]
    pub threads: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dedicatedCpuPlacement")]
    pub dedicated_cpu_placement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isolateEmulatorThread")]
    pub isolate_emulator_thread: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDMemorySpec {
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hugepagesPageSize")]
    pub hugepages_page_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxGuest")]
    pub max_guest: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDDiskSpec {
    pub name: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "storageClass")]
    pub storage_class: Option<String>,
    #[serde(default)]
    #[serde(rename = "bootOrder")]
    pub boot_order: u32,
    pub source: CRDDiskSource,
    #[serde(default = "default_disk")]
    #[serde(rename = "deviceType")]
    pub device_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDDiskSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDInterfaceSpec {
    pub name: String,
    pub network: String,
    #[serde(default = "default_virtio")]
    pub model: String,
    #[serde(rename = "networkType")]
    pub network_type: CRDNetworkType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "macAddress")]
    pub mac_address: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDNetworkType {
    #[serde(rename = "type")]
    pub net_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDCloudInitSpec {
    #[serde(rename = "userData")]
    pub user_data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "networkData")]
    pub network_data: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDFeaturesSpec {
    #[serde(default = "default_true")]
    pub acpi: bool,
    #[serde(default)]
    pub apic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperv: Option<CRDHyperVSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "kvmHidden")]
    pub kvm_hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smm: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDHyperVSpec {
    #[serde(default)]
    pub relaxed: bool,
    #[serde(default)]
    pub vapic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinlocks: Option<u32>,
    #[serde(default)]
    pub vpindex: bool,
    #[serde(default)]
    pub runtime: bool,
    #[serde(default)]
    pub synic: bool,
    #[serde(default)]
    pub stimer: bool,
    #[serde(default)]
    pub reset: bool,
    #[serde(default)]
    pub frequencies: bool,
    #[serde(default)]
    pub reenlightenment: bool,
    #[serde(default)]
    pub tlbflush: bool,
    #[serde(default)]
    pub ipi: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDFirmwareSpec {
    pub bootloader: String,
    #[serde(default)]
    #[serde(rename = "secureBoot")]
    pub secure_boot: bool,
    #[serde(default)]
    pub persistent: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDClockSpec {
    #[serde(default = "default_true")]
    pub utc: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timers: Option<CRDTimersSpec>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDTimersSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hpetPresent")]
    pub hpet_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "pitTickPolicy")]
    pub pit_tick_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "rtcTickPolicy")]
    pub rtc_tick_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hypervPresent")]
    pub hyperv_present: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VMRogueVMStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "kubevirtVMName")]
    pub kubevirt_vm_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nodeName")]
    pub node_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ipAddress")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "observedGeneration")]
    pub observed_generation: Option<i64>,
}

// =============================================================================
// VMRogueBlueprint CRD
// =============================================================================

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "vmrogue.io",
    version = "v1alpha1",
    kind = "VMRogueBlueprint",
    namespaced
)]
#[kube(status = "VMRogueBlueprintStatus")]
#[kube(shortname = "vrbp")]
pub struct VMRogueBlueprintSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub vms: Vec<CRDBlueprintVMSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDBlueprintVMSpec {
    pub name: String,
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "diskSize")]
    pub disk_size: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VMRogueBlueprintStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "totalVMs")]
    pub total_vms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "readyVMs")]
    pub ready_vms: Option<i32>,
}

// =============================================================================
// VMRoguePolicy CRD
// =============================================================================

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "vmrogue.io",
    version = "v1alpha1",
    kind = "VMRoguePolicy",
    namespaced
)]
#[kube(status = "VMRoguePolicyStatus")]
#[kube(shortname = "vrpol")]
pub struct VMRoguePolicySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(rename = "enforcementAction")]
    pub enforcement_action: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    /// Label selector to match VMRogueVMs this policy applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<std::collections::BTreeMap<String, String>>,
    pub rules: Vec<CRDPolicyRule>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct CRDPolicyRule {
    pub name: String,
    /// Condition identifier, e.g. "max_cpu_cores", "require_cloud_init".
    pub condition: String,
    /// Human-readable message shown on violation.
    pub message: String,
    /// Numeric threshold for comparison-based conditions (e.g. max_cpu_cores: 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VMRoguePolicyStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "matchingVMs")]
    pub matching_vms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "compliantVMs")]
    pub compliant_vms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "violatingVMs")]
    pub violating_vms: Option<i32>,
}

// =============================================================================
// VMRogueInsight CRD
// =============================================================================

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "vmrogue.io",
    version = "v1alpha1",
    kind = "VMRogueInsight",
    namespaced
)]
#[kube(status = "VMRogueInsightStatus")]
#[kube(shortname = "vrin")]
pub struct VMRogueInsightSpec {
    #[serde(rename = "insightType")]
    pub insight_type: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "vmRef")]
    pub vm_ref: Option<String>,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub data: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VMRogueInsightStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "actionRef")]
    pub action_ref: Option<String>,
}

// =============================================================================
// VMRogueAction CRD
// =============================================================================

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "vmrogue.io",
    version = "v1alpha1",
    kind = "VMRogueAction",
    namespaced
)]
#[kube(status = "VMRogueActionStatus")]
#[kube(shortname = "vract")]
pub struct VMRogueActionSpec {
    #[serde(rename = "actionType")]
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "vmRef")]
    pub vm_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "insightRef")]
    pub insight_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub approved: bool,
    #[serde(default)]
    #[serde(rename = "autoApprove")]
    pub auto_approve: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct VMRogueActionStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// Conversion: VMConfig <-> VMRogueVMSpec
// =============================================================================

impl From<&crate::config::VMConfig> for VMRogueVMSpec {
    fn from(config: &crate::config::VMConfig) -> Self {
        let disks = config
            .disks
            .iter()
            .map(|d| {
                let (source_type, name, image) = match &d.source {
                    crate::config::DiskSource::Blank => ("blank".to_string(), None, None),
                    crate::config::DiskSource::PVC { name } => {
                        ("pvc".to_string(), Some(name.clone()), None)
                    }
                    crate::config::DiskSource::ContainerDisk { image } => {
                        ("containerDisk".to_string(), None, Some(image.clone()))
                    }
                    crate::config::DiskSource::DataVolume { name } => {
                        ("dataVolume".to_string(), Some(name.clone()), None)
                    }
                };

                let device_type = match d.device_type {
                    crate::config::DiskDeviceType::Disk => "disk",
                    crate::config::DiskDeviceType::CDROM => "cdrom",
                    crate::config::DiskDeviceType::LUN => "lun",
                };

                CRDDiskSpec {
                    name: d.name.clone(),
                    size: d.size.clone(),
                    storage_class: d.storage_class.clone(),
                    boot_order: d.boot_order,
                    source: CRDDiskSource {
                        source_type,
                        name,
                        image,
                    },
                    device_type: device_type.to_string(),
                    bus: d.bus.clone(),
                    cache: d.cache.clone(),
                    io: d.io.clone(),
                }
            })
            .collect();

        let interfaces = config
            .interfaces
            .iter()
            .map(|i| {
                let (net_type, net_name) = match &i.network_type {
                    crate::config::NetworkType::Pod => ("pod".to_string(), None),
                    crate::config::NetworkType::Bridge => ("bridge".to_string(), None),
                    crate::config::NetworkType::Multus { name } => {
                        ("multus".to_string(), Some(name.clone()))
                    }
                    crate::config::NetworkType::SRIOV { name } => {
                        ("sriov".to_string(), Some(name.clone()))
                    }
                };

                CRDInterfaceSpec {
                    name: i.name.clone(),
                    network: i.network.clone(),
                    model: i.model.clone(),
                    network_type: CRDNetworkType {
                        net_type,
                        name: net_name,
                    },
                    mac_address: i.mac_address.clone(),
                }
            })
            .collect();

        Self {
            template: None,
            profile: None,
            cpu: CRDCPUSpec {
                cores: config.cpu.cores,
                sockets: config.cpu.sockets,
                threads: config.cpu.threads,
                model: config.cpu.model.clone(),
                dedicated_cpu_placement: config.cpu.dedicated_cpu_placement,
                isolate_emulator_thread: config.cpu.isolate_emulator_thread,
            },
            memory: CRDMemorySpec {
                size: config.memory.size.clone(),
                hugepages_page_size: config.memory.hugepages_page_size.clone(),
                max_guest: config.memory.max_guest.clone(),
            },
            disks,
            interfaces,
            cloud_init: config.cloud_init.as_ref().map(|ci| CRDCloudInitSpec {
                user_data: ci.user_data.clone(),
                network_data: ci.network_data.clone(),
            }),
            features: config.features.as_ref().map(|f| CRDFeaturesSpec {
                acpi: f.acpi,
                apic: f.apic,
                hyperv: f.hyperv.as_ref().map(|h| CRDHyperVSpec {
                    relaxed: h.relaxed,
                    vapic: h.vapic,
                    spinlocks: h.spinlocks,
                    vpindex: h.vpindex,
                    runtime: h.runtime,
                    synic: h.synic,
                    stimer: h.stimer,
                    reset: h.reset,
                    frequencies: h.frequencies,
                    reenlightenment: h.reenlightenment,
                    tlbflush: h.tlbflush,
                    ipi: h.ipi,
                }),
                kvm_hidden: f.kvm_hidden,
                smm: f.smm,
            }),
            firmware: config.firmware.as_ref().map(|fw| {
                let (bootloader, secure_boot, persistent) = match &fw.bootloader {
                    crate::config::BootloaderType::BIOS => ("bios".to_string(), false, false),
                    crate::config::BootloaderType::EFI {
                        secure_boot,
                        persistent,
                    } => ("efi".to_string(), *secure_boot, *persistent),
                };
                CRDFirmwareSpec {
                    bootloader,
                    secure_boot,
                    persistent,
                }
            }),
            clock: config.clock.as_ref().map(|c| CRDClockSpec {
                utc: c.utc,
                timezone: c.timezone.clone(),
                timers: c.timers.as_ref().map(|t| CRDTimersSpec {
                    hpet_present: t.hpet_present,
                    pit_tick_policy: t.pit_tick_policy.clone(),
                    rtc_tick_policy: t.rtc_tick_policy.clone(),
                    hyperv_present: t.hyperv_present,
                }),
            }),
            eviction_strategy: config.eviction_strategy.clone(),
            termination_grace_period: config.termination_grace_period,
            enable_tpm: config.enable_tpm,
            enable_rng: config.enable_rng,
            machine_type: config.machine_type.clone(),
            running: None,
            labels: config.labels.clone(),
            annotations: config.annotations.clone(),
        }
    }
}

fn default_one() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_disk() -> String {
    "disk".to_string()
}

fn default_virtio() -> String {
    "virtio".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfigBuilder;
    use kube::CustomResourceExt;

    #[test]
    fn test_vmconfig_to_crd_spec() {
        let config = VMConfigBuilder::new("test-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("root", "20Gi", 1)
            .add_pod_network("eth0")
            .build();

        let spec = VMRogueVMSpec::from(&config);

        assert_eq!(spec.cpu.cores, 4);
        assert_eq!(spec.memory.size, "8Gi");
        assert_eq!(spec.disks.len(), 1);
        assert_eq!(spec.disks[0].source.source_type, "blank");
        assert_eq!(spec.interfaces.len(), 1);
        assert_eq!(spec.interfaces[0].network_type.net_type, "pod");
    }

    #[test]
    fn test_vmroguevm_crd_generates() {
        // Verify the CustomResource derive generates the correct CRD metadata
        let crd = VMRogueVM::crd();
        assert_eq!(crd.metadata.name.as_deref(), Some("vmroguevms.vmrogue.io"));
    }

    #[test]
    fn test_vmrogueblueprint_crd_generates() {
        let crd = VMRogueBlueprint::crd();
        assert_eq!(
            crd.metadata.name.as_deref(),
            Some("vmrogueblueprints.vmrogue.io")
        );
    }

    #[test]
    fn test_vmroguepolicy_crd_generates() {
        let crd = VMRoguePolicy::crd();
        assert_eq!(
            crd.metadata.name.as_deref(),
            Some("vmroguepolicies.vmrogue.io")
        );
    }

    #[test]
    fn test_vmrogueinsight_crd_generates() {
        let crd = VMRogueInsight::crd();
        assert_eq!(
            crd.metadata.name.as_deref(),
            Some("vmrogueinsights.vmrogue.io")
        );
    }

    #[test]
    fn test_vmrogueaction_crd_generates() {
        let crd = VMRogueAction::crd();
        assert_eq!(
            crd.metadata.name.as_deref(),
            Some("vmrogueactions.vmrogue.io")
        );
    }
}
