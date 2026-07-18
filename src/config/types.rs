// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pod-like scheduling for the virt-launcher / VMI (`affinity`, `tolerations`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VmScheduling {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<HashMap<String, String>>,
    /// Same JSON shape as `pod.spec.affinity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerations: Option<Vec<serde_json::Value>>,
    /// Same as `pod.spec.topologySpreadConstraints`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topology_spread_constraints: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority_class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_name: Option<String>,
}

/// KubeVirt `instancetype` / `preference` matcher (`kind` optional; defaults apply in KubeVirt).
///
/// Either `name` names an instancetype/preference directly, or `infer_from_volume`
/// names a VM volume — KubeVirt then reads the size/shape from the
/// `instancetype.kubevirt.io/default-instancetype` (or `default-preference`) label on
/// that volume's PVC/DataSource. Inference is how a VM auto-sizes from a golden image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmMatcherRef {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Volume name to infer the matcher from (mutually exclusive with `name`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infer_from_volume: Option<String>,
}

/// QEMU watchdog (`i6300esb`, `ib700`, …) and action (`poweroff`, `reset`, `pause`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmWatchdogConfig {
    pub model: String,
    pub action: String,
}

/// Mediated device / GPU pool reference (`resourceName` from the device plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHostDevice {
    pub name: String,
    pub resource_name: String,
}

/// vGPU display options (`domain.devices.gpus[].virtualGPUOptions`). Phase-2
/// material — only meaningful for mediated (vGPU) devices, ignored for
/// whole-GPU passthrough.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VmVgpuOptions {
    /// Enable the vGPU display head (`virtualGPUOptions.display.enabled`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<bool>,
    /// Enable the RAM framebuffer (`virtualGPUOptions.display.ramFB.enabled`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_fb: Option<bool>,
}

/// GPU assignment via KubeVirt `domain.devices.gpus` (whole-GPU passthrough
/// or a vGPU mediated device — `device_name` is the device-plugin resource,
/// e.g. `nvidia.com/gpu` or `nvidia.com/GRID_T4-2Q`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmGpuDevice {
    pub name: String,
    pub device_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_gpu_options: Option<VmVgpuOptions>,
}

/// Virtio-FS mount backed by a PVC (requires KubeVirt + cluster support).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmVirtioFs {
    pub name: String,
    pub pvc_name: String,
}

/// Main VM configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VMConfig {
    pub name: String,
    pub namespace: String,
    pub cpu: CPUConfig,
    pub memory: MemoryConfig,
    pub disks: Vec<DiskConfig>,
    pub interfaces: Vec<InterfaceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_init: Option<CloudInitConfig>,
    /// Windows unattended setup (`autounattend.xml`), mounted as a sysprep CD-ROM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysprep: Option<SysprepConfig>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    /// Domain features (ACPI, HyperV enlightenments, SMM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<FeaturesConfig>,
    /// Firmware configuration (UEFI/BIOS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<FirmwareConfig>,
    /// Clock and timekeeping configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<ClockConfig>,
    /// Eviction strategy (e.g., "LiveMigrate", "LiveMigrateIfPossible")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eviction_strategy: Option<String>,
    /// Termination grace period in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_grace_period: Option<i64>,
    /// Enable TPM 2.0 device
    #[serde(default)]
    pub enable_tpm: bool,
    /// Enable RNG (virtio-rng) device
    #[serde(default)]
    pub enable_rng: bool,
    /// Enable USB tablet input device for accurate mouse positioning over VNC
    #[serde(default)]
    pub usb_tablet: bool,
    /// Disable virtio-balloon memory reclaim to avoid latency jitter
    #[serde(default)]
    pub disable_balloon: bool,
    /// KubeVirt ioThreadsPolicy: "shared" or "auto" for better disk IO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_threads_policy: Option<String>,
    /// KubeVirt `spec.template.spec.domain.devices.video` device type (e.g. `virtio`, `bochs`).
    /// Requires the cluster KubeVirt **VideoConfig** alpha feature gate when overriding defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubevirt_video_type: Option<String>,
    /// Machine type (e.g., "q35")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,
    /// When set, Veyron creates a Kubernetes Service targeting the virt-launcher pod (`vm.kubevirt.io/name`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose: Option<VmExposeConfig>,
    /// KubeVirt `runStrategy` (`Always`, `Manual`, `RerunOnFailure`, `Halted`). When set, `spec.running` is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<VmScheduling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instancetype: Option<VmMatcherRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preference: Option<VmMatcherRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<VmWatchdogConfig>,
    #[serde(default)]
    pub host_devices: Vec<VmHostDevice>,
    /// GPUs bound via KubeVirt `domain.devices.gpus` (passthrough or vGPU).
    #[serde(default)]
    pub gpus: Vec<VmGpuDevice>,
    #[serde(default)]
    pub virtio_fs: Vec<VmVirtioFs>,
    /// Raw KubeVirt `accessCredentials` entries (password / SSH injection).
    #[serde(default)]
    pub access_credentials: Vec<serde_json::Value>,
    /// When true (default), apply CiliumNetworkPolicy or NetworkPolicy egress for virt-launcher pods.
    #[serde(default = "default_allow_internet")]
    pub allow_internet: bool,
}

