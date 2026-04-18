use crate::config::*;
use std::collections::HashMap;
use std::sync::LazyLock;

// ============================================================================
// Shared feature/firmware/clock presets
// ============================================================================

/// Windows HyperV enlightenments for optimal performance
fn windows_features() -> FeaturesConfig {
    FeaturesConfig {
        acpi: true,
        apic: true,
        hyperv: Some(HyperVConfig {
            relaxed: true,
            vapic: true,
            spinlocks: Some(8191),
            vpindex: true,
            runtime: true,
            synic: true,
            stimer: true,
            reset: true,
            frequencies: true,
            reenlightenment: true,
            tlbflush: true,
            ipi: true,
        }),
        kvm_hidden: None,
        smm: Some(true),
    }
}

/// UEFI firmware with Secure Boot (required for Windows 11)
fn uefi_secure_boot_firmware() -> FirmwareConfig {
    FirmwareConfig {
        bootloader: BootloaderType::EFI {
            secure_boot: true,
            persistent: true,
        },
    }
}

/// UEFI firmware without Secure Boot
fn uefi_firmware() -> FirmwareConfig {
    FirmwareConfig {
        bootloader: BootloaderType::EFI {
            secure_boot: false,
            persistent: true,
        },
    }
}

/// Windows clock configuration with HyperV timer
fn windows_clock() -> ClockConfig {
    ClockConfig {
        utc: true,
        timezone: None,
        timers: Some(TimersConfig {
            hpet_present: Some(false),
            pit_tick_policy: Some("delay".to_string()),
            rtc_tick_policy: Some("catchup".to_string()),
            hyperv_present: Some(true),
        }),
    }
}

/// Linux clock configuration
fn linux_clock() -> ClockConfig {
    ClockConfig {
        utc: true,
        timezone: None,
        timers: Some(TimersConfig {
            hpet_present: Some(false),
            pit_tick_policy: Some("delay".to_string()),
            rtc_tick_policy: Some("catchup".to_string()),
            hyperv_present: None,
        }),
    }
}

pub static TEMPLATES: LazyLock<TemplateManager> = LazyLock::new(TemplateManager::new);

/// Manages VM templates
pub struct TemplateManager {
    templates: HashMap<String, VMConfig>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Ubuntu variants
        templates.insert("ubuntu".to_string(), ubuntu_2204_template());
        templates.insert("ubuntu-24.04".to_string(), ubuntu_2404_template());
        templates.insert("ubuntu-22.04".to_string(), ubuntu_2204_template());
        templates.insert("ubuntu-20.04".to_string(), ubuntu_2004_template());
        templates.insert("ubuntu-18.04".to_string(), ubuntu_1804_template());

        // Fedora variants
        templates.insert("fedora".to_string(), fedora_43_template());
        templates.insert("fedora-43".to_string(), fedora_43_template());
        templates.insert("fedora-42".to_string(), fedora_42_template());

        // CentOS variants
        templates.insert("centos".to_string(), centos_stream9_template());
        templates.insert("centos-stream-9".to_string(), centos_stream9_template());
        templates.insert("centos-stream-8".to_string(), centos_stream8_template());

        // Debian variants
        templates.insert("debian".to_string(), debian_12_template());
        templates.insert("debian-12".to_string(), debian_12_template());
        templates.insert("debian-11".to_string(), debian_11_template());

        // RHEL variants
        templates.insert("rhel".to_string(), rhel_9_template());
        templates.insert("rhel-9".to_string(), rhel_9_template());
        templates.insert("rhel-8".to_string(), rhel_8_template());

        // AlmaLinux
        templates.insert("almalinux".to_string(), almalinux_9_template());
        templates.insert("almalinux-9".to_string(), almalinux_9_template());
        templates.insert("almalinux-8".to_string(), almalinux_8_template());

        // Rocky Linux
        templates.insert("rocky".to_string(), rocky_9_template());
        templates.insert("rocky-9".to_string(), rocky_9_template());
        templates.insert("rocky-8".to_string(), rocky_8_template());

        // OpenSUSE
        templates.insert("opensuse".to_string(), opensuse_leap_template());
        templates.insert("opensuse-leap".to_string(), opensuse_leap_template());
        templates.insert(
            "opensuse-tumbleweed".to_string(),
            opensuse_tumbleweed_template(),
        );

        // Alpine Linux
        templates.insert("alpine".to_string(), alpine_template());
        templates.insert("alpine-3.19".to_string(), alpine_template());

        // Arch Linux
        templates.insert("arch".to_string(), arch_template());

