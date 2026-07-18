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

/// Volume/disk name for the Windows unattend answer-file CD-ROM.
const SYSPREP_VOLUME_NAME: &str = "sysprep";

const QEMU_GUEST_AGENT_CHANNEL: &str = "org.qemu.guest_agent.0";

fn qemu_guest_agent_channel() -> Channel {
    Channel {
        name: "qemu".into(),
        target: ChannelTarget {
            name: QEMU_GUEST_AGENT_CHANNEL.into(),
            channel_type: "virtio".into(),
        },
    }
}

/// When true, emit `domain.devices.channels` for the QEMU guest agent virtio-serial socket.
/// KubeVirt 1.8+ auto-injects this channel; explicit channels fail strict validation unless needed.
fn emit_guest_agent_channels() -> bool {
    matches!(
        std::env::var("VEYRON_EMIT_GUEST_AGENT_CHANNELS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Convert a [`VMConfig`] to a KubeVirt `VirtualMachine` custom resource.
///
/// ```
/// use veyron::{VMConfigBuilder, vm_config_to_kubevirt};
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
    // Golden-image disks additionally emit a VM-owned DataVolume (clone-on-create).
    let mut data_volume_templates: Vec<DataVolumeTemplate> = Vec::new();

    for disk in &config.disks {
        let volume = match &disk.source {
            DiskSource::Blank => Volume {
                name: disk.name.clone(),
                empty_disk: Some(EmptyDiskSource {
                    capacity: disk.size.clone(),
                }),
                ..Default::default()
            },
            DiskSource::PVC { name } => Volume {
                name: disk.name.clone(),
                persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                    claim_name: name.clone(),
                }),
                ..Default::default()
            },
            DiskSource::ContainerDisk { image } => Volume {
                name: disk.name.clone(),
                container_disk: Some(ContainerDiskSource {
                    image: image.clone(),
                    image_pull_policy: Some("IfNotPresent".to_string()),
                }),
                ..Default::default()
            },
            DiskSource::DataVolume { name } => Volume {
                name: disk.name.clone(),
                data_volume: Some(DataVolumeSource { name: name.clone() }),
                ..Default::default()
            },
            DiskSource::GoldenImage {
                name,
                namespace,
                from_pvc,
                storage_class,
                volume_mode,
            } => {
                // The clone gets its own DataVolume, named for the VM+disk so two
                // VMs from the same image never collide.
                let dv_name = format!("{}-{}", config.name, disk.name);

                let (source, source_ref) = if *from_pvc {
                    (
                        Some(DataVolumeSourceSpec {
                            pvc: Some(DataVolumeSourcePVC {
                                namespace: namespace.clone(),
                                name: name.clone(),
                            }),
                            ..Default::default()
                        }),
                        None,
                    )
                } else {
                    (
                        None,
                        Some(DataVolumeSourceRef {
                            kind: "DataSource".to_string(),
                            name: name.clone(),
                            namespace: Some(namespace.clone()),
                        }),
                    )
                };

                let mut requests = BTreeMap::new();
                requests.insert("storage".to_string(), disk.size.clone());

                data_volume_templates.push(DataVolumeTemplate {
                    metadata: Some(ObjectMeta {
                        name: Some(dv_name.clone()),
                        ..Default::default()
                    }),
                    spec: DataVolumeSpec {
                        source,
                        source_ref,
                        storage: Some(DataVolumeStorage {
                            storage_class_name: storage_class.clone(),
                            access_modes: None, // let the StorageProfile decide
                            // Default to Filesystem: golden images are Filesystem, and a
                            // matching-mode clone gets the fast native CSI clone. Leaving
                            // this to the StorageProfile default (often Block) forces a
                            // slow host-assisted copy that fails on some Ceph RBD setups.
                            volume_mode: Some(
                                volume_mode
                                    .clone()
                                    .unwrap_or_else(|| "Filesystem".to_string()),
                            ),
                            resources: Some(DataVolumeResources {
                                requests: Some(requests),
                            }),
                        }),
                    },
                });

                Volume {
                    name: disk.name.clone(),
                    data_volume: Some(DataVolumeSource { name: dv_name }),
                    ..Default::default()
                }
            }
        };
        volumes.push(volume);
    }

    // Windows unattended setup: mount autounattend.xml as a sysprep CD-ROM.
    // Previously this was accepted and silently dropped.
    if let Some(sysprep) = &config.sysprep {
        volumes.push(Volume {
            name: SYSPREP_VOLUME_NAME.to_string(),
            sysprep: Some(SysprepSource {
                secret: sysprep
                    .secret
                    .as_ref()
                    .map(|n| LocalObjectRef { name: n.clone() }),
                config_map: sysprep
                    .config_map
                    .as_ref()
                    .map(|n| LocalObjectRef { name: n.clone() }),
            }),
            ..Default::default()
        });
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
                    ..Default::default()
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
                    ..Default::default()
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
            ..Default::default()
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

    // Sysprep media must be a CD-ROM — Windows Setup only reads autounattend.xml
    // from removable/optical media.
    if config.sysprep.is_some() {
        disks.push(Disk {
            name: SYSPREP_VOLUME_NAME.to_string(),
            disk: None,
            lun: None,
            cdrom: Some(CDROMTarget {
                bus: Some("sata".to_string()),
                readonly: Some(true),
            }),
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

    // Build features. NVIDIA guest drivers refuse to load when they detect
    // KVM, so default the KVM-hidden feature on for NVIDIA GPU passthrough
    // unless the config took an explicit stance.
    let mut features = config.features.as_ref().map(convert_features);
    let wants_nvidia_gpu = config
        .gpus
        .iter()
        .any(|g| g.device_name.starts_with("nvidia.com/"));
    let kvm_hidden_explicit = config
        .features
        .as_ref()
        .is_some_and(|f| f.kvm_hidden.is_some());
    if wants_nvidia_gpu && !kvm_hidden_explicit {
        features
            .get_or_insert(Features {
                acpi: None,
                apic: None,
                hyperv: None,
                kvm: None,
                smm: None,
            })
            .kvm = Some(KVMFeatures { hidden: Some(true) });
    }

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
                    tag: None,
                })
                .collect(),
        )
    };

    let gpus = if config.gpus.is_empty() {
        None
    } else {
        Some(
            config
                .gpus
                .iter()
                .map(|g| Gpu {
                    name: g.name.clone(),
                    device_name: g.device_name.clone(),
                    tag: None,
                    virtual_gpu_options: g.virtual_gpu_options.as_ref().map(|o| {
                        VirtualGpuOptions {
                            display: Some(VgpuDisplayOptions {
                                enabled: o.display,
                                ram_fb: o.ram_fb.map(|e| VgpuRamFb { enabled: Some(e) }),
                            }),
                        }
                    }),
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
        json!([{
            "name": "video0",
            "type": t,
        }])
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

    // inferFromVolume wins over an explicit name: the whole point of inference is to
    // let the golden image decide the size/shape. Ignore-on-missing so a VM created
    // from an unlabeled image still boots (with KubeVirt defaults) rather than failing.
    let to_matcher = |m: &crate::config::VmMatcherRef| {
        if let Some(vol) = m.infer_from_volume.as_ref().filter(|v| !v.is_empty()) {
            InstancetypeMatcher {
                name: String::new(),
                kind: None,
                infer_from_volume: Some(vol.clone()),
                infer_from_volume_failure_policy: Some("Ignore".to_string()),
            }
        } else {
            InstancetypeMatcher {
                name: m.name.clone(),
                kind: m.kind.clone(),
                infer_from_volume: None,
                infer_from_volume_failure_policy: None,
            }
        }
    };
    let instancetype = config.instancetype.as_ref().map(to_matcher);
    let preference = config.preference.as_ref().map(to_matcher);

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
        data_volume_templates: if data_volume_templates.is_empty() {
            None
        } else {
            Some(data_volume_templates)
        },
        template: VirtualMachineInstanceTemplateSpec {
            metadata: Some(ObjectMeta {
                labels: Some(template_labels),
                ..Default::default()
            }),
            spec: VirtualMachineInstanceSpec {
                architecture: None,
                dns_policy: None,
                dns_config: None,
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
                        gpus,
                        watchdog,
                        autoattach_graphics_device: None,
                        autoattach_mem_balloon: if config.disable_balloon {
                            Some(false)
                        } else {
                            None
                        },
                        network_interface_multiqueue: None,
                        video,
                        channels: if emit_guest_agent_channels() {
                            Some(vec![qemu_guest_agent_channel()])
                        } else {
                            None
                        },
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
    fn test_host_device_serializes_kubevirt_device_name() {
        let mut config = VMConfigBuilder::new("gpu-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .build();
        config
            .host_devices
            .push(crate::config::types::VmHostDevice {
                name: "gpu0".to_string(),
                resource_name: "nvidia.com/gpu".to_string(),
            });

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let json = serde_json::to_value(&vm).unwrap();
        let hd = &json["spec"]["template"]["spec"]["domain"]["devices"]["hostDevices"][0];
        // KubeVirt's HostDevice schema field is `deviceName`; `resourceName`
        // would be pruned by the CRD schema and the device would never bind.
        assert_eq!(hd["deviceName"], "nvidia.com/gpu");
        assert!(hd.get("resourceName").is_none());
        assert_eq!(hd["name"], "gpu0");
    }

    #[test]
    fn test_gpu_conversion_defaults_kvm_hidden_for_nvidia() {
        let config = VMConfigBuilder::new("gpu-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .add_gpu("gpu0", "nvidia.com/gpu")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let json = serde_json::to_value(&vm).unwrap();
        let domain = &json["spec"]["template"]["spec"]["domain"];
        let gpu = &domain["devices"]["gpus"][0];
        assert_eq!(gpu["name"], "gpu0");
        assert_eq!(gpu["deviceName"], "nvidia.com/gpu");
        // NVIDIA guest drivers refuse to load when they see KVM; with no
        // explicit stance the converter must hide it.
        assert_eq!(domain["features"]["kvm"]["hidden"], true);
    }

    #[test]
    fn test_gpu_conversion_respects_explicit_kvm_hidden() {
        let mut config = VMConfigBuilder::new("gpu-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .add_gpu("gpu0", "nvidia.com/gpu")
            .build();
        config.features = Some(crate::config::FeaturesConfig {
            acpi: true,
            apic: false,
            hyperv: None,
            kvm_hidden: Some(false),
            smm: None,
        });

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let json = serde_json::to_value(&vm).unwrap();
        assert_eq!(
            json["spec"]["template"]["spec"]["domain"]["features"]["kvm"]["hidden"],
            false
        );
    }

    #[test]
    fn test_non_nvidia_gpu_does_not_touch_kvm_features() {
        let config = VMConfigBuilder::new("gpu-vm")
            .namespace("default")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .add_gpu("gpu0", "amd.com/gpu")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let json = serde_json::to_value(&vm).unwrap();
        assert!(
            json["spec"]["template"]["spec"]["domain"]
                .get("features")
                .is_none()
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
        let first = video.as_array().and_then(|a| a.first()).unwrap_or(video);
        assert_eq!(first.get("type").and_then(|v| v.as_str()), Some("virtio"));
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

    #[test]
    fn test_guest_agent_channels_omitted_by_default() {
        let config = VMConfigBuilder::new("no-channel-vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .add_blank_disk("rootdisk", "10Gi", 1)
            .add_pod_network("default")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let dev = vm.spec.template.spec.domain.devices.as_ref().unwrap();
        assert!(dev.channels.is_none());
    }

    // ── Golden image + sysprep (the Windows catalog path) ────────────────────

    /// A golden-image disk must emit a VM-owned `dataVolumeTemplates` entry, not a
    /// bare reference — two VMs from the same image must never share one disk.
    #[test]
    fn golden_image_disk_emits_owned_data_volume_template() {
        let mut config = VMConfigBuilder::new("win01")
            .namespace("customer-a")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "150Gi", 1)
            .add_pod_network("default")
            .build();
        config.disks[0].source = DiskSource::GoldenImage {
            name: "windows-server-2022".to_string(),
            namespace: "vm-images".to_string(),
            from_pvc: false,
            storage_class: Some("zyvor-rbd-prod".to_string()),
            volume_mode: None,
        };

        let vm = vm_config_to_kubevirt(&config).unwrap();

        let dvts = vm
            .spec
            .data_volume_templates
            .as_ref()
            .expect("golden image must produce a dataVolumeTemplate");
        assert_eq!(dvts.len(), 1);
        let dvt = &dvts[0];
        // Named per VM+disk so a second VM from the same image cannot collide.
        assert_eq!(
            dvt.metadata.as_ref().unwrap().name.as_deref(),
            Some("win01-rootdisk")
        );

        // sourceRef -> DataSource is the stable catalog handle (not an inline PVC).
        let source_ref = dvt.spec.source_ref.as_ref().expect("expected sourceRef");
        assert_eq!(source_ref.kind, "DataSource");
        assert_eq!(source_ref.name, "windows-server-2022");
        assert_eq!(source_ref.namespace.as_deref(), Some("vm-images"));
        assert!(
            dvt.spec.source.is_none(),
            "sourceRef and source are exclusive"
        );

        let storage = dvt.spec.storage.as_ref().unwrap();
        assert_eq!(
            storage.storage_class_name.as_deref(),
            Some("zyvor-rbd-prod")
        );
        assert_eq!(
            storage
                .resources
                .as_ref()
                .unwrap()
                .requests
                .as_ref()
                .unwrap()
                .get("storage"),
            Some(&"150Gi".to_string())
        );

        // The VM's volume must point at the clone it owns.
        let vol = vm
            .spec
            .template
            .spec
            .volumes
            .as_ref()
            .unwrap()
            .iter()
            .find(|v| v.name == "rootdisk")
            .unwrap();
        assert_eq!(vol.data_volume.as_ref().unwrap().name, "win01-rootdisk");
        assert!(
            vol.empty_disk.is_none(),
            "golden image must not boot a blank disk"
        );
    }

    /// `from_pvc` clones straight from a source PVC instead of a DataSource.
    #[test]
    fn golden_image_from_pvc_uses_inline_source() {
        let mut config = VMConfigBuilder::new("win02")
            .namespace("customer-a")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "120Gi", 1)
            .add_pod_network("default")
            .build();
        config.disks[0].source = DiskSource::GoldenImage {
            name: "windows-2022-golden-2026-07".to_string(),
            namespace: "vm-images".to_string(),
            from_pvc: true,
            storage_class: None,
            volume_mode: None,
        };

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let dvt = &vm.spec.data_volume_templates.as_ref().unwrap()[0];

        let pvc = dvt.spec.source.as_ref().unwrap().pvc.as_ref().unwrap();
        assert_eq!(pvc.name, "windows-2022-golden-2026-07");
        assert_eq!(pvc.namespace, "vm-images");
        assert!(dvt.spec.source_ref.is_none());
        // No storage class => inherit the cluster default.
        assert!(
            dvt.spec
                .storage
                .as_ref()
                .unwrap()
                .storage_class_name
                .is_none()
        );
    }

    /// Regression: sysprep used to be accepted and silently dropped. It must reach
    /// the guest as CD-ROM media, or Windows Setup boots to an interactive prompt.
    #[test]
    fn sysprep_secret_emits_cdrom_volume_and_disk() {
        let mut config = VMConfigBuilder::new("win01")
            .namespace("customer-a")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_blank_disk("rootdisk", "150Gi", 1)
            .add_pod_network("default")
            .build();
        config.sysprep = Some(crate::config::SysprepConfig {
            secret: Some("win01-sysprep".to_string()),
            config_map: None,
        });

        let vm = vm_config_to_kubevirt(&config).unwrap();

        let vol = vm
            .spec
            .template
            .spec
            .volumes
            .as_ref()
            .unwrap()
            .iter()
            .find(|v| v.name == "sysprep")
            .expect("sysprep volume must be emitted");
        assert_eq!(
            vol.sysprep.as_ref().unwrap().secret.as_ref().unwrap().name,
            "win01-sysprep"
        );

        // Windows Setup only reads autounattend.xml from optical media.
        let disk = vm
            .spec
            .template
            .spec
            .domain
            .devices
            .as_ref()
            .unwrap()
            .disks
            .as_ref()
            .unwrap()
            .iter()
            .find(|d| d.name == "sysprep")
            .expect("sysprep disk must be attached");
        assert!(disk.cdrom.is_some(), "sysprep must be a CD-ROM, not a disk");
        assert!(disk.disk.is_none());
    }

    /// inferFromVolume must emit as inference (no name), with Ignore failure policy so
    /// a VM off an unlabeled image still boots.
    #[test]
    fn instancetype_infers_from_volume() {
        let mut config = VMConfigBuilder::new("win01")
            .namespace("customer-a")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .add_blank_disk("rootdisk", "50Gi", 1)
            .add_pod_network("default")
            .build();
        config.instancetype = Some(crate::config::VmMatcherRef {
            name: String::new(),
            kind: None,
            infer_from_volume: Some("rootdisk".to_string()),
        });
        config.preference = Some(crate::config::VmMatcherRef {
            name: String::new(),
            kind: None,
            infer_from_volume: Some("rootdisk".to_string()),
        });

        let vm = vm_config_to_kubevirt(&config).unwrap();

        let it = vm.spec.instancetype.as_ref().unwrap();
        assert_eq!(it.infer_from_volume.as_deref(), Some("rootdisk"));
        assert!(it.name.is_empty(), "inference must not also set a name");
        assert_eq!(
            it.infer_from_volume_failure_policy.as_deref(),
            Some("Ignore")
        );

        // Serialized JSON must carry inferFromVolume and omit an empty name.
        let json = serde_json::to_value(&vm.spec).unwrap();
        let itj = &json["instancetype"];
        assert_eq!(itj["inferFromVolume"], "rootdisk");
        assert!(itj.get("name").is_none(), "empty name must be skipped");
    }

    /// A named instancetype still emits by name, no inference.
    #[test]
    fn instancetype_by_name_when_not_inferring() {
        let mut config = VMConfigBuilder::new("vm")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .add_blank_disk("rootdisk", "10Gi", 1)
            .add_pod_network("default")
            .build();
        config.instancetype = Some(crate::config::VmMatcherRef {
            name: "windows-medium".to_string(),
            kind: Some("VirtualMachineClusterInstancetype".to_string()),
            infer_from_volume: None,
        });

        let vm = vm_config_to_kubevirt(&config).unwrap();
        let it = vm.spec.instancetype.as_ref().unwrap();
        assert_eq!(it.name, "windows-medium");
        assert!(it.infer_from_volume.is_none());
    }

    /// No golden image => no dataVolumeTemplates at all (must not emit an empty list).
    #[test]
    fn plain_disk_emits_no_data_volume_templates() {
        let config = VMConfigBuilder::new("plain")
            .namespace("default")
            .cpu(1, 1, 1)
            .memory("1Gi")
            .add_blank_disk("rootdisk", "10Gi", 1)
            .add_pod_network("default")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        assert!(vm.spec.data_volume_templates.is_none());
    }
}