/// Default for [`VMConfig::allow_internet`].
pub fn default_allow_internet() -> bool {
    true
}

impl VMConfig {
    /// First meaningful blank/data disk size for UI defaults (skips container/CDROM `0` sizes).
    pub fn default_disk_size_label(&self) -> String {
        self.disks
            .iter()
            .find(|d| {
                d.device_type != DiskDeviceType::CDROM
                    && !matches!(d.source, DiskSource::ContainerDisk { .. })
                    && !d.size.is_empty()
                    && d.size != "0"
            })
            .map(|d| d.size.clone())
            .or_else(|| {
                self.disks
                    .iter()
                    .find(|d| !d.size.is_empty() && d.size != "0")
                    .map(|d| d.size.clone())
            })
            .unwrap_or_else(|| "20Gi".to_string())
    }
}

/// CPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUConfig {
    pub cores: u32,
    #[serde(default = "default_sockets")]
    pub sockets: u32,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Pin vCPUs to physical CPUs for latency-sensitive workloads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_cpu_placement: Option<bool>,
    /// Isolate QEMU emulator thread from vCPU threads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolate_emulator_thread: Option<bool>,
}

impl Default for CPUConfig {
    fn default() -> Self {
        Self {
            cores: 1,
            sockets: 1,
            threads: 1,
            model: None,
            dedicated_cpu_placement: None,
            isolate_emulator_thread: None,
        }
    }
}

fn default_sockets() -> u32 {
    1
}

fn default_threads() -> u32 {
    1
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub size: String,
    /// Hugepages page size (e.g., "2Mi", "1Gi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages_page_size: Option<String>,
    /// Maximum guest memory for hotplug (e.g., "16Gi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_guest: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            size: "2Gi".to_string(),
            hugepages_page_size: None,
            max_guest: None,
        }
    }
}

/// Disk device type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiskDeviceType {
    #[default]
    Disk,
    #[serde(rename = "cdrom")]
    CDROM,
    #[serde(rename = "lun")]
    LUN,
}

/// Disk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub name: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class: Option<String>,
    pub boot_order: u32,
    pub source: DiskSource,
    /// Device type: "disk" (default), "cdrom", or "lun"
    #[serde(default)]
    pub device_type: DiskDeviceType,
    /// Bus type: "virtio" (default), "sata", "scsi"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    /// Cache mode: "none", "writethrough", "writeback"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    /// I/O mode: "native", "threads", "default"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io: Option<String>,
}

/// Disk source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DiskSource {
    Blank,
    #[serde(rename = "pvc")]
    PVC {
        name: String,
    },
    ContainerDisk {
        image: String,
    },
    /// Reference a DataVolume that already exists, by name.
    DataVolume {
        name: String,
    },
    /// Clone a per-VM disk from a golden image at create time.
    ///
    /// Unlike `PVC` / `DataVolume` — which *point at* a shared disk and would let
    /// two VMs write the same volume — this emits a `dataVolumeTemplates` entry, so
    /// the VM gets its own copy, garbage-collected with it.
    ///
    /// `DataSource` is the stable catalog handle: repointing it at a new image
    /// version rolls the whole fleet's next clone without editing any VM.
    GoldenImage {
        /// CDI `DataSource` name (preferred), or a source PVC name when `from_pvc`.
        name: String,
        /// Namespace holding the DataSource / source PVC (e.g. `vm-images`).
        namespace: String,
        /// Clone straight from a PVC instead of resolving a DataSource.
        #[serde(default)]
        from_pvc: bool,
        /// Storage class for the *clone*; falls back to the cluster default.
        #[serde(skip_serializing_if = "Option::is_none")]
        storage_class: Option<String>,
        /// `Filesystem` or `Block` for the clone. Omit to inherit the StorageProfile
        /// default — but a clone should match its source's mode: a cross-mode clone
        /// falls back to host-assisted copy, which is slow and, on some Ceph RBD
        /// setups, fails with a block-device permission error. Prefer matching the
        /// golden image's mode (usually `Filesystem`) to get the fast CSI clone.
        #[serde(skip_serializing_if = "Option::is_none")]
        volume_mode: Option<String>,
    },
}

/// Windows unattended-setup answer file, delivered to the guest as sysprep media.
///
/// Prefer `secret` — an `autounattend.xml` almost always carries the local
/// administrator password and/or domain-join credentials.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SysprepConfig {
    /// Secret holding key `autounattend.xml`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    /// ConfigMap holding key `autounattend.xml` (only when there are no secrets in it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<String>,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceConfig {
    pub name: String,
    pub network: String,
    #[serde(default = "default_interface_model")]
    pub model: String,
    pub network_type: NetworkType,
    /// MAC address for the interface (e.g., "52:54:00:12:34:56")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
}

fn default_interface_model() -> String {
    "virtio".to_string()
}

