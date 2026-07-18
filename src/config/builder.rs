// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use super::types::*;
use std::collections::HashMap;

/// Fluent builder for constructing [`VMConfig`] instances.
///
/// # Example
/// ```
/// use veyron::VMConfigBuilder;
///
/// let config = VMConfigBuilder::new("my-vm")
///     .namespace("production")
///     .cpu(4, 1, 1)
///     .memory("8Gi")
///     .add_blank_disk("rootdisk", "40Gi", 1)
///     .add_pod_network("eth0")
///     .label("app", "web")
///     .build();
///
/// assert_eq!(config.name, "my-vm");
/// assert_eq!(config.cpu.cores, 4);
/// ```
#[derive(Debug, Default)]
pub struct VMConfigBuilder {
    config: VMConfig,
}

impl VMConfigBuilder {
    /// Create a new builder with the given VM name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: VMConfig {
                name: name.into(),
                namespace: "default".to_string(),
                allow_internet: true,
                ..Default::default()
            },
        }
    }

    /// Set the Kubernetes namespace for the VM.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.config.namespace = namespace.into();
        self
    }

    /// Set the CPU topology (cores, sockets, threads).
    pub fn cpu(mut self, cores: u32, sockets: u32, threads: u32) -> Self {
        self.config.cpu.cores = cores;
        self.config.cpu.sockets = sockets;
        self.config.cpu.threads = threads;
        self
    }

    /// Set the CPU model (e.g., "host-passthrough", "Haswell").
    pub fn cpu_model(mut self, model: impl Into<String>) -> Self {
        self.config.cpu.model = Some(model.into());
        self
    }

    /// Enable dedicated CPU placement (CPU pinning).
    pub fn dedicated_cpu_placement(mut self, enabled: bool) -> Self {
        self.config.cpu.dedicated_cpu_placement = Some(enabled);
        self
    }

    /// Isolate emulator thread from vCPU threads.
    pub fn isolate_emulator_thread(mut self, enabled: bool) -> Self {
        self.config.cpu.isolate_emulator_thread = Some(enabled);
        self
    }

    /// Set the memory size (e.g., "4Gi", "512Mi").
    pub fn memory(mut self, size: impl Into<String>) -> Self {
        self.config.memory.size = size.into();
        self
    }

    /// Set hugepages page size (e.g., "2Mi", "1Gi").
    pub fn hugepages(mut self, page_size: impl Into<String>) -> Self {
        self.config.memory.hugepages_page_size = Some(page_size.into());
        self
    }

    /// Set maximum guest memory for hotplug (e.g., "16Gi").
    pub fn max_guest_memory(mut self, size: impl Into<String>) -> Self {
        self.config.memory.max_guest = Some(size.into());
        self
    }

    /// Add a pre-configured disk to the VM.
    pub fn add_disk(mut self, disk: DiskConfig) -> Self {
        self.config.disks.push(disk);
        self
    }

    /// Add an empty (blank) disk with the given size and boot order.
    pub fn add_blank_disk(
        mut self,
        name: impl Into<String>,
        size: impl Into<String>,
        boot_order: u32,
    ) -> Self {
        self.config.disks.push(DiskConfig {
            name: name.into(),
            size: size.into(),
            storage_class: None,
            boot_order,
            source: DiskSource::Blank,
            device_type: DiskDeviceType::default(),
            bus: None,
            cache: None,
            io: None,
        });
        self
    }

    /// Add an empty (blank) disk on the SATA bus — required for Windows VMs (no VirtIO storage driver).
    pub fn add_blank_disk_sata(
        mut self,
        name: impl Into<String>,
        size: impl Into<String>,
        boot_order: u32,
    ) -> Self {
        self.config.disks.push(DiskConfig {
            name: name.into(),
            size: size.into(),
            storage_class: None,
            boot_order,
            source: DiskSource::Blank,
            device_type: DiskDeviceType::default(),
            bus: Some("sata".to_string()),
            cache: None,
            io: None,
        });
        self
    }

    /// Add a container disk from an OCI image (e.g., `quay.io/containerdisks/fedora:39`).
    pub fn add_container_disk(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        boot_order: u32,
    ) -> Self {
        self.config.disks.push(DiskConfig {
            name: name.into(),
            size: "0".to_string(), // Container disks don't need size
            storage_class: None,
            boot_order,
            source: DiskSource::ContainerDisk {
                image: image.into(),
            },
            device_type: DiskDeviceType::default(),
            bus: None,
            cache: None,
            io: None,
        });
        self
    }

    /// Add a pre-configured network interface.
    pub fn add_interface(mut self, interface: InterfaceConfig) -> Self {
        self.config.interfaces.push(interface);
        self
    }

    /// Add a pod network interface with masquerade binding.
    pub fn add_pod_network(mut self, name: impl Into<String>) -> Self {
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: "default".to_string(),
            model: "virtio".to_string(),
            network_type: NetworkType::Pod,
            mac_address: None,
        });
        self
    }

    /// Add a pod network interface using e1000e — required for Windows VMs without VirtIO NIC driver.
    pub fn add_windows_network(mut self, name: impl Into<String>) -> Self {
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: "default".to_string(),
            model: "e1000e".to_string(),
            network_type: NetworkType::Pod,
            mac_address: None,
        });
        self
    }

    /// Add a bridge network interface.
    pub fn add_bridge_network(mut self, name: impl Into<String>) -> Self {
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: "default".to_string(),
            model: "virtio".to_string(),
            network_type: NetworkType::Bridge,
            mac_address: None,
        });
        self
    }

    /// Add a Multus CNI network interface with the given network attachment definition.
    pub fn add_multus_network(
        mut self,
        name: impl Into<String>,
        network_name: impl Into<String>,
    ) -> Self {
        let net_name = network_name.into();
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: net_name.clone(),
            model: "virtio".to_string(),
            network_type: NetworkType::Multus { name: net_name },
            mac_address: None,
        });
        self
    }

    /// Add a CDROM disk from an ISO image or container disk.
    pub fn add_cdrom(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        boot_order: u32,
    ) -> Self {
        self.config.disks.push(DiskConfig {
            name: name.into(),
            size: "0".to_string(),
            storage_class: None,
            boot_order,
            source: DiskSource::ContainerDisk {
                image: image.into(),
            },
            device_type: DiskDeviceType::CDROM,
            bus: Some("sata".to_string()),
            cache: None,
            io: None,
        });
        self
    }

    /// Add an SRIOV network interface.
    pub fn add_sriov_network(
        mut self,
        name: impl Into<String>,
        network_name: impl Into<String>,
    ) -> Self {
        let net_name = network_name.into();
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: net_name.clone(),
            model: "virtio".to_string(),
            network_type: NetworkType::SRIOV { name: net_name },
            mac_address: None,
        });
        self
    }

    /// Add an OVN-Kubernetes overlay interface via Multus NAD (e.g. `ovn-k8s-cni-overlay`).
    pub fn add_ovn_network(mut self, name: impl Into<String>, nad_name: impl Into<String>) -> Self {
        let net_name = nad_name.into();
        self.config.interfaces.push(InterfaceConfig {
            name: name.into(),
            network: net_name.clone(),
            model: "virtio".to_string(),
            network_type: NetworkType::Ovn { name: net_name },
            mac_address: None,
        });
        self
    }

    /// Set cloud-init user data for VM initialization (`cloudInitNoCloud`).
    pub fn cloud_init(mut self, user_data: impl Into<String>) -> Self {
        self.config.cloud_init = Some(CloudInitConfig {
            user_data: user_data.into(),
            network_data: None,
            delivery: crate::config::CloudInitDelivery::NoCloud,
        });
        self
    }

    /// User data delivered via config-drive ISO (`cloudInitConfigDrive`), e.g. Cloudbase-Init on Windows.
    ///
    /// Veyron creates a Secret with userdata and references it from the volume (see API create VM path).
    pub fn cloud_init_config_drive(mut self, user_data: impl Into<String>) -> Self {
        self.config.cloud_init = Some(CloudInitConfig {
            user_data: user_data.into(),
            network_data: None,
            delivery: crate::config::CloudInitDelivery::ConfigDrive,
        });
        self
    }

    /// Create a Kubernetes Service targeting this VM's virt-launcher (`kubevirt.io/domain` selector).
    pub fn expose(mut self, expose: crate::config::VmExposeConfig) -> Self {
        self.config.expose = Some(expose);
        self
    }

    /// Enable TPM 2.0 device.
    pub fn enable_tpm(mut self) -> Self {
        self.config.enable_tpm = true;
        self
    }

    /// Enable virtio-rng device.
    pub fn enable_rng(mut self) -> Self {
        self.config.enable_rng = true;
        self
    }

    /// Enable USB tablet input for accurate absolute mouse positioning over VNC.
    pub fn usb_tablet(mut self) -> Self {
        self.config.usb_tablet = true;
        self
    }

    /// Disable virtio-balloon to eliminate memory reclaim latency jitter.
    pub fn disable_balloon(mut self) -> Self {
        self.config.disable_balloon = true;
        self
    }

    /// Set ioThreadsPolicy ("shared" or "auto") for better disk IO throughput.
    pub fn io_threads_policy(mut self, policy: impl Into<String>) -> Self {
        self.config.io_threads_policy = Some(policy.into());
        self
    }

    /// USB tablet, no virtio-balloon, and shared ioThreads — improves pointer tracking and reduces
    /// latency jitter for browser VNC over KubeVirt.
    pub fn interactive_console_defaults(mut self) -> Self {
        self.config.usb_tablet = true;
        self.config.disable_balloon = true;
        self.config.io_threads_policy = Some("shared".into());
        self
    }

    /// Set KubeVirt console video device type (`domain.devices.video`). See `kubevirt_video_type` on [`VMConfig`].
    pub fn kubevirt_video_type(mut self, video_type: impl Into<String>) -> Self {
        self.config.kubevirt_video_type = Some(video_type.into());
        self
    }

    /// Enable or disable Veyron-managed internet egress policy for virt-launcher pods.
    pub fn allow_internet(mut self, enabled: bool) -> Self {
        self.config.allow_internet = enabled;
        self
    }

    /// Set the cache mode on a named disk ("none", "writethrough", "writeback").
    pub fn set_disk_cache(mut self, disk_name: &str, cache: impl Into<String>) -> Self {
        if let Some(d) = self.config.disks.iter_mut().find(|d| d.name == disk_name) {
            d.cache = Some(cache.into());
        }
        self
    }

    /// Set the eviction strategy (e.g., "LiveMigrate").
    pub fn eviction_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.config.eviction_strategy = Some(strategy.into());
        self
    }

    /// Set the termination grace period in seconds.
    pub fn termination_grace_period(mut self, seconds: i64) -> Self {
        self.config.termination_grace_period = Some(seconds);
        self
    }

    /// Set the machine type (e.g., "q35").
    pub fn machine_type(mut self, machine_type: impl Into<String>) -> Self {
        self.config.machine_type = Some(machine_type.into());
        self
    }

    /// Attach a GPU via KubeVirt `domain.devices.gpus` (`device_name` is the
    /// device-plugin resource, e.g. `nvidia.com/gpu`).
    pub fn add_gpu(mut self, name: impl Into<String>, device_name: impl Into<String>) -> Self {
        self.config.gpus.push(crate::config::VmGpuDevice {
            name: name.into(),
            device_name: device_name.into(),
            virtual_gpu_options: None,
        });
        self
    }

    /// Attach a generic host device via `domain.devices.hostDevices`.
    pub fn add_host_device(
        mut self,
        name: impl Into<String>,
        device_name: impl Into<String>,
    ) -> Self {
        self.config.host_devices.push(crate::config::VmHostDevice {
            name: name.into(),
            resource_name: device_name.into(),
        });
        self
    }

    /// Set domain features (ACPI, HyperV, etc.).
    pub fn features(mut self, features: FeaturesConfig) -> Self {
        self.config.features = Some(features);
        self
    }

    /// Set firmware/bootloader configuration.
    pub fn firmware(mut self, firmware: FirmwareConfig) -> Self {
        self.config.firmware = Some(firmware);
        self
    }

    /// Set clock configuration.
    pub fn clock(mut self, clock: ClockConfig) -> Self {
        self.config.clock = Some(clock);
        self
    }

    /// Add a Kubernetes label to the VM.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.labels.insert(key.into(), value.into());
        self
    }

    /// Replace all labels with the given map.
    pub fn labels(mut self, labels: HashMap<String, String>) -> Self {
        self.config.labels = labels;
        self
    }

    /// Add a Kubernetes annotation to the VM.
    pub fn annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.annotations.insert(key.into(), value.into());
        self
    }

    /// Build the VMConfig without validation.
    pub fn build(self) -> VMConfig {
        self.config
    }

    /// Build and validate the VMConfig, returning an error if validation fails.
    ///
    /// Validates VM name, namespace, CPU topology, memory size, disk sizes,
    /// and network interface requirements.
    pub fn build_validated(self) -> anyhow::Result<VMConfig> {
        let config = self.config;
        super::validator::validate_vm_config(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let config = VMConfigBuilder::new("test-vm")
            .namespace("test")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("eth0")
            .label("app", "test")
            .build();

        assert_eq!(config.name, "test-vm");
        assert_eq!(config.namespace, "test");
        assert_eq!(config.cpu.cores, 2);
        assert_eq!(config.memory.size, "4Gi");
        assert_eq!(config.disks.len(), 1);
        assert_eq!(config.interfaces.len(), 1);
        assert_eq!(config.labels.get("app"), Some(&"test".to_string()));
    }

    #[test]
    fn test_default_disk_size_label_skips_container_root() {
        let config = VMConfigBuilder::new("ubuntu-vm")
            .add_container_disk("rootdisk", "quay.io/containerdisks/ubuntu:22.04", 1)
            .add_blank_disk("datadisk", "20Gi", 2)
            .build();
        assert_eq!(config.default_disk_size_label(), "20Gi");
    }

    #[test]
    fn test_builder_multus_network() {
        let config = VMConfigBuilder::new("multus-vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("2Gi")
            .add_multus_network("net1", "my-bridge-net")
            .build();

        assert_eq!(config.interfaces.len(), 1);
        assert_eq!(config.interfaces[0].name, "net1");
        assert_eq!(
            config.interfaces[0].network_type,
            NetworkType::Multus {
                name: "my-bridge-net".to_string()
            }
        );
    }

    #[test]
    fn test_builder_bridge_network() {
        let config = VMConfigBuilder::new("bridge-vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("2Gi")
            .add_bridge_network("br0")
            .build();

        assert_eq!(config.interfaces.len(), 1);
        assert_eq!(config.interfaces[0].name, "br0");
        assert_eq!(config.interfaces[0].network_type, NetworkType::Bridge);
    }

    #[test]
    fn test_builder_multiple_networks() {
        let config = VMConfigBuilder::new("multi-net-vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("2Gi")
            .add_pod_network("eth0")
            .add_multus_network("net1", "storage-net")
            .add_bridge_network("br0")
            .build();

        assert_eq!(config.interfaces.len(), 3);
        assert_eq!(config.interfaces[0].network_type, NetworkType::Pod);
        assert_eq!(
            config.interfaces[1].network_type,
            NetworkType::Multus {
                name: "storage-net".to_string()
            }
        );
        assert_eq!(config.interfaces[2].network_type, NetworkType::Bridge);
    }

    #[test]
    fn test_builder_validated_success() {
        let result = VMConfigBuilder::new("valid-vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .add_blank_disk("root", "20Gi", 1)
            .add_pod_network("eth0")
            .build_validated();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_validated_failure() {
        let result = VMConfigBuilder::new("")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .build_validated();
        assert!(result.is_err());
    }
}
