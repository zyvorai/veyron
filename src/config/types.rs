use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Machine type (e.g., "q35")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,
    /// When set, VMRogue creates a Kubernetes Service targeting the virt-launcher pod (`kubevirt.io/domain`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose: Option<VmExposeConfig>,
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
    DataVolume {
        name: String,
    },
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