        // Oracle Linux
        templates.insert("oracle".to_string(), oracle_9_template());
        templates.insert("oracle-9".to_string(), oracle_9_template());
        templates.insert("oracle-8".to_string(), oracle_8_template());

        // Windows variants
        templates.insert("windows".to_string(), windows_2022_template());
        templates.insert("windows-2022".to_string(), windows_2022_template());
        templates.insert("windows-2019".to_string(), windows_2019_template());
        templates.insert("windows-11".to_string(), windows_11_template());
        templates.insert("windows-10".to_string(), windows_10_template());

        // FreeBSD
        templates.insert("freebsd".to_string(), freebsd_14_template());
        templates.insert("freebsd-14".to_string(), freebsd_14_template());
        templates.insert("freebsd-13".to_string(), freebsd_13_template());

        // Flatcar Linux
        templates.insert("flatcar".to_string(), flatcar_template());

        // Talos Linux (for Kubernetes)
        templates.insert("talos".to_string(), talos_template());

        Self { templates }
    }

    pub fn get(&self, name: &str) -> Option<VMConfig> {
        self.templates.get(name).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<_> = self.templates.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn exists(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get templates grouped by OS family
    pub fn list_by_family(&self) -> HashMap<String, Vec<String>> {
        let mut families: HashMap<String, Vec<String>> = HashMap::new();

        for name in self.templates.keys() {
            let family = if name.starts_with("ubuntu") {
                "Ubuntu"
            } else if name.starts_with("fedora") {
                "Fedora"
            } else if name.starts_with("centos") {
                "CentOS"
            } else if name.starts_with("debian") {
                "Debian"
            } else if name.starts_with("rhel") {
                "RHEL"
            } else if name.starts_with("almalinux") {
                "AlmaLinux"
            } else if name.starts_with("rocky") {
                "Rocky Linux"
            } else if name.starts_with("opensuse") {
                "OpenSUSE"
            } else if name.starts_with("alpine") {
                "Alpine"
            } else if name.starts_with("arch") {
                "Arch Linux"
            } else if name.starts_with("oracle") {
                "Oracle Linux"
            } else if name.starts_with("windows") {
                "Windows"
            } else if name.starts_with("freebsd") {
                "FreeBSD"
            } else if name.starts_with("flatcar") {
                "Flatcar"
            } else if name.starts_with("talos") {
                "Talos"
            } else {
                "Other"
            };

            families
                .entry(family.to_string())
                .or_default()
                .push(name.clone());
        }

        // Sort each family's templates
        for templates in families.values_mut() {
            templates.sort();
        }

        families
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ubuntu Templates
// ============================================================================

fn ubuntu_2404_template() -> VMConfig {
    VMConfigBuilder::new("ubuntu-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:24.04", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "ubuntu")
        .label("os.version", "24.04")
        .cloud_init(default_ubuntu_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn ubuntu_2204_template() -> VMConfig {
    VMConfigBuilder::new("ubuntu-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:22.04", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "ubuntu")
        .label("os.version", "22.04")
        .cloud_init(default_ubuntu_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn ubuntu_2004_template() -> VMConfig {
    VMConfigBuilder::new("ubuntu-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:20.04", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "ubuntu")
        .label("os.version", "20.04")
        .cloud_init(default_ubuntu_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn ubuntu_1804_template() -> VMConfig {
    VMConfigBuilder::new("ubuntu-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:18.04", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "ubuntu")
        .label("os.version", "18.04")
        .cloud_init(default_ubuntu_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Fedora Templates
// ============================================================================

fn fedora_43_template() -> VMConfig {
    VMConfigBuilder::new("fedora-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/fedora:43", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "fedora")
        .label("os.version", "43")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn fedora_42_template() -> VMConfig {
    VMConfigBuilder::new("fedora-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/fedora:42", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "fedora")
        .label("os.version", "42")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// CentOS Templates
// ============================================================================

fn centos_stream9_template() -> VMConfig {
    VMConfigBuilder::new("centos-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/centos-stream:9", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "centos")
        .label("os.version", "stream9")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn centos_stream8_template() -> VMConfig {
    VMConfigBuilder::new("centos-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/centos-stream:8", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "centos")
        .label("os.version", "stream8")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Debian Templates
// ============================================================================

fn debian_12_template() -> VMConfig {
    VMConfigBuilder::new("debian-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/debian:12", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "debian")
        .label("os.version", "12")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn debian_11_template() -> VMConfig {
    VMConfigBuilder::new("debian-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/debian:11", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "debian")
        .label("os.version", "11")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// RHEL Templates
// ============================================================================

fn rhel_9_template() -> VMConfig {
    VMConfigBuilder::new("rhel-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "30Gi", 1)
        .add_pod_network("default")
        .label("os", "rhel")
        .label("os.version", "9")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn rhel_8_template() -> VMConfig {
    VMConfigBuilder::new("rhel-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "30Gi", 1)
        .add_pod_network("default")
        .label("os", "rhel")
        .label("os.version", "8")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// AlmaLinux Templates
// ============================================================================

fn almalinux_9_template() -> VMConfig {
    VMConfigBuilder::new("almalinux-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/almalinux:9", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "almalinux")
        .label("os.version", "9")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn almalinux_8_template() -> VMConfig {
    VMConfigBuilder::new("almalinux-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/almalinux:8", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "almalinux")
        .label("os.version", "8")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Rocky Linux Templates
// ============================================================================

fn rocky_9_template() -> VMConfig {
    VMConfigBuilder::new("rocky-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/rockylinux:9", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "rocky")
        .label("os.version", "9")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn rocky_8_template() -> VMConfig {
    VMConfigBuilder::new("rocky-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/rockylinux:8", 1)
        .add_blank_disk("datadisk", "20Gi", 2)
        .add_pod_network("default")
        .label("os", "rocky")
        .label("os.version", "8")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// OpenSUSE Templates
// ============================================================================

fn opensuse_leap_template() -> VMConfig {
    VMConfigBuilder::new("opensuse-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "opensuse")
        .label("os.version", "leap")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn opensuse_tumbleweed_template() -> VMConfig {
    VMConfigBuilder::new("opensuse-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "opensuse")
        .label("os.version", "tumbleweed")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Alpine Linux Template
// ============================================================================

fn alpine_template() -> VMConfig {
    VMConfigBuilder::new("alpine-vm")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("512Mi")
        .add_container_disk("rootdisk", "quay.io/containerdisks/alpine:3.19", 1)
        .add_blank_disk("datadisk", "10Gi", 2)
        .add_pod_network("default")
        .label("os", "alpine")
        .label("os.version", "3.19")
        .cloud_init(alpine_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Arch Linux Template
// ============================================================================

fn arch_template() -> VMConfig {
    VMConfigBuilder::new("arch-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("2Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "arch")
        .label("os.version", "latest")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Oracle Linux Templates
// ============================================================================

fn oracle_9_template() -> VMConfig {
    VMConfigBuilder::new("oracle-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "30Gi", 1)
        .add_pod_network("default")
        .label("os", "oracle")
        .label("os.version", "9")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn oracle_8_template() -> VMConfig {
    VMConfigBuilder::new("oracle-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "30Gi", 1)
        .add_pod_network("default")
        .label("os", "oracle")
        .label("os.version", "8")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Windows Templates
// ============================================================================

fn windows_2022_template() -> VMConfig {
    VMConfigBuilder::new("windows-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "60Gi", 1)
        .add_cdrom("virtio-drivers", "quay.io/containerdisks/virtio-win", 2)
        .add_pod_network("default")
        .label("os", "windows")
        .label("os.version", "2022")
        .features(windows_features())
        .firmware(uefi_firmware())
        .clock(windows_clock())
        .enable_tpm()
        .enable_rng()
        .termination_grace_period(120)
        .machine_type("q35")
        .build()
}

fn windows_2019_template() -> VMConfig {
    VMConfigBuilder::new("windows-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "60Gi", 1)
        .add_cdrom("virtio-drivers", "quay.io/containerdisks/virtio-win", 2)
        .add_pod_network("default")
        .label("os", "windows")
        .label("os.version", "2019")
        .features(windows_features())
        .firmware(uefi_firmware())
        .clock(windows_clock())
        .enable_rng()
        .termination_grace_period(120)
        .machine_type("q35")
        .build()
}

fn windows_11_template() -> VMConfig {
    VMConfigBuilder::new("windows-vm")
        .namespace("default")
        .cpu(4, 2, 1)
        // Keep the default guest small enough to schedule on single-node lab clusters.
        // Users can still override this upward for production-grade Windows 11 guests.
        .memory("4Gi")
        .add_blank_disk("rootdisk", "80Gi", 1)
        .add_cdrom("virtio-drivers", "quay.io/containerdisks/virtio-win", 2)
        .add_pod_network("default")
        .label("os", "windows")
        .label("os.version", "11")
        .features(windows_features())
        .firmware(uefi_secure_boot_firmware())
        .clock(windows_clock())
        .enable_tpm()
        .enable_rng()
        .termination_grace_period(120)
        .machine_type("q35")
        .build()
}

fn windows_10_template() -> VMConfig {
    VMConfigBuilder::new("windows-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "60Gi", 1)
        .add_cdrom("virtio-drivers", "quay.io/containerdisks/virtio-win", 2)
        .add_pod_network("default")
        .label("os", "windows")
        .label("os.version", "10")
        .features(windows_features())
        .firmware(uefi_firmware())
        .clock(windows_clock())
        .enable_rng()
        .termination_grace_period(120)
        .machine_type("q35")
        .build()
}

// ============================================================================
// FreeBSD Templates
// ============================================================================

fn freebsd_14_template() -> VMConfig {
    VMConfigBuilder::new("freebsd-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("2Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "freebsd")
        .label("os.version", "14")
        .enable_rng()
        .clock(linux_clock())
        .build()
}

fn freebsd_13_template() -> VMConfig {
    VMConfigBuilder::new("freebsd-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("2Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "freebsd")
        .label("os.version", "13")
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Flatcar Linux Template
// ============================================================================

fn flatcar_template() -> VMConfig {
    VMConfigBuilder::new("flatcar-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("2Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "flatcar")
        .label("os.version", "stable")
        .cloud_init(default_cloud_init())
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Talos Linux Template (for Kubernetes)
// ============================================================================

fn talos_template() -> VMConfig {
    VMConfigBuilder::new("talos-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .label("os", "talos")
        .label("os.version", "latest")
        .enable_rng()
        .clock(linux_clock())
        .build()
}

// ============================================================================
// Cloud-init configurations
// ============================================================================

fn generate_random_password() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%"
        .chars()
        .collect();
    (0..16)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

fn default_cloud_init() -> String {
    let password = generate_random_password();
    format!(
        r#"#cloud-config
user: vmrogue
password: {}
lock_passwd: false
chpasswd: {{ expire: True }}
ssh_pwauth: False
package_update: true
packages:
  - qemu-guest-agent
runcmd:
  - [ systemctl, enable, qemu-guest-agent ]
  - [ systemctl, start, qemu-guest-agent ]
"#,
        password
    )
}

fn default_ubuntu_cloud_init() -> String {
    let password = generate_random_password();
    format!(
        r#"#cloud-config
user: ubuntu
password: {}
lock_passwd: false
chpasswd: {{ expire: True }}
ssh_pwauth: False
package_update: true
packages:
  - qemu-guest-agent
runcmd:
  - [ systemctl, enable, qemu-guest-agent ]
  - [ systemctl, start, qemu-guest-agent ]
"#,
        password
    )
}

fn alpine_cloud_init() -> String {
    let password = generate_random_password();
    format!(
        r#"#cloud-config
user: alpine
password: {}
lock_passwd: false
chpasswd: {{ expire: True }}
ssh_pwauth: False
packages:
  - qemu-guest-agent
runcmd:
  - rc-update add qemu-guest-agent
  - rc-service qemu-guest-agent start
"#,
        password
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager() {
        let manager = TemplateManager::new();

        assert!(manager.exists("ubuntu"));
        assert!(manager.exists("ubuntu-22.04"));
        assert!(manager.exists("fedora"));
        assert!(manager.exists("almalinux"));
        assert!(manager.exists("rocky"));
        assert!(!manager.exists("nonexistent"));

        let ubuntu = manager.get("ubuntu").unwrap();
        assert_eq!(ubuntu.cpu.cores, 2);
        assert_eq!(ubuntu.memory.size, "4Gi");
    }

    #[test]
    fn test_list_templates() {
        let manager = TemplateManager::new();
        let templates = manager.list();

        assert!(templates.contains(&"ubuntu".to_string()));
        assert!(templates.contains(&"fedora".to_string()));
        assert!(templates.contains(&"almalinux".to_string()));
        assert!(templates.contains(&"rocky".to_string()));
        assert!(templates.contains(&"alpine".to_string()));

        // Should have many templates
        assert!(templates.len() > 30);
    }

    #[test]
    fn test_list_by_family() {
        let manager = TemplateManager::new();
        let families = manager.list_by_family();

        assert!(families.contains_key("Ubuntu"));
        assert!(families.contains_key("Fedora"));
        assert!(families.contains_key("AlmaLinux"));
        assert!(families.contains_key("Rocky Linux"));

        // Ubuntu should have multiple versions
        let ubuntu_templates = families.get("Ubuntu").unwrap();
        assert!(ubuntu_templates.len() >= 4);
    }
}
