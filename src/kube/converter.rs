// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::config::{
    BootloaderType, ClockConfig, CloudInitDelivery, DiskDeviceType, DiskSource, FeaturesConfig,
    FirmwareConfig, NetworkType, VMConfig,
};
use crate::kube::types::*;
use anyhow::Result;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde_json::json;
use std::collections::BTreeMap;

/// Convert a [`VMConfig`] to a KubeVirt `VirtualMachine` custom resource.
///
/// ```
/// use vmrogue::{VMConfigBuilder, vm_config_to_kubevirt};
///
/// let config = VMConfigBuilder::new("my-vm")
///     .namespace("default")
///     .cpu(2, 1, 1)
///     .memory("4Gi")
///     .add_blank_disk("root", "20Gi", 1)
///     .add_pod_network("eth0")
///     .build();
///
/// let vm = vm_config_to_kubevirt(&config).unwrap();
/// assert_eq!(vm.spec.running, Some(false));
/// ```
pub fn vm_config_to_kubevirt(config: &VMConfig) -> Result<VirtualMachine> {
    let mut labels: BTreeMap<String, String> = config.labels.clone().into_iter().collect();
    labels.insert("kubevirt.io/vm".to_string(), config.name.clone());

    // Reuse labels for template metadata (includes kubevirt.io/vm for consistency)
    let template_labels = labels.clone();

    let annotations: BTreeMap<String, String> = config.annotations.clone().into_iter().collect();

    let metadata = ObjectMeta {
        name: Some(config.name.clone()),
        namespace: Some(config.namespace.clone()),
        labels: Some(labels),
        annotations: if annotations.is_empty() {
            None
        } else {
            Some(annotations)
        },
        ..Default::default()
    };

    // Build volumes
    let mut volumes = Vec::new();

    for disk in &config.disks {
        let volume = match &disk.source {
            DiskSource::Blank => Volume {
                name: disk.name.clone(),
                empty_disk: Some(EmptyDiskSource {
                    capacity: disk.size.clone(),
                }),
                container_disk: None,
                persistent_volume_claim: None,
                data_volume: None,
                cloud_init_no_cloud: None,
                cloud_init_config_drive: None,
            },
            DiskSource::PVC { name } => Volume {
                name: disk.name.clone(),
                persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                    claim_name: name.clone(),
                }),
                empty_disk: None,
                container_disk: None,
                data_volume: None,
                cloud_init_no_cloud: None,
                cloud_init_config_drive: None,
            },
            DiskSource::ContainerDisk { image } => Volume {
                name: disk.name.clone(),
                container_disk: Some(ContainerDiskSource {
                    image: image.clone(),
                    image_pull_policy: Some("IfNotPresent".to_string()),
                }),
                empty_disk: None,
                persistent_volume_claim: None,
                data_volume: None,
                cloud_init_no_cloud: None,
                cloud_init_config_drive: None,
            },
            DiskSource::DataVolume { name } => Volume {
                name: disk.name.clone(),
                data_volume: Some(DataVolumeSource { name: name.clone() }),
                empty_disk: None,
                container_disk: None,
                persistent_volume_claim: None,
                cloud_init_no_cloud: None,
                cloud_init_config_drive: None,
            },
        };
        volumes.push(volume);
    }

    // Add cloud-init volume if present (`NoCloud` vs config-drive / Cloudbase-Init).
    if let Some(cloud_init) = &config.cloud_init {
        match cloud_init.delivery {
            CloudInitDelivery::NoCloud => {
                volumes.push(Volume {
                    name: "cloudinitdisk".to_string(),
                    cloud_init_no_cloud: Some(CloudInitNoCloudSource {
                        user_data: Some(cloud_init.user_data.clone()),
                        network_data: cloud_init.network_data.clone(),
                    }),
                    cloud_init_config_drive: None,
                    empty_disk: None,
                    container_disk: None,
                    persistent_volume_claim: None,
                    data_volume: None,
                });
            }
            CloudInitDelivery::ConfigDrive => {
                volumes.push(Volume {
                    name: "cloudinitdisk".to_string(),
                    cloud_init_no_cloud: None,
                    cloud_init_config_drive: Some(CloudInitConfigDriveSource {
                        user_data_secret_ref: Some(UserDataSecretRef {
                            name: crate::kube::cloudinit_configdrive_secret_name(&config.name),
                        }),
                    }),
                    empty_disk: None,
                    container_disk: None,
                    persistent_volume_claim: None,
                    data_volume: None,
                });
            }
        }
    }

    for fs in &config.virtio_fs {
        volumes.push(Volume {
            name: fs.name.clone(),
            persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                claim_name: fs.pvc_name.clone(),
            }),
            empty_disk: None,
            container_disk: None,
            data_volume: None,
            cloud_init_no_cloud: None,
            cloud_init_config_drive: None,
        });
    }

    // Build disks
    let mut disks = Vec::new();
    for disk in &config.disks {
        let boot_order = if disk.boot_order > 0 {
            Some(disk.boot_order)
        } else {
            None
        };
        let bus = disk.bus.clone().unwrap_or_else(|| "virtio".to_string());

        match disk.device_type {
            DiskDeviceType::CDROM => {
                disks.push(Disk {
                    name: disk.name.clone(),
                    disk: None,
                    lun: None,
                    cdrom: Some(CDROMTarget {
                        bus: Some(disk.bus.clone().unwrap_or_else(|| "sata".to_string())),
                        readonly: Some(true),
                    }),
                    boot_order,
                    cache: disk.cache.clone(),
                    io: disk.io.clone(),
                    dedicated_io_thread: None,
                    serial: None,
                });
            }
            DiskDeviceType::LUN => {
                disks.push(Disk {
                    name: disk.name.clone(),
                    disk: None,
                    lun: Some(LUNTarget {
                        bus: Some(disk.bus.clone().unwrap_or_else(|| "scsi".to_string())),
                        readonly: None,
                    }),
                    cdrom: None,
                    boot_order,
                    cache: disk.cache.clone(),
                    io: disk.io.clone(),
                    dedicated_io_thread: None,
                    serial: None,
                });
            }
            _ => {
                disks.push(Disk {
                    name: disk.name.clone(),
                    disk: Some(DiskTarget {
                        bus: Some(bus),
                        readonly: None,
                    }),
                    lun: None,
                    cdrom: None,
                    boot_order,
                    cache: disk.cache.clone(),
                    io: disk.io.clone(),
                    dedicated_io_thread: None,
                    serial: None,
                });
            }
        }
    }

    // Add cloud-init disk if present (same disk attachment for NoCloud or config-drive volume).
    if config.cloud_init.is_some() {
        disks.push(Disk {
            name: "cloudinitdisk".to_string(),
            disk: Some(DiskTarget {
                bus: Some("virtio".to_string()),
                readonly: None,
            }),
            lun: None,
            cdrom: None,
            boot_order: None,
            cache: None,
            io: None,
            dedicated_io_thread: None,
            serial: None,
        });
    }

    // Build interfaces
    let mut interfaces = Vec::new();
    for iface in &config.interfaces {
        let (masquerade, bridge, sriov) = match &iface.network_type {
            NetworkType::Pod => (Some(BTreeMap::new()), None, None),
            NetworkType::Bridge => (None, Some(BTreeMap::new()), None),
            NetworkType::Multus { .. } | NetworkType::Ovn { .. } => {
                (None, Some(BTreeMap::new()), None)
            }
            NetworkType::SRIOV { .. } => (None, None, Some(BTreeMap::new())),
        };

        interfaces.push(Interface {
            name: iface.name.clone(),
            model: Some(iface.model.clone()),
            mac_address: iface.mac_address.clone(),
            masquerade,
            bridge,
            sriov,
            ports: None,
            boot_order: None,
        });
    }

    // Build networks
    let mut networks = Vec::new();
    for iface in &config.interfaces {
        let network = match &iface.network_type {
            NetworkType::Pod => Network {
                name: iface.name.clone(),
                pod: Some(BTreeMap::new()),
                multus: None,
            },
            NetworkType::Bridge => Network {
                name: iface.name.clone(),
                pod: Some(BTreeMap::new()),
                multus: None,
            },
            NetworkType::Multus { name } | NetworkType::Ovn { name } => Network {
                name: iface.name.clone(),
                pod: None,
                multus: Some(MultusNetwork {
                    network_name: name.clone(),
                }),
            },
            NetworkType::SRIOV { name } => Network {
                name: iface.name.clone(),
                pod: None,
                multus: Some(MultusNetwork {
                    network_name: name.clone(),
                }),
            },
        };
        networks.push(network);
    }

    // Build resource requests
    let mut requests = BTreeMap::new();
    requests.insert("memory".to_string(), config.memory.size.clone());

    // Build features
    let features = config.features.as_ref().map(convert_features);

    // Build clock
    let clock = config.clock.as_ref().map(convert_clock);

    // Build firmware
    let firmware = config.firmware.as_ref().map(convert_firmware);

    // Build machine type
    let machine = config.machine_type.as_ref().map(|t| Machine {
        machine_type: Some(t.clone()),
    });

    // Build memory with hugepages
    let memory_hugepages = config
        .memory
        .hugepages_page_size
        .as_ref()
        .map(|ps| Hugepages {
            page_size: Some(ps.clone()),
        });

    // Build devices with TPM, RNG, inputs
    let tpm = if config.enable_tpm {
        Some(TPMDevice {})
    } else {
        None
    };
    let rng = if config.enable_rng {
        Some(RNGDevice {})
    } else {
        None
    };
    let inputs = if config.usb_tablet {
        Some(vec![InputDevice {
            input_type: "tablet".to_string(),
            name: "tablet0".to_string(),
            bus: Some("usb".to_string()),
        }])
    } else {
        None
    };

    let watchdog = config.watchdog.as_ref().map(|w| WatchdogDevice {
        model: Some(w.model.clone()),
        action: Some(w.action.clone()),
    });

    let host_devices = if config.host_devices.is_empty() {
        None
    } else {
        Some(
            config
                .host_devices
                .iter()
                .map(|h| HostDevice {
                    name: h.name.clone(),
                    resource_name: h.resource_name.clone(),
                })
                .collect(),
        )
    };

    let filesystems = if config.virtio_fs.is_empty() {
        None
    } else {
        Some(
            config
                .virtio_fs
                .iter()
                .map(|f| Filesystem {
                    name: f.name.clone(),
                    virtiofs: Some(VirtiofsSource::default()),
                })
                .collect(),
        )
    };

    let video = config.kubevirt_video_type.as_ref().map(|t| {
        vec![json!({
            "name": "video0",
            "type": t,
        })]
    });

    let (
        sched_node_selector,
        priority_class_name,
        affinity,
        tolerations,
        topology_spread_constraints,
        scheduler_name,
    ) = if let Some(ref sch) = config.scheduling {
        (
            sch.node_selector
                .as_ref()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            sch.priority_class_name.clone(),
            sch.affinity.clone(),
            sch.tolerations.clone(),
            sch.topology_spread_constraints.clone(),
            sch.scheduler_name.clone(),
        )
    } else {
        (None, None, None, None, None, None)
    };

    let access_credentials = if config.access_credentials.is_empty() {
        None
    } else {
        Some(config.access_credentials.clone())
    };

    let instancetype = config.instancetype.as_ref().map(|i| InstancetypeMatcher {
        name: i.name.clone(),
        kind: i.kind.clone(),
    });
    let preference = config.preference.as_ref().map(|i| InstancetypeMatcher {
        name: i.name.clone(),
        kind: i.kind.clone(),
    });

    let (running_field, run_strategy_field) = match &config.run_strategy {
        Some(rs) => (None, Some(rs.clone())),
        None => (Some(false), None),
    };

    let termination_grace_period = config.termination_grace_period.unwrap_or(30);

    let spec = VirtualMachineSpec {
        running: running_field,
        run_strategy: run_strategy_field,
        instancetype,
        preference,
        template: VirtualMachineInstanceTemplateSpec {
            metadata: Some(ObjectMeta {
                labels: Some(template_labels),
                ..Default::default()
            }),
            spec: VirtualMachineInstanceSpec {
                domain: DomainSpec {
                    resources: ResourceRequirements {
                        requests: Some(requests),
                        limits: None,
                    },
                    cpu: Some(CPU {
                        cores: Some(config.cpu.cores),
                        sockets: Some(config.cpu.sockets),
                        threads: Some(config.cpu.threads),
                        model: config.cpu.model.clone(),
                        dedicated_cpu_placement: config.cpu.dedicated_cpu_placement,
                        isolate_emulator_thread: config.cpu.isolate_emulator_thread,
                        numa: None,
                        realtime: None,
                    }),
                    memory: Some(Memory {
                        guest: Some(config.memory.size.clone()),
                        hugepages: memory_hugepages,
                        max_guest: config.memory.max_guest.clone(),
                    }),
                    devices: Some(Devices {
                        disks: Some(disks),
                        interfaces: Some(interfaces),
                        tpm,
                        rng,
                        inputs,
                        host_devices,
                        watchdog,
                        autoattach_graphics_device: None,
                        autoattach_mem_balloon: if config.disable_balloon {
                            Some(false)
                        } else {
                            None
                        },
                        network_interface_multiqueue: None,
                        video,
                    }),
                    filesystems,
                    features,
                    clock,
                    firmware,
                    machine,
                    io_threads_policy: config.io_threads_policy.clone(),
                },
                volumes: Some(volumes),
                networks: Some(networks),
                termination_grace_period_seconds: Some(termination_grace_period),
                eviction_strategy: config.eviction_strategy.clone(),
                node_selector: sched_node_selector,
                priority_class_name,
                affinity,
                tolerations,
                topology_spread_constraints,
                scheduler_name,
                access_credentials,
            },
        },
    };

    Ok(VirtualMachine {
        metadata,
        spec,
        status: None,
    })
}