/// Network type options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum NetworkType {
    Bridge,
    Multus {
        name: String,
    },
    #[default]
    Pod,
    #[serde(rename = "sriov")]
    SRIOV {
        name: String,
    },
    /// OVN-Kubernetes overlay via Multus NetworkAttachmentDefinition (e.g. `ovn-k8s-cni-overlay`).
    Ovn {
        name: String,
    },
}

/// How cloud-init / Cloudbase-Init user data is delivered (`NoCloud` vs ISO config-drive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloudInitDelivery {
    /// Linux cloud-init via `cloudInitNoCloud`.
    #[default]
    NoCloud,
    /// Config-drive ISO (`cloudInitConfigDrive`), e.g. Cloudbase-Init on Windows.
    ConfigDrive,
}

/// Cloud-init configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudInitConfig {
    pub user_data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_data: Option<String>,
    #[serde(default)]
    pub delivery: CloudInitDelivery,
}

/// Opt-in Kubernetes Service to reach guest ports via the VM pod network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExposeConfig {
    #[serde(default)]
    pub enabled: bool,
    /// `ClusterIP`, `NodePort`, or `LoadBalancer`.
    #[serde(default = "default_expose_service_type")]
    pub service_type: String,
    #[serde(default)]
    pub ports: Vec<VmExposePort>,
}

fn default_expose_service_type() -> String {
    "ClusterIP".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExposePort {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub port: i32,
    pub target_port: i32,
    #[serde(default = "default_tcp_protocol")]
    pub protocol: String,
    /// External port on each node for `NodePort`/`LoadBalancer` services.
    /// Must be within the cluster NodePort range (30000–32767); `None` lets
    /// Kubernetes auto-assign one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_port: Option<i32>,
}

fn default_tcp_protocol() -> String {
    "TCP".to_string()
}

// ============================================================================
// Features Configuration
// ============================================================================

/// VM features configuration (ACPI, HyperV enlightenments, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Enable ACPI (default: true for most guests)
    #[serde(default = "default_true")]
    pub acpi: bool,
    /// Enable APIC
    #[serde(default)]
    pub apic: bool,
    /// HyperV enlightenments (critical for Windows performance)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperv: Option<HyperVConfig>,
    /// KVM hidden state (hide KVM from guest, e.g., for GPU passthrough)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm_hidden: Option<bool>,
    /// SMM (System Management Mode) — required for UEFI Secure Boot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smm: Option<bool>,
}

fn default_true() -> bool {
    true
}

/// HyperV enlightenments for Windows guest performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperVConfig {
    /// Relaxed timing (reduces timer-related overhead)
    #[serde(default = "default_true")]
    pub relaxed: bool,
    /// Virtual APIC (reduces interrupt overhead)
    #[serde(default = "default_true")]
    pub vapic: bool,
    /// Spinlock retries before yielding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinlocks: Option<u32>,
    /// Virtual processor index MSR
    #[serde(default)]
    pub vpindex: bool,
    /// Partition reference time MSR
    #[serde(default)]
    pub runtime: bool,
    /// Synthetic interrupt controller
    #[serde(default)]
    pub synic: bool,
    /// Synthetic timers
    #[serde(default)]
    pub stimer: bool,
    /// Reset support
    #[serde(default)]
    pub reset: bool,
    /// Frequency MSRs
    #[serde(default)]
    pub frequencies: bool,
    /// Reenlightenment notification
    #[serde(default)]
    pub reenlightenment: bool,
    /// TLB flush optimization
    #[serde(default)]
    pub tlbflush: bool,
    /// IPI (Inter-Processor Interrupt) optimization
    #[serde(default)]
    pub ipi: bool,
}

// ============================================================================
// Firmware Configuration
// ============================================================================

/// Firmware/bootloader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareConfig {
    /// Bootloader type
    pub bootloader: BootloaderType,
}

/// Bootloader type selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootloaderType {
    /// Standard BIOS boot
    BIOS,
    /// UEFI boot
    EFI {
        /// Enable UEFI Secure Boot
        #[serde(default)]
        secure_boot: bool,
        /// Persist NVRAM across reboots
        #[serde(default = "default_true_val")]
        persistent: bool,
    },
}

fn default_true_val() -> bool {
    true
}

// ============================================================================
// Clock Configuration
// ============================================================================

/// Clock and timekeeping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockConfig {
    /// Use UTC offset (default: true)
    #[serde(default = "default_true")]
    pub utc: bool,
    /// Timezone (alternative to UTC, e.g., "America/New_York")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Timer configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timers: Option<TimersConfig>,
}

/// Timer configuration for guest clock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimersConfig {
    /// HPET timer present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hpet_present: Option<bool>,
    /// PIT tick policy ("delay", "catchup", "merge", "discard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_tick_policy: Option<String>,
    /// RTC tick policy ("delay", "catchup", "merge", "discard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_tick_policy: Option<String>,
    /// Enable HyperV timer (for Windows guests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperv_present: Option<bool>,
}