fn convert_features(cfg: &FeaturesConfig) -> Features {
    let hyperv = cfg.hyperv.as_ref().map(|hv| {
        let spinlocks = hv.spinlocks.map(|retries| SpinlockRetries {
            spinlocks: Some(retries),
        });

        HyperVFeatures {
            relaxed: Some(FeatureState {
                enabled: Some(hv.relaxed),
            }),
            vapic: Some(FeatureState {
                enabled: Some(hv.vapic),
            }),
            spinlocks,
            vpindex: Some(FeatureState {
                enabled: Some(hv.vpindex),
            }),
            runtime: Some(FeatureState {
                enabled: Some(hv.runtime),
            }),
            synic: Some(FeatureState {
                enabled: Some(hv.synic),
            }),
            stimer: Some(STimer {
                enabled: Some(hv.stimer),
                direct: None,
            }),
            reset: Some(FeatureState {
                enabled: Some(hv.reset),
            }),
            frequencies: Some(FeatureState {
                enabled: Some(hv.frequencies),
            }),
            reenlightenment: Some(FeatureState {
                enabled: Some(hv.reenlightenment),
            }),
            tlbflush: Some(FeatureState {
                enabled: Some(hv.tlbflush),
            }),
            ipi: Some(FeatureState {
                enabled: Some(hv.ipi),
            }),
            evmcs: None,
        }
    });

    let kvm = cfg.kvm_hidden.map(|hidden| KVMFeatures {
        hidden: Some(hidden),
    });

    let smm = cfg.smm.map(|enabled| SMMFeatures {
        enabled: Some(enabled),
    });

    Features {
        acpi: Some(FeatureEnabled {
            enabled: Some(cfg.acpi),
        }),
        apic: if cfg.apic {
            Some(FeatureEnabled {
                enabled: Some(true),
            })
        } else {
            None
        },
        hyperv,
        kvm,
        smm,
    }
}

fn convert_clock(cfg: &ClockConfig) -> Clock {
    let timer = cfg.timers.as_ref().map(|t| ClockTimer {
        hpet: t.hpet_present.map(|present| TimerConfig {
            present: Some(present),
            tick_policy: None,
        }),
        pit: t.pit_tick_policy.as_ref().map(|policy| TimerConfig {
            present: Some(true),
            tick_policy: Some(policy.clone()),
        }),
        rtc: t.rtc_tick_policy.as_ref().map(|policy| RTCTimer {
            present: Some(true),
            tick_policy: Some(policy.clone()),
            track: Some("guest".to_string()),
        }),
        hyperv: t.hyperv_present.map(|present| TimerConfig {
            present: Some(present),
            tick_policy: None,
        }),
        kvm: None,
    });

    let utc = if cfg.utc {
        Some(UTCClock {
            offset_seconds: None,
        })
    } else {
        None
    };

    Clock {
        utc,
        timezone: cfg.timezone.clone(),
        timer,
    }
}

fn convert_firmware(cfg: &FirmwareConfig) -> Firmware {
    let bootloader = match &cfg.bootloader {
        BootloaderType::BIOS => Bootloader {
            bios: Some(BIOSBootloader {
                use_serial: Some(true),
            }),
            efi: None,
        },
        BootloaderType::EFI {
            secure_boot,
            persistent,
        } => Bootloader {
            bios: None,
            efi: Some(EFIBootloader {
                secure_boot: Some(*secure_boot),
                persistent: Some(*persistent),
            }),
        },
    };

    Firmware {
        bootloader: Some(bootloader),
        uuid: None,
        serial: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfigBuilder;

    #[test]
    fn test_basic_conversion() {
        let config = VMConfigBuilder::new("test-vm")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();

        assert_eq!(vm.metadata.name, Some("test-vm".to_string()));
        assert_eq!(vm.metadata.namespace, Some("default".to_string()));
        assert_eq!(vm.spec.running, Some(false));

        let vmi_spec = &vm.spec.template.spec;
        assert_eq!(vmi_spec.domain.cpu.as_ref().unwrap().cores, Some(2));
        assert_eq!(
            vmi_spec.domain.memory.as_ref().unwrap().guest,
            Some("4Gi".to_string())
        );
    }

    #[test]
    fn test_kubevirt_video_type_conversion() {
        let config = VMConfigBuilder::new("vid-vm")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .kubevirt_video_type("virtio")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let dev = vm.spec.template.spec.domain.devices.as_ref().unwrap();
        let video = dev.video.as_ref().expect("video");
        assert_eq!(video.len(), 1);
        assert_eq!(
            video[0].get("type").and_then(|v| v.as_str()),
            Some("virtio")
        );
    }

    #[test]
    fn test_container_disk_conversion() {
        let config = VMConfigBuilder::new("fedora-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_container_disk("rootdisk", "quay.io/containerdisks/fedora:39", 1)
            .add_pod_network("default")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();

        assert_eq!(volumes.len(), 1);
        assert!(volumes[0].container_disk.is_some());
        assert_eq!(
            volumes[0].container_disk.as_ref().unwrap().image,
            "quay.io/containerdisks/fedora:39"
        );
    }

    #[test]
    fn test_cloud_init_conversion() {
        let config = VMConfigBuilder::new("ubuntu-vm")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .cloud_init("#cloud-config\nuser: ubuntu")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();

        // Should have rootdisk + cloudinitdisk
        assert_eq!(volumes.len(), 2);

        let cloud_init_vol = volumes.iter().find(|v| v.name == "cloudinitdisk");
        assert!(cloud_init_vol.is_some());
        assert!(cloud_init_vol.unwrap().cloud_init_no_cloud.is_some());
    }
}
