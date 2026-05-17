use crate::config::{InterfaceConfig, NetworkType, VMConfig, VMConfigBuilder, validate_vm_config};
use crate::kube::types::VirtualMachine;
use crate::output::{OutputFormat, format_output};
use crate::templates::TEMPLATES;
use crate::tui::colors::cli as color;
use anyhow::{Result, anyhow};

/// Convert a KubeVirt `VirtualMachine` into a [`VMConfig`].
///
/// **Limits:** userdata stored only in a referenced **Secret** (config-drive delivery) cannot be
/// reconstructed from the VM spec; round-trips for that path require reading the Secret separately.
fn vm_to_config(vm: &VirtualMachine, namespace: &str) -> VMConfig {
    let name = vm.metadata.name.clone().unwrap_or_default();
    let spec = &vm.spec.template.spec;
    let domain = &spec.domain;

    let cpu_cores = domain.cpu.as_ref().and_then(|c| c.cores).unwrap_or(1);
    let cpu_sockets = domain.cpu.as_ref().and_then(|c| c.sockets).unwrap_or(1);
    let cpu_threads = domain.cpu.as_ref().and_then(|c| c.threads).unwrap_or(1);

    let memory = domain
        .memory
        .as_ref()
        .and_then(|m| m.guest.as_deref())
        .or_else(|| {
            domain
                .resources
                .requests
                .as_ref()
                .and_then(|r| r.get("memory"))
                .map(|s| s.as_str())
        })
        .unwrap_or("2Gi")
        .to_string();

    let mut builder = VMConfigBuilder::new(&name)
        .namespace(namespace)
        .cpu(cpu_cores, cpu_sockets, cpu_threads)
        .memory(&memory);

    // Extract CPU model and advanced settings
    if let Some(ref cpu) = domain.cpu {
        if let Some(ref model) = cpu.model {
            builder = builder.cpu_model(model);
        }
        if let Some(true) = cpu.dedicated_cpu_placement {
            builder = builder.dedicated_cpu_placement(true);
        }
        if let Some(true) = cpu.isolate_emulator_thread {
            builder = builder.isolate_emulator_thread(true);
        }
    }

    // Extract memory hugepages and max_guest
    if let Some(ref mem) = domain.memory {
        if let Some(ref hp) = mem.hugepages {
            if let Some(ref ps) = hp.page_size {
                builder = builder.hugepages(ps);
            }
        }
        if let Some(ref mg) = mem.max_guest {
            builder = builder.max_guest_memory(mg);
        }
    }

    // Extract TPM and RNG from devices
    if let Some(ref devices) = domain.devices {
        if devices.tpm.is_some() {
            builder = builder.enable_tpm();
        }
        if devices.rng.is_some() {
            builder = builder.enable_rng();
        }
    }

    // Extract eviction strategy
    if let Some(ref eviction) = spec.eviction_strategy {
        builder = builder.eviction_strategy(eviction);
    }

    // Extract termination grace period
    if let Some(tgp) = spec.termination_grace_period_seconds {
        builder = builder.termination_grace_period(tgp);
    }

    // Extract machine type
    if let Some(ref machine) = domain.machine {
        if let Some(ref mt) = machine.machine_type {
            builder = builder.machine_type(mt);
        }
    }

    // Extract features (preserve HyperV enlightenments, ACPI, etc.)
    if let Some(ref features) = domain.features {
        let acpi = features
            .acpi
            .as_ref()
            .and_then(|a| a.enabled)
            .unwrap_or(true);
        let apic = features
            .apic
            .as_ref()
            .and_then(|a| a.enabled)
            .unwrap_or(false);

        let hyperv = features.hyperv.as_ref().map(|hv| {
            let enabled = |fs: &Option<crate::kube::types::FeatureState>| -> bool {
                fs.as_ref().and_then(|f| f.enabled).unwrap_or(false)
            };
            crate::config::HyperVConfig {
                relaxed: enabled(&hv.relaxed),
                vapic: enabled(&hv.vapic),
                spinlocks: hv.spinlocks.as_ref().and_then(|s| s.spinlocks),
                vpindex: enabled(&hv.vpindex),
                runtime: enabled(&hv.runtime),
                synic: enabled(&hv.synic),
                stimer: hv.stimer.as_ref().and_then(|s| s.enabled).unwrap_or(false),
                reset: enabled(&hv.reset),
                frequencies: enabled(&hv.frequencies),
                reenlightenment: enabled(&hv.reenlightenment),
                tlbflush: enabled(&hv.tlbflush),
                ipi: enabled(&hv.ipi),
            }
        });

        let kvm_hidden = features.kvm.as_ref().and_then(|k| k.hidden);
        let smm = features.smm.as_ref().and_then(|s| s.enabled);

        builder = builder.features(crate::config::FeaturesConfig {
            acpi,
            apic,
            hyperv,
            kvm_hidden,
            smm,
        });
    }

    // Extract firmware configuration
    if let Some(ref firmware) = domain.firmware {
        if let Some(ref bootloader) = firmware.bootloader {
            let fw = if let Some(ref efi) = bootloader.efi {
                crate::config::FirmwareConfig {
                    bootloader: crate::config::BootloaderType::EFI {
                        secure_boot: efi.secure_boot.unwrap_or(false),
                        persistent: efi.persistent.unwrap_or(true),
                    },
                }
            } else {
                crate::config::FirmwareConfig {
                    bootloader: crate::config::BootloaderType::BIOS,
                }
            };
            builder = builder.firmware(fw);
        }
    }

    // Extract clock configuration
    if let Some(ref clock) = domain.clock {
        let utc = clock.utc.is_some();
        let timezone = clock.timezone.clone();
        let timers = clock.timer.as_ref().map(|t| crate::config::TimersConfig {
            hpet_present: t.hpet.as_ref().and_then(|h| h.present),
            pit_tick_policy: t.pit.as_ref().and_then(|p| p.tick_policy.clone()),
            rtc_tick_policy: t.rtc.as_ref().and_then(|r| r.tick_policy.clone()),
            hyperv_present: t.hyperv.as_ref().and_then(|h| h.present),
        });
        builder = builder.clock(crate::config::ClockConfig {
            utc,
            timezone,
            timers,
        });
    }

    // Extract disks from volumes
    if let Some(ref volumes) = spec.volumes {
        for (i, vol) in volumes.iter().enumerate() {
            let skip_virtiofs = domain
                .filesystems
                .as_ref()
                .map(|fss| {
                    fss.iter()
                        .any(|f| f.name == vol.name && f.virtiofs.is_some())
                })
                .unwrap_or(false);
            if skip_virtiofs {
                continue;
            }
            if let Some(ref container_disk) = vol.container_disk {
                builder = builder.add_container_disk(
                    &vol.name,
                    &container_disk.image,
                    (i as u32).saturating_add(1),
                );
            } else if let Some(ref pvc) = vol.persistent_volume_claim {
                builder = builder.add_disk(crate::config::DiskConfig {
                    name: vol.name.clone(),
                    size: "0".to_string(),
                    storage_class: None,
                    boot_order: (i as u32).saturating_add(1),
                    source: crate::config::DiskSource::PVC {
                        name: pvc.claim_name.clone(),
                    },
                    device_type: crate::config::DiskDeviceType::default(),
                    bus: None,
                    cache: None,
                    io: None,
                });
            } else if let Some(ref empty) = vol.empty_disk {
                builder = builder.add_blank_disk(
                    &vol.name,
                    &empty.capacity,
                    (i as u32).saturating_add(1),
                );
            } else if let Some(ref cloud_init) = vol.cloud_init_no_cloud {
                if let Some(ref user_data) = cloud_init.user_data {
                    builder = builder.cloud_init(user_data);
                }
            }
        }
    }

    // Extract network interfaces
    if let Some(ref devices) = domain.devices {
        if let Some(ref ifaces) = devices.interfaces {
            for iface in ifaces {
                let is_sriov = iface.sriov.is_some();
                let network_type = if let Some(ref networks) = spec.networks {
                    networks
                        .iter()
                        .find(|n| n.name == iface.name)
                        .map(|n| {
                            if is_sriov {
                                if let Some(ref multus) = n.multus {
                                    NetworkType::SRIOV {
                                        name: multus.network_name.clone(),
                                    }
                                } else {
                                    NetworkType::SRIOV {
                                        name: iface.name.clone(),
                                    }
                                }
                            } else if let Some(ref multus) = n.multus {
                                NetworkType::Multus {
                                    name: multus.network_name.clone(),
                                }
                            } else if n.pod.is_some() {
                                NetworkType::Pod
                            } else {
                                NetworkType::Bridge
                            }
                        })
                        .unwrap_or(NetworkType::Pod)
                } else {
                    NetworkType::Pod
                };

                builder = builder.add_interface(InterfaceConfig {
                    name: iface.name.clone(),
                    network: iface.name.clone(),
                    model: iface.model.clone().unwrap_or_else(|| "virtio".to_string()),
                    network_type,
                    mac_address: iface.mac_address.clone(),
                });
            }
        }
    }

    // Extract labels
    if let Some(ref labels) = vm.metadata.labels {
        for (k, v) in labels {
            builder = builder.label(k, v);
        }
    }

    let mut config = builder.build();
    config.run_strategy = vm.spec.run_strategy.clone();
    config.instancetype = vm
        .spec
        .instancetype
        .as_ref()
        .map(|m| crate::config::VmMatcherRef {
            name: m.name.clone(),
            kind: m.kind.clone(),
        });
    config.preference = vm
        .spec
        .preference
        .as_ref()
        .map(|m| crate::config::VmMatcherRef {
            name: m.name.clone(),
            kind: m.kind.clone(),
        });
    if spec.node_selector.is_some()
        || spec.affinity.is_some()
        || spec.tolerations.is_some()
        || spec.topology_spread_constraints.is_some()
        || spec.priority_class_name.is_some()
        || spec.scheduler_name.is_some()
    {
        config.scheduling = Some(crate::config::VmScheduling {
            node_selector: spec
                .node_selector
                .as_ref()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            affinity: spec.affinity.clone(),
            tolerations: spec.tolerations.clone(),
            topology_spread_constraints: spec.topology_spread_constraints.clone(),
            priority_class_name: spec.priority_class_name.clone(),
            scheduler_name: spec.scheduler_name.clone(),
        });
    }
    config.access_credentials = spec.access_credentials.clone().unwrap_or_default();
    if let Some(ref dev) = domain.devices {
        if let Some(ref wd) = dev.watchdog {
            if let (Some(model), Some(action)) = (wd.model.as_ref(), wd.action.as_ref()) {
                config.watchdog = Some(crate::config::VmWatchdogConfig {
                    model: model.clone(),
                    action: action.clone(),
                });
            }
        }
        if let Some(ref hosts) = dev.host_devices {
            config.host_devices = hosts
                .iter()
                .map(|h| crate::config::VmHostDevice {
                    name: h.name.clone(),
                    resource_name: h.resource_name.clone(),
                })
                .collect();
        }
        if let Some(ref videos) = dev.video {
            if let Some(first) = videos.first() {
                if let Some(t) = first.get("type").and_then(|v| v.as_str()) {
                    config.kubevirt_video_type = Some(t.to_string());
                } else if let Some(m) = first.get("model").and_then(|v| v.as_str()) {
                    config.kubevirt_video_type = Some(m.to_string());
                }
            }
        }
    }
    if let Some(ref fss) = domain.filesystems {
        for fs in fss {
            if fs.virtiofs.is_some() {
                if let Some(ref vols) = spec.volumes {
                    if let Some(vol) = vols.iter().find(|v| v.name == fs.name) {
                        if let Some(ref pvc) = vol.persistent_volume_claim {
                            config.virtio_fs.push(crate::config::VmVirtioFs {
                                name: fs.name.clone(),
                                pvc_name: pvc.claim_name.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    config
}

/// CLI overrides that can be applied on top of a loaded VMConfig.
#[derive(Default)]
pub struct CliOverrides {
    pub cpus: Option<u32>,
    pub memory: Option<String>,
    pub disk_size: Option<String>,
    pub storage_class: Option<String>,
    pub container_disk: Option<String>,
    pub cloud_init: Option<String>,
}

/// Apply CLI overrides on top of a loaded VMConfig.
fn apply_cli_overrides(config: &mut VMConfig, overrides: CliOverrides) -> Result<()> {
    let CliOverrides {
        cpus,
        memory,
        disk_size,
        storage_class,
        container_disk,
        cloud_init,
    } = overrides;
    if let Some(cpus) = cpus {
        config.cpu.cores = cpus;
    }
    if let Some(memory) = memory {
        config.memory.size = memory;
    }
    if let Some(disk_size) = disk_size {
        if let Some(disk) = config.disks.first_mut() {
            disk.size = disk_size;
        } else {
            log::warn!("--disk-size specified but no disks configured; adding a blank disk");
            config.disks.push(crate::config::DiskConfig {
                name: "rootdisk".to_string(),
                size: disk_size,
                storage_class: None,
                boot_order: 1,
                source: crate::config::DiskSource::Blank,
                device_type: crate::config::DiskDeviceType::default(),
                bus: None,
                cache: None,
                io: None,
            });
        }
    }
    if let Some(ref sc) = storage_class {
        for disk in &mut config.disks {
            disk.storage_class = Some(sc.clone());
        }
    }
    if let Some(ref image) = container_disk {
        let disk = crate::config::DiskConfig {
            name: "containerdisk".to_string(),
            size: "0".to_string(),
            storage_class: None,
            boot_order: 1,
            source: crate::config::DiskSource::ContainerDisk {
                image: image.clone(),
            },
            device_type: crate::config::DiskDeviceType::default(),
            bus: None,
            cache: None,
            io: None,
        };
        if config.disks.is_empty() {
            config.disks.push(disk);
        } else {
            config.disks.insert(0, disk);
            for disk in config.disks.iter_mut().skip(1) {
                disk.boot_order = disk.boot_order.saturating_add(1);
            }
        }
    }
    if let Some(cloud_init_file) = cloud_init {
        let user_data = std::fs::read_to_string(&cloud_init_file)?;
        config.cloud_init = Some(crate::config::CloudInitConfig {
            user_data,
            network_data: None,
            delivery: crate::config::CloudInitDelivery::NoCloud,
        });
    }
    Ok(())
}

fn load_or_create_config(
    name: &str,
    namespace: &str,
    template: Option<String>,
    from_file: Option<String>,
) -> Result<VMConfig> {
    use std::fs;

    if let Some(file) = from_file {
        let content = fs::read_to_string(&file)?;
        let mut config: VMConfig = if file.ends_with(".json") {
            serde_json::from_str(&content)?
        } else {
            serde_yml::from_str(&content)?
        };
        config.name = name.to_string();
        config.namespace = namespace.to_string();
        Ok(config)
    } else if let Some(template_name) = template {
        let mut config = TEMPLATES.get(&template_name).ok_or_else(|| {
            let available = TEMPLATES.list();
            let mut msg = format!("Template not found: '{}'", template_name);
            // Suggest similar template names
            let suggestions: Vec<_> = available
                .iter()
                .filter(|t| {
                    t.contains(&template_name)
                        || template_name.contains(t.as_str())
                        || t.starts_with(template_name.split('-').next().unwrap_or(""))
                })
                .take(5)
                .collect();
            if !suggestions.is_empty() {
                msg.push_str("\n  Did you mean: ");
                msg.push_str(
                    &suggestions
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            msg.push_str("\n  Run 'vmrogue templates' to see all available templates.");
            anyhow!(msg)
        })?;
        config.name = name.to_string();
        config.namespace = namespace.to_string();
        Ok(config)
    } else {
        // Create a basic default VM
        Ok(VMConfigBuilder::new(name)
            .namespace(namespace)
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .build())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_create(
    name: String,
    template: Option<String>,
    from_file: Option<String>,
    cpus: Option<u32>,
    memory: Option<String>,
    disk_size: Option<String>,
    storage_class: Option<String>,
    container_disk: Option<String>,
    cloud_init: Option<String>,
    no_internet: bool,
    dry_run: bool,
    output: String,
    namespace: &str,
) -> Result<()> {
    use crate::kube;

    let mut config = load_or_create_config(&name, namespace, template, from_file)?;
    apply_cli_overrides(
        &mut config,
        CliOverrides {
            cpus,
            memory,
            disk_size,
            storage_class,
            container_disk,
            cloud_init,
        },
    )?;
    if no_internet {
        config.allow_internet = false;
    }
    validate_vm_config(&config)?;

    let format = OutputFormat::parse_format(&output)
        .ok_or_else(|| anyhow!("Invalid output format: {}", output))?;

    if dry_run {
        println!("# Dry run - VM manifest:");
        let kubevirt_vm = kube::vm_config_to_kubevirt(&config)?;
        let output = format_output(&kubevirt_vm, format)?;
        println!("{}", output);
    } else {
        log::info!("Creating VM: {}", name);

        // Create the VM via Kubernetes API
        let sp = crate::output::spinner::spinner(&format!("Creating VM '{}'...", name));
        match kube::KubeClient::new().await {
            Ok(client) => match client.create_vm(&config).await {
                Ok(_vm) => {
                    sp.finish_and_clear();
                    println!(
                        "{}",
                        color::success(&format!("VM '{}' created successfully", name))
                    );
                    println!("  Namespace: {}", color::namespace(&config.namespace));
                    println!(
                        "  Status: {} (use '{}' to start)",
                        color::vm_status("Stopped"),
                        color::command(&format!("vmrogue start {}", name))
                    );
                }
                Err(e) => {
                    sp.finish_and_clear();
                    return Err(anyhow!("Failed to create VM: {}", e));
                }
            },
            Err(e) => {
                sp.finish_and_clear();
                return Err(anyhow!("Failed to connect to Kubernetes: {}", e));
            }
        }
    }
    Ok(())
}

pub async fn handle_list(all_namespaces: bool, output: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output;

    let client = kube::KubeClient::new().await?;

    let vms = if all_namespaces {
        client.list_all_vms().await?
    } else {
        client.list_vms(namespace).await?
    };

    if vms.is_empty() {
        use crate::tui::colors::cli as color;
        if all_namespaces {
            println!("{}", color::muted("No VMs found in any namespace."));
        } else {
            println!(
                "{}",
                color::muted(&format!("No VMs found in namespace '{}'.", namespace))
            );
            println!(
                "  {}",
                color::muted(
                    "Use --all-namespaces to search everywhere, or --namespace to change."
                )
            );
        }
        return Ok(());
    }

    match output.as_str() {
        "yaml" => {
            let yaml = output::to_yaml(&vms)?;
            println!("{}", yaml);
        }
        "json" => {
            let json = output::to_json(&vms)?;
            println!("{}", json);
        }
        _ => {
            use crate::output::table::CliTable;
            use crate::tui::colors::vm_status_symbol;

            let mut table = CliTable::new(vec!["NAME", "NAMESPACE", "STATUS", "RUNNING"]);
            for vm in &vms {
                let name = vm.metadata.name.as_deref().unwrap_or("N/A");
                let ns = vm.metadata.namespace.as_deref().unwrap_or("N/A");
                let status = vm
                    .status
                    .as_ref()
                    .and_then(|s| s.printable_status.as_deref())
                    .unwrap_or("Unknown");
                let running = if vm.spec.running.unwrap_or(false) {
                    color::success("Yes")
                } else {
                    color::muted("No")
                };
                table.add_row(vec![
                    color::vm_name(name),
                    color::namespace(ns),
                    format!("{} {}", vm_status_symbol(status), color::vm_status(status)),
                    running,
                ]);
            }
            table.print();
        }
    }
    Ok(())
}

pub async fn handle_get(name: String, output: String, namespace: &str) -> Result<()> {
    use crate::kube;

    let client = kube::KubeClient::new().await?;
    let vm = client.get_vm(namespace, &name).await?;

    let format = OutputFormat::parse_format(&output)
        .ok_or_else(|| anyhow!("Invalid output format: {}", output))?;

    let formatted = format_output(&vm, format)?;
    println!("{}", formatted);
    Ok(())
}

pub async fn handle_delete(name: String, yes: bool, namespace: &str) -> Result<()> {
    use crate::kube;

    let client = kube::KubeClient::new().await?;

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Delete VM '{}'?", name))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("{}", color::muted("Cancelled"));
            return Ok(());
        }
    }

    let sp = crate::output::spinner::spinner(&format!("Deleting VM '{}'...", name));
    client.delete_vm(namespace, &name).await?;
    sp.finish_and_clear();
    println!(
        "{}",
        color::success(&format!("VM '{}' deleted successfully", name))
    );
    Ok(())
}

pub async fn handle_start(name: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    let sp = spinner(&format!("Starting VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client.start_vm(namespace, &name).await?;
    sp.set_message(format!("Waiting for VM '{}' to become running...", name));

    // Poll until actually running (up to 30s)
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        match client.is_running(namespace, &name).await {
            Ok(true) => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::success(&format!("VM '{}' is now running", name))
                );
                return Ok(());
            }
            Ok(false) if tokio::time::Instant::now() >= deadline => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::warning(&format!(
                        "VM '{}' start initiated but not yet running (may take longer)",
                        name
                    ))
                );
                return Ok(());
            }
            Ok(false) => continue,
            Err(_) if tokio::time::Instant::now() >= deadline => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::warning(&format!(
                        "VM '{}' start initiated (could not confirm status)",
                        name
                    ))
                );
                return Ok(());
            }
            Err(_) => continue,
        }
    }
}

pub async fn handle_stop(name: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    let sp = spinner(&format!("Stopping VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client.stop_vm(namespace, &name).await?;
    sp.set_message(format!("Waiting for VM '{}' to stop...", name));

    // Poll until actually stopped (up to 30s)
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        match client.is_running(namespace, &name).await {
            Ok(false) => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::success(&format!("VM '{}' is now stopped", name))
                );
                return Ok(());
            }
            Ok(true) if tokio::time::Instant::now() >= deadline => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::warning(&format!(
                        "VM '{}' stop initiated but not yet stopped (may take longer)",
                        name
                    ))
                );
                return Ok(());
            }
            Ok(true) => continue,
            Err(_) if tokio::time::Instant::now() >= deadline => {
                sp.finish_and_clear();
                println!(
                    "{}",
                    color::warning(&format!(
                        "VM '{}' stop initiated (could not confirm status)",
                        name
                    ))
                );
                return Ok(());
            }
            Err(_) => continue,
        }
    }
}

pub async fn handle_restart(name: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    let sp = spinner(&format!("Restarting VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client.restart_vm(namespace, &name).await?;
    sp.finish_and_clear();
    println!(
        "{}",
        color::success(&format!("VM '{}' restarted successfully", name))
    );
    Ok(())
}

pub async fn handle_pause(name: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    let sp = spinner(&format!("Pausing VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client.pause_vm(namespace, &name).await?;
    sp.finish_and_clear();
    println!("{}", color::success(&format!("VM '{}' paused", name)));
    Ok(())
}

pub async fn handle_unpause(name: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    let sp = spinner(&format!("Unpausing VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client.unpause_vm(namespace, &name).await?;
    sp.finish_and_clear();
    println!("{}", color::success(&format!("VM '{}' unpaused", name)));
    Ok(())
}

pub async fn handle_guest_freeze(name: String, namespace: &str) -> Result<()> {
    let client = crate::kube::KubeClient::new().await?;
    client.guest_freeze(namespace, &name).await?;
    println!(
        "{}",
        color::success(&format!("Guest freeze requested for '{}'", name))
    );
    Ok(())
}

pub async fn handle_guest_unfreeze(name: String, namespace: &str) -> Result<()> {
    let client = crate::kube::KubeClient::new().await?;
    client.guest_unfreeze(namespace, &name).await?;
    println!(
        "{}",
        color::success(&format!("Guest unfreeze requested for '{}'", name))
    );
    Ok(())
}

pub async fn handle_guest_softreboot(name: String, namespace: &str) -> Result<()> {
    let client = crate::kube::KubeClient::new().await?;
    client.guest_soft_reboot(namespace, &name).await?;
    println!(
        "{}",
        color::success(&format!("Guest soft reboot requested for '{}'", name))
    );
    Ok(())
}

pub async fn handle_volume_add(
    name: String,
    volume_name: String,
    pvc: String,
    namespace: &str,
) -> Result<()> {
    let client = crate::kube::KubeClient::new().await?;
    client
        .add_vm_volume(namespace, &name, &volume_name, &pvc)
        .await?;
    println!(
        "{}",
        color::success(&format!(
            "Hotplug volume '{}' (PVC {}) added to VM '{}'",
            volume_name, pvc, name
        ))
    );
    Ok(())
}

pub async fn handle_volume_remove(
    name: String,
    volume_name: String,
    namespace: &str,
) -> Result<()> {
    let client = crate::kube::KubeClient::new().await?;
    client
        .remove_vm_volume(namespace, &name, &volume_name)
        .await?;
    println!(
        "{}",
        color::success(&format!(
            "Hotplug volume '{}' removed from VM '{}'",
            volume_name, name
        ))
    );
    Ok(())
}

pub async fn handle_resize(
    name: String,
    cpus: Option<u32>,
    memory: Option<String>,
    namespace: &str,
) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    if cpus.is_none() && memory.is_none() {
        anyhow::bail!("At least one of --cpus or --memory must be specified");
    }

    // Validate memory format if provided
    if let Some(ref mem) = memory {
        let valid = regex::Regex::new(r"^\d+(\.\d+)?(Mi|Gi|Ti|M|G|T)$")
            .map(|re| re.is_match(mem))
            .unwrap_or(false);
        if !valid {
            anyhow::bail!(
                "Invalid memory format '{}'. Use Kubernetes quantities like '4Gi', '512Mi'",
                mem
            );
        }
    }

    // Validate CPU count
    if let Some(c) = cpus {
        if c == 0 || c > 256 {
            anyhow::bail!("CPU cores must be between 1 and 256, got {}", c);
        }
    }

    let sp = spinner(&format!("Resizing VM '{}'...", name));
    let client = kube::KubeClient::new().await?;
    client
        .update_vm_resources(namespace, &name, cpus, memory.as_deref())
        .await?;
    sp.finish_and_clear();

    let mut changes = Vec::new();
    if let Some(c) = cpus {
        changes.push(format!("CPUs: {}", c));
    }
    if let Some(ref m) = memory {
        changes.push(format!("Memory: {}", m));
    }
    println!(
        "{}",
        color::success(&format!("VM '{}' resized ({})", name, changes.join(", ")))
    );
    println!(
        "  {}",
        color::muted("Note: Changes take effect after VM restart")
    );
    Ok(())
}

pub async fn handle_console(name: String, namespace: &str) -> Result<()> {
    use std::process::Command;

    println!("Attaching to console of VM '{}'...", name);
    println!("(Use Ctrl+] to detach)");
    println!();

    // Use virtctl (KubeVirt CLI) to connect to the console
    // This is the standard way to access VM consoles in KubeVirt
    let status = Command::new("virtctl")
        .args(["console", &name, "-n", namespace])
        .status();

    match status {
        Ok(exit) if exit.success() => Ok(()),
        Ok(exit) => Err(anyhow::anyhow!(
            "Console session ended with exit code: {}",
            exit.code().unwrap_or(-1)
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("Error: 'virtctl' not found in PATH.");
            eprintln!();
            eprintln!("Install virtctl to use the console command:");
            eprintln!("  kubectl krew install virt");
            eprintln!("  # or download from: https://github.com/kubevirt/kubevirt/releases");
            Err(anyhow::anyhow!("virtctl is required for console access"))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to launch virtctl: {}", e)),
    }
}

pub async fn handle_ssh(name: String, user: String, namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;

    // First try to get the VM's IP address
    match KubeClient::new().await {
        Ok(client) => {
            match client.get_vm_ip(namespace, &name).await? {
                Some(ip) => {
                    println!("Connecting to VM '{}' at {}...", name, ip);
                    let status = std::process::Command::new("ssh")
                        .args([&format!("{}@{}", user, ip)])
                        .status();
                    match status {
                        Ok(exit) if exit.success() => Ok(()),
                        Ok(exit) => Err(anyhow::anyhow!(
                            "SSH session ended with exit code: {}",
                            exit.code().unwrap_or(-1)
                        )),
                        Err(e) => Err(anyhow::anyhow!("Failed to launch ssh: {}", e)),
                    }
                }
                None => {
                    // Fallback to virtctl ssh
                    println!("No IP found, trying virtctl ssh...");
                    let status = std::process::Command::new("virtctl")
                        .args(["ssh", "--user", &user, &name, "-n", namespace])
                        .status();
                    match status {
                        Ok(exit) if exit.success() => Ok(()),
                        Ok(_) => Err(anyhow::anyhow!(
                            "SSH session failed. Ensure the VM is running and SSH is enabled."
                        )),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
                            "Neither direct SSH (no IP available) nor virtctl found. Install virtctl for SSH access."
                        )),
                        Err(e) => Err(anyhow::anyhow!("Failed to launch virtctl: {}", e)),
                    }
                }
            }
        }
        Err(_) => Err(anyhow::anyhow!("Failed to connect to Kubernetes cluster")),
    }
}

pub async fn handle_vnc(name: String, namespace: &str) -> Result<()> {
    println!("Opening VNC console for VM '{}'...", name);
    let status = std::process::Command::new("virtctl")
        .args(["vnc", &name, "-n", namespace])
        .status();
    match status {
        Ok(exit) if exit.success() => Ok(()),
        Ok(exit) => Err(anyhow::anyhow!(
            "VNC session ended with exit code: {}",
            exit.code().unwrap_or(-1)
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("Error: 'virtctl' not found in PATH.");
            eprintln!("Install virtctl for VNC access:");
            eprintln!("  kubectl krew install virt");
            Err(anyhow::anyhow!("virtctl is required for VNC access"))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to launch virtctl: {}", e)),
    }
}

pub async fn handle_logs(name: String, follow: bool, tail: u32, namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;

    println!("Fetching logs for VM '{}'...", name);
    println!();

    // Find the virt-launcher pod for this VM
    let client = KubeClient::new().await?;
    let k8s_client = client.client();

    use k8s_openapi::api::core::v1::Pod;
    let pods_api: kube::api::Api<Pod> = kube::api::Api::namespaced(k8s_client, namespace);
    let lp = kube::api::ListParams::default().labels(&format!("kubevirt.io/vm={}", name));

    let pod_list = pods_api
        .list(&lp)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list pods: {}", e))?;

    let pod = pod_list.items.first().ok_or_else(|| {
        anyhow::anyhow!(
            "No virt-launcher pod found for VM '{}'. Is the VM running?",
            name
        )
    })?;
    let pod_name = pod.metadata.name.as_deref().unwrap_or_default();

    // Use kubectl logs for streaming
    let tail_str = tail.to_string();
    let mut args = vec!["logs", pod_name, "-n", namespace, "--tail", &tail_str];
    if follow {
        args.push("-f");
    }

    let status = std::process::Command::new("kubectl")
        .args(&args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to launch kubectl: {}", e))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "kubectl logs exited with code: {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_generate(
    name: String,
    template: Option<String>,
    from_file: Option<String>,
    cpus: Option<u32>,
    memory: Option<String>,
    disk_size: Option<String>,
    output: Option<String>,
    format: String,
    kubevirt: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube;
    use std::fs;

    let mut config = load_or_create_config(&name, namespace, template, from_file)?;
    apply_cli_overrides(
        &mut config,
        CliOverrides {
            cpus,
            memory,
            disk_size,
            ..CliOverrides::default()
        },
    )?;
    validate_vm_config(&config)?;

    let output_format = OutputFormat::parse_format(&format)
        .ok_or_else(|| anyhow!("Invalid output format: {}", format))?;

    let manifest = if kubevirt {
        // Convert to KubeVirt VirtualMachine CRD
        let kubevirt_vm = kube::vm_config_to_kubevirt(&config)?;
        format_output(&kubevirt_vm, output_format)?
    } else {
        // Output VMConfig format
        format_output(&config, output_format)?
    };

    if let Some(output_file) = output {
        fs::write(&output_file, &manifest)?;
        println!("Manifest written to: {}", output_file);
    } else {
        println!("{}", manifest);
    }
    Ok(())
}

pub fn handle_templates() -> Result<()> {
    println!("{}", color::header("Available templates:"));
    for template in TEMPLATES.list() {
        println!("  {} {}", color::value("•"), color::label(&template));
    }
    Ok(())
}

pub fn handle_template(name: String, output: String) -> Result<()> {
    let config = TEMPLATES
        .get(&name)
        .ok_or_else(|| anyhow!("Template not found: {}", name))?;

    let format = OutputFormat::parse_format(&output)
        .ok_or_else(|| anyhow!("Invalid output format: {}", output))?;

    let manifest = format_output(&config, format)?;
    println!("{}", manifest);
    Ok(())
}

pub fn handle_validate(file: String) -> Result<()> {
    use std::fs;

    let content = fs::read_to_string(&file)?;
    let config: VMConfig = if file.ends_with(".json") {
        serde_json::from_str(&content)?
    } else {
        serde_yml::from_str(&content)?
    };

    validate_vm_config(&config)?;
    println!("{}", color::success("Configuration is valid"));
    Ok(())
}

pub async fn handle_status(
    name: String,
    watch: bool,
    interval: u64,
    namespace: &str,
) -> Result<()> {
    use crate::kube;

    let client = kube::KubeClient::new().await?;

    let interval = interval.max(1); // Enforce minimum 1-second interval
    if watch {
        loop {
            // Clear screen
            print!("\x1B[2J\x1B[1;1H");

            match client.get_vm(namespace, &name).await {
                Ok(vm) => {
                    let status = kube::VMStatus::from_vm(&vm);
                    status.display();
                }
                Err(e) => {
                    eprintln!("Error fetching VM status: {}", e);
                    break;
                }
            }

            println!();
            println!("Press Ctrl+C to exit watch mode...");
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {}
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }
    } else {
        let vm = client.get_vm(namespace, &name).await?;
        let status = kube::VMStatus::from_vm(&vm);
        status.display();
    }
    Ok(())
}

pub async fn handle_clone(
    source: String,
    target: String,
    start: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube;

    let sp =
        crate::output::spinner::spinner(&format!("Cloning VM '{}' to '{}'...", source, target));
    let client = kube::KubeClient::new().await?;

    // Get source VM and convert to VMConfig to preserve all fields
    let source_vm = client.get_vm(namespace, &source).await?;
    let mut config = vm_to_config(&source_vm, namespace);
    config.name = target.clone();

    // Update the kubevirt.io/vm label to the new name
    config
        .labels
        .insert("kubevirt.io/vm".to_string(), target.clone());

    // Create the cloned VM
    client.create_vm(&config).await?;
    sp.finish_and_clear();
    println!(
        "{}",
        color::success(&format!("VM '{}' cloned successfully", target))
    );

    if start {
        println!("{}", color::info(&format!("Starting VM '{}'...", target)));
        client.start_vm(namespace, &target).await?;
        println!("{}", color::success(&format!("VM '{}' started", target)));
    }
    Ok(())
}

pub async fn handle_resources(
    all_namespaces: bool,
    sort_by: String,
    namespace: &str,
) -> Result<()> {
    use crate::kube;

    let client = kube::KubeClient::new().await?;

    let vms = if all_namespaces {
        client.list_all_vms().await?
    } else {
        client.list_vms(namespace).await?
    };

    let summary = kube::ResourceSummary::from_vms(&vms);
    summary.display();

    println!();
    println!(
        "{}",
        color::header("╔═══════════════════════════════════════════════════════════════╗")
    );
    println!(
        "{}",
        color::header("║                   VM Resource Details                         ║")
    );
    println!(
        "{}",
        color::header("╚═══════════════════════════════════════════════════════════════╝")
    );
    println!();
    println!(
        "{:<30} {:<15} {:<10} {:<10}",
        color::header("NAME"),
        color::header("NAMESPACE"),
        color::header("CPU"),
        color::header("MEMORY")
    );
    println!("{}", color::muted(&"-".repeat(70)));

    let mut vm_infos: Vec<_> = vms
        .iter()
        .map(|vm| {
            let name = vm.metadata.name.as_deref().unwrap_or("N/A");
            let namespace = vm.metadata.namespace.as_deref().unwrap_or("N/A");
            let cpu = vm
                .spec
                .template
                .spec
                .domain
                .cpu
                .as_ref()
                .and_then(|c| c.cores)
                .unwrap_or(0);
            let memory = vm
                .spec
                .template
                .spec
                .domain
                .memory
                .as_ref()
                .and_then(|m| m.guest.as_deref())
                .unwrap_or("N/A");
            (name, namespace, cpu, memory)
        })
        .collect();

    // Sort
    match sort_by.as_str() {
        "cpu" => vm_infos.sort_by_key(|&(_, _, cpu, _)| std::cmp::Reverse(cpu)),
        "memory" => {
            vm_infos.sort_by(|a, b| {
                let parse_mem = |s: &str| -> u64 {
                    let s = s.trim();
                    if let Some(v) = s.strip_suffix("Ti") {
                        v.parse::<u64>().unwrap_or(0) * 1024 * 1024 * 1024 * 1024
                    } else if let Some(v) = s.strip_suffix("Gi") {
                        v.parse::<u64>().unwrap_or(0) * 1024 * 1024 * 1024
                    } else if let Some(v) = s.strip_suffix("Mi") {
                        v.parse::<u64>().unwrap_or(0) * 1024 * 1024
                    } else if let Some(v) = s.strip_suffix("Ki") {
                        v.parse::<u64>().unwrap_or(0) * 1024
                    } else {
                        s.parse::<u64>().unwrap_or(0)
                    }
                };
                let a_bytes = parse_mem(a.3);
                let b_bytes = parse_mem(b.3);
                b_bytes.cmp(&a_bytes)
            });
        }
        _ => vm_infos.sort_by_key(|&(name, _, _, _)| name),
    }

    for (name, namespace, cpu, memory) in vm_infos {
        println!(
            "{:<30} {:<15} {:<10} {:<10}",
            color::vm_name(name),
            color::namespace(namespace),
            color::resource(&cpu.to_string(), "cpu"),
            color::resource(memory, "memory")
        );
    }
    Ok(())
}

pub async fn handle_export(
    name: String,
    output: Option<String>,
    kubevirt: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube;
    use crate::output;
    use std::fs;

    let client = kube::KubeClient::new().await?;
    let vm = client.get_vm(namespace, &name).await?;

    let manifest = if kubevirt {
        output::to_yaml(&vm)?
    } else {
        // Convert KubeVirt VM to VMConfig format
        let config = vm_to_config(&vm, namespace);
        output::to_yaml(&config)?
    };

    if let Some(output_file) = output {
        fs::write(&output_file, &manifest)?;
        println!(
            "{}",
            color::success(&format!("VM exported to: {}", color::path(&output_file)))
        );
    } else {
        println!("{}", manifest);
    }
    Ok(())
}

pub async fn handle_wizard(name: Option<String>, namespace: &str) -> Result<()> {
    use crate::kube;
    use dialoguer::{Input, Select};

    println!(
        "{}",
        color::header("╔═══════════════════════════════════════════════════════════════╗")
    );
    println!(
        "{}",
        color::header("║           Interactive VM Creation Wizard                      ║")
    );
    println!(
        "{}",
        color::header("╚═══════════════════════════════════════════════════════════════╝")
    );
    println!();

    // VM Name
    let vm_name: String = if let Some(n) = name {
        n
    } else {
        Input::new().with_prompt("VM Name").interact_text()?
    };

    // Template selection
    let templates = TEMPLATES.list();
    let template_idx = Select::new()
        .with_prompt("Select a template")
        .items(&templates)
        .default(0)
        .interact()?;
    let template_name = &templates[template_idx];

    // CPU Cores
    let cpu_cores: u32 = Input::new()
        .with_prompt("CPU Cores")
        .default(2)
        .interact_text()?;

    // Memory
    let memory: String = Input::new()
        .with_prompt("Memory (e.g., 4Gi, 8Gi)")
        .default("4Gi".to_string())
        .interact_text()?;

    // Disk Size
    let disk_size: String = Input::new()
        .with_prompt("Disk Size (e.g., 20Gi, 40Gi)")
        .default("20Gi".to_string())
        .interact_text()?;

    // Start immediately?
    let start_vm = Select::new()
        .with_prompt("Start VM immediately?")
        .items(&["No", "Yes"])
        .default(0)
        .interact()?
        == 1;

    println!();
    println!(
        "{}",
        color::info("Creating VM with the following configuration:")
    );
    println!("  {:<12} {}", color::label("Name:"), color::value(&vm_name));
    println!(
        "  {:<12} {}",
        color::label("Template:"),
        color::value(template_name)
    );
    println!(
        "  {:<12} {}",
        color::label("CPU:"),
        color::resource(&format!("{} cores", cpu_cores), "cpu")
    );
    println!(
        "  {:<12} {}",
        color::label("Memory:"),
        color::resource(&memory, "memory")
    );
    println!(
        "  {:<12} {}",
        color::label("Disk:"),
        color::resource(&disk_size, "disk")
    );
    println!();

    // Create VM
    let mut config = TEMPLATES
        .get(template_name)
        .ok_or_else(|| anyhow!("Unknown template: {}", template_name))?;
    config.name = vm_name.clone();
    config.namespace = namespace.to_string();
    config.cpu.cores = cpu_cores;
    config.memory.size = memory;
    if let Some(disk) = config.disks.first_mut() {
        disk.size = disk_size;
    }

    validate_vm_config(&config)?;

    let client = kube::KubeClient::new().await?;
    client.create_vm(&config).await?;
    println!(
        "{}",
        color::success(&format!("VM '{}' created successfully", vm_name))
    );

    if start_vm {
        client.start_vm(namespace, &vm_name).await?;
        println!("{}", color::success(&format!("VM '{}' started", vm_name)));
    }
    Ok(())
}

pub async fn handle_batch(
    file: String,
    namespace_override: Option<String>,
    dry_run: bool,
    continue_on_error: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube;
    use crate::utils;
    use indicatif::{ProgressBar, ProgressStyle};

    println!(
        "{}",
        color::info(&format!(
            "Loading batch configuration from: {}",
            color::path(&file)
        ))
    );
    let mut batch = utils::BatchConfig::from_file(&file)?;

    // Apply namespace override
    if let Some(ns) = namespace_override {
        batch.apply_namespace(&ns);
    } else {
        batch.apply_namespace(namespace);
    }

    println!(
        "{}",
        color::info(&format!("Found {} VMs to create", batch.vms.len()))
    );
    println!();

    if dry_run {
        println!("{}", color::info("Dry run - VMs that would be created:"));
        for (i, vm) in batch.vms.iter().enumerate() {
            println!(
                "  {}. {} (namespace: {}, {} cores, {})",
                color::value(&(i + 1).to_string()),
                color::vm_name(&vm.name),
                color::namespace(&vm.namespace),
                color::resource(&vm.cpu.cores.to_string(), "cpu"),
                color::resource(&vm.memory.size, "memory")
            );
        }
        return Ok(());
    }

    let client = kube::KubeClient::new().await?;
    let pb = ProgressBar::new(batch.vms.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for vm_config in batch.vms.iter() {
        pb.set_message(format!("Creating {}", vm_config.name));

        // Validate
        if let Err(e) = validate_vm_config(vm_config) {
            eprintln!("✗ Validation failed for '{}': {}", vm_config.name, e);
            error_count += 1;

            if !continue_on_error {
                pb.abandon();
                return Err(e);
            }

            pb.inc(1);
            continue;
        }

        // Create
        match client.create_vm(vm_config).await {
            Ok(_) => {
                success_count += 1;
                pb.println(format!("  ✓ {} created successfully", vm_config.name));
            }
            Err(e) => {
                error_count += 1;
                pb.println(format!("  ✗ {} failed: {}", vm_config.name, e));

                if !continue_on_error {
                    pb.abandon();
                    return Err(e);
                }
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Batch creation complete");

    println!();
    println!(
        "{}",
        color::header("╔═══════════════════════════════════════════════════════════════╗")
    );
    println!(
        "{}",
        color::header("║                   Batch Summary                               ║")
    );
    println!(
        "{}",
        color::header("╚═══════════════════════════════════════════════════════════════╝")
    );
    println!(
        "  {:<12} {}",
        color::label("Total VMs:"),
        color::value(&batch.vms.len().to_string())
    );
    println!(
        "  {:<12} {}",
        color::label("Successful:"),
        color::success(&format!("{} ✓", success_count))
    );
    println!(
        "  {:<12} {}",
        color::label("Failed:"),
        if error_count > 0 {
            color::error(&format!("{} ✗", error_count))
        } else {
            color::muted(&format!("{} ✗", error_count))
        }
    );

    if error_count > 0 && !continue_on_error {
        return Err(anyhow!("Batch creation had {} error(s)", error_count));
    }
    Ok(())
}

/// Diagnose environment: kubeconfig, cluster connectivity, KubeVirt CRDs, namespace.
pub async fn handle_doctor(namespace: &str) -> Result<()> {
    println!("{}", color::header("VMRogue Doctor"));
    println!();

    let mut issues = 0u32;

    // 1. Check kubeconfig
    print!("  Kubeconfig ............ ");
    let kubeconfig_display = if let Some(p) = crate::kube::KUBECONFIG_PATH.get() {
        format!("--kubeconfig {}", p)
    } else if let Ok(env) = std::env::var("KUBECONFIG") {
        format!("$KUBECONFIG={}", env)
    } else {
        "~/.kube/config (default)".to_string()
    };
    println!("{}", color::success(&kubeconfig_display));

    // 2. Check cluster connectivity
    print!("  Cluster connection .... ");
    let client = match crate::kube::get_client().await {
        Ok(c) => {
            println!("{}", color::success("ok"));
            Some(c)
        }
        Err(e) => {
            println!("{}", color::error("FAILED"));
            println!("    {}", color::muted(&format!("{}", e)));
            issues += 1;
            None
        }
    };

    // 3. Check KubeVirt CRDs
    print!("  KubeVirt CRDs ......... ");
    if let Some(ref client) = client {
        let vms: kube::api::Api<crate::kube::types::VirtualMachine> =
            kube::api::Api::namespaced(client.clone(), namespace);
        match vms.list(&kube::api::ListParams::default().limit(1)).await {
            Ok(_) => println!("{}", color::success("installed")),
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                println!("{}", color::error("NOT FOUND"));
                println!(
                    "    {}",
                    color::muted("KubeVirt does not appear to be installed.")
                );
                println!(
                    "    {}",
                    color::muted("See: https://kubevirt.io/user-guide/cluster_admin/installation/")
                );
                issues += 1;
            }
            Err(_) => {
                println!("{}", color::error("NOT FOUND"));
                println!(
                    "    {}",
                    color::muted("VirtualMachine CRD not available. Install KubeVirt first.")
                );
                issues += 1;
            }
        }
    } else {
        println!("{}", color::muted("skipped (no cluster connection)"));
    }

    // 4. Check namespace
    print!("  Namespace '{}' .. ", namespace);
    if let Some(ref client) = client {
        let ns_api: kube::api::Api<k8s_openapi::api::core::v1::Namespace> =
            kube::api::Api::all(client.clone());
        match ns_api.get(namespace).await {
            Ok(_) => println!("{}", color::success("exists")),
            Err(_) => {
                println!("{}", color::warning("NOT FOUND"));
                println!(
                    "    {}",
                    color::muted(&format!(
                        "Namespace '{}' does not exist. Create it or use --namespace.",
                        namespace
                    ))
                );
                issues += 1;
            }
        }
    } else {
        println!("{}", color::muted("skipped (no cluster connection)"));
    }

    // 5. Check data directory
    print!("  Data directory ........ ");
    match crate::utils::data_dir() {
        Ok(dir) => {
            if dir.exists() {
                println!("{}", color::success(&format!("ok ({})", dir.display())));
            } else {
                println!(
                    "{}",
                    color::success(&format!("will be created at {}", dir.display()))
                );
            }
        }
        Err(e) => {
            println!("{}", color::error("FAILED"));
            println!("    {}", color::muted(&format!("{}", e)));
            issues += 1;
        }
    }

    // 6. Config file
    print!("  Config file ........... ");
    match crate::config::AppConfig::user_path() {
        Ok(path) => {
            if path.exists() {
                println!(
                    "{}",
                    color::success(&format!("loaded ({})", path.display()))
                );
            } else {
                println!(
                    "{}",
                    color::muted(&format!("not found ({})", path.display()))
                );
                println!(
                    "    {}",
                    color::muted("Run 'vmrogue config-init' to create one.")
                );
            }
        }
        Err(_) => println!("{}", color::muted("could not determine path")),
    }

    // Summary
    println!();
    if issues == 0 {
        println!("{}", color::success("✓ All checks passed"));
    } else {
        println!(
            "{}",
            color::warning(&format!("✗ {} issue(s) found", issues))
        );
    }

    Ok(())
}

// ========== SEARCH & TROUBLESHOOT ==========

pub async fn handle_search(query: Vec<String>, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::nlp_search::SearchQuery;
    use crate::output::table::CliTable;

    let query_str = query.join(" ");
    if query_str.len() > 1000 {
        anyhow::bail!("Search query too long (max 1000 characters)");
    }
    println!("{}", color::header(&format!("Search: {}", query_str)));
    println!();

    let parsed = SearchQuery::parse(&query_str);

    let client = kube::KubeClient::new().await?;
    let vms = client.list_vms(namespace).await?;

    // Apply parsed filters to VM list
    let mut filtered: Vec<_> = vms
        .iter()
        .filter(|vm| {
            let name = vm.metadata.name.as_deref().unwrap_or("");
            let status = vm
                .status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                .unwrap_or("Unknown");

            // Apply parsed filters
            for filter in &parsed.filters {
                match filter {
                    crate::nlp_search::Filter::Status(s) => {
                        let status_lower = status.to_lowercase();
                        match s {
                            crate::nlp_search::StatusFilter::Running => {
                                if !status_lower.contains("running") {
                                    return false;
                                }
                            }
                            crate::nlp_search::StatusFilter::Stopped => {
                                if !status_lower.contains("stop") {
                                    return false;
                                }
                            }
                            crate::nlp_search::StatusFilter::Failed => {
                                if !status_lower.contains("fail") {
                                    return false;
                                }
                            }
                            crate::nlp_search::StatusFilter::Any => {}
                        }
                    }
                    crate::nlp_search::Filter::Name(n) => {
                        if !name.to_lowercase().contains(&n.to_lowercase()) {
                            return false;
                        }
                    }
                    crate::nlp_search::Filter::Namespace(ns) => {
                        let vm_ns = vm.metadata.namespace.as_deref().unwrap_or("");
                        if !vm_ns.to_lowercase().contains(&ns.to_lowercase()) {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
            true
        })
        .collect();

    // Apply limit
    if let Some(limit) = parsed.limit {
        filtered.truncate(limit);
    }

    if filtered.is_empty() {
        println!("  {}", color::muted("No VMs match the query"));
        return Ok(());
    }

    let mut table = CliTable::new(vec!["Name", "Status", "CPU", "Memory", "Namespace"]);
    for vm in &filtered {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("");
        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            .unwrap_or("Unknown");
        let cpu = vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .and_then(|c| c.cores)
            .map(|c| c.to_string())
            .unwrap_or_default();
        let memory = vm
            .spec
            .template
            .spec
            .domain
            .memory
            .as_ref()
            .and_then(|m| m.guest.clone())
            .unwrap_or_default();

        table.add_row(vec![
            name.to_string(),
            status.to_string(),
            cpu,
            memory,
            ns.to_string(),
        ]);
    }

    println!(
        "  {} VM(s) found",
        color::value(&filtered.len().to_string())
    );
    println!();
    table.print();
    Ok(())
}

pub async fn handle_troubleshoot(name: String, namespace: &str) -> Result<()> {
    use crate::kube;

    println!(
        "{}",
        color::header(&format!("Troubleshooting VM: {}", name))
    );
    println!();

    let client = kube::KubeClient::new().await?;

    // Check if VM exists
    let vm = match client.get_vm(namespace, &name).await {
        Ok(vm) => vm,
        Err(_) => {
            println!(
                "{}",
                color::error(&format!(
                    "VM '{}' not found in namespace '{}'",
                    name, namespace
                ))
            );
            return Ok(());
        }
    };

    let status_str = vm
        .status
        .as_ref()
        .and_then(|s| s.printable_status.as_deref())
        .unwrap_or("Unknown");
    let ready = vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false);
    let running = vm.spec.running.unwrap_or(false);

    println!("  Status:  {}", color::vm_status(status_str));
    println!(
        "  Running: {}",
        if running {
            color::success("true")
        } else {
            color::warning("false")
        }
    );
    println!(
        "  Ready:   {}",
        if ready {
            color::success("true")
        } else {
            color::warning("false")
        }
    );
    println!();

    // Diagnose issues
    let mut issues = Vec::new();

    if running && !ready {
        issues.push((
            "HIGH",
            "VM is running but not ready — may be booting or stuck",
        ));
    }

    if !running && status_str.contains("Failed") {
        issues.push(("CRIT", "VM is in Failed state — check events for errors"));
    }

    if !running && !status_str.contains("Stop") {
        issues.push(("MED", "VM is not running and not cleanly stopped"));
    }

    // Check for VMI
    match client.get_vmi(namespace, &name).await {
        Ok(vmi) => {
            let phase = vmi
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("Unknown");
            println!("  VMI Phase: {}", phase);

            if let Some(ref status) = vmi.status {
                if !status.interfaces.is_empty() {
                    for iface in &status.interfaces {
                        let ip = iface.ip_address.as_deref().unwrap_or("none");
                        let name = iface.name.as_deref().unwrap_or("unknown");
                        println!("  Interface {}: IP={}", name, ip);
                    }
                } else if status.interfaces.is_empty() {
                    issues.push(("MED", "No network interfaces reported by guest agent"));
                }

                if status.node_name.is_none() {
                    issues.push(("HIGH", "VM is not scheduled to any node"));
                } else {
                    println!(
                        "  Node: {}",
                        status.node_name.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
        Err(_) => {
            if running {
                issues.push((
                    "HIGH",
                    "VM is set to running but no VMI found — instance may have failed to start",
                ));
            }
        }
    }

    // Check events
    let events = client.list_events(namespace).await.unwrap_or_default();
    let vm_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.involved_object
                .name
                .as_deref()
                .map(|n| n == name || n.starts_with(&format!("virt-launcher-{}", name)))
                .unwrap_or(false)
        })
        .collect();

    let warning_events: Vec<_> = vm_events
        .iter()
        .filter(|e| e.type_.as_deref() == Some("Warning"))
        .collect();

    if !warning_events.is_empty() {
        issues.push(("MED", "Warning events detected for this VM"));
        println!();
        println!("  {}", color::warning("Recent warnings:"));
        for event in warning_events.iter().take(5) {
            println!(
                "    {} - {}",
                event.reason.as_deref().unwrap_or("Unknown"),
                event.message.as_deref().unwrap_or("")
            );
        }
    }

    println!();
    if issues.is_empty() {
        println!(
            "  {}",
            color::success("No issues detected — VM appears healthy")
        );
    } else {
        println!("  {} issue(s) found:", issues.len());
        for (severity, description) in &issues {
            let sev_color = match *severity {
                "CRIT" => color::error(severity),
                "HIGH" => color::warning(severity),
                _ => color::info(severity),
            };
            println!("    [{}] {}", sev_color, description);
        }
    }

    Ok(())
}

// ========== INFRASTRUCTURE COMMANDS ==========

pub async fn handle_events_list(limit: usize, output: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::table::CliTable;

    let client = kube::KubeClient::new().await?;
    let events = client.list_events(namespace).await?;

    if output == "json" || output == "yaml" {
        let limited: Vec<_> = events.iter().take(limit).collect();
        let text = if output == "json" {
            serde_json::to_string_pretty(&limited)?
        } else {
            serde_yml::to_string(&limited)?
        };
        println!("{}", text);
        return Ok(());
    }

    let mut table = CliTable::new(vec!["Type", "Reason", "Object", "Message", "Age"]);
    for event in events.iter().take(limit) {
        let event_type = event.type_.as_deref().unwrap_or("Normal");
        let reason = event.reason.as_deref().unwrap_or("");
        let obj = event.involved_object.name.as_deref().unwrap_or("");
        let message = event
            .message
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(60)
            .collect::<String>();
        let age = event
            .last_timestamp
            .as_ref()
            .or(event.metadata.creation_timestamp.as_ref())
            .map(|t| {
                let secs = (chrono::Utc::now() - t.0).num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else if secs < 3600 {
                    format!("{}m", secs / 60)
                } else {
                    format!("{}h", secs / 3600)
                }
            })
            .unwrap_or_default();

        table.add_row(vec![
            event_type.to_string(),
            reason.to_string(),
            obj.to_string(),
            message,
            age,
        ]);
    }
    table.print();
    Ok(())
}

pub async fn handle_nodes_list(output: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::table::CliTable;

    let _ = namespace;
    let client = kube::KubeClient::new().await?;
    let nodes = client.list_nodes().await?;

    if output == "json" || output == "yaml" {
        let text = if output == "json" {
            serde_json::to_string_pretty(&nodes)?
        } else {
            serde_yml::to_string(&nodes)?
        };
        println!("{}", text);
        return Ok(());
    }

    let mut table = CliTable::new(vec!["Name", "Status", "Roles", "Version", "CPU", "Memory"]);
    for node in &nodes {
        let name = node.metadata.name.as_deref().unwrap_or("");
        let status = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
            .map(|c| {
                if c.status == "True" {
                    "Ready"
                } else {
                    "NotReady"
                }
            })
            .unwrap_or("Unknown");
        let roles: Vec<String> = node
            .metadata
            .labels
            .as_ref()
            .map(|l| {
                l.keys()
                    .filter_map(|k| k.strip_prefix("node-role.kubernetes.io/").map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let version = node
            .status
            .as_ref()
            .and_then(|s| s.node_info.as_ref())
            .map(|i| i.kubelet_version.as_str())
            .unwrap_or("");
        let cpu = node
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("cpu"))
            .map(|v| v.0.clone())
            .unwrap_or_default();
        let memory = node
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("memory"))
            .map(|v| v.0.clone())
            .unwrap_or_default();

        table.add_row(vec![
            name.to_string(),
            status.to_string(),
            if roles.is_empty() {
                "<none>".to_string()
            } else {
                roles.join(",")
            },
            version.to_string(),
            cpu,
            memory,
        ]);
    }
    table.print();
    Ok(())
}

pub async fn handle_pods_list(output: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::table::CliTable;

    let client = kube::KubeClient::new().await?;
    let pods = client.list_pods(namespace).await?;

    if output == "json" || output == "yaml" {
        let text = if output == "json" {
            serde_json::to_string_pretty(&pods)?
        } else {
            serde_yml::to_string(&pods)?
        };
        println!("{}", text);
        return Ok(());
    }

    let mut table = CliTable::new(vec!["Name", "Status", "Node", "IP", "Restarts", "Age"]);
    for pod in &pods {
        let name = pod.metadata.name.as_deref().unwrap_or("");
        let phase = pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("Unknown");
        let node = pod
            .spec
            .as_ref()
            .and_then(|s| s.node_name.as_deref())
            .unwrap_or("");
        let ip = pod
            .status
            .as_ref()
            .and_then(|s| s.pod_ip.as_deref())
            .unwrap_or("");
        let restarts: i32 = pod
            .status
            .as_ref()
            .and_then(|s| s.container_statuses.as_ref())
            .map(|cs| cs.iter().map(|c| c.restart_count).sum())
            .unwrap_or(0);
        let age = pod
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| {
                let secs = (chrono::Utc::now() - t.0).num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else if secs < 3600 {
                    format!("{}m", secs / 60)
                } else if secs < 86400 {
                    format!("{}h", secs / 3600)
                } else {
                    format!("{}d", secs / 86400)
                }
            })
            .unwrap_or_default();

        table.add_row(vec![
            name.to_string(),
            phase.to_string(),
            node.to_string(),
            ip.to_string(),
            restarts.to_string(),
            age,
        ]);
    }
    table.print();
    Ok(())
}

pub async fn handle_import(
    file: String,
    start: bool,
    dry_run: bool,
    namespace: &str,
) -> Result<()> {
    use crate::kube;
    use crate::output::spinner::spinner;

    // Validate file size to prevent resource exhaustion
    let metadata = std::fs::metadata(&file)
        .map_err(|e| anyhow::anyhow!("Cannot read file '{}': {}", file, e))?;
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
    if metadata.len() > MAX_FILE_SIZE {
        anyhow::bail!(
            "File too large ({} bytes, max {} bytes)",
            metadata.len(),
            MAX_FILE_SIZE
        );
    }

    let content = std::fs::read_to_string(&file)?;
    let vm: crate::kube::types::VirtualMachine = if file.ends_with(".json") {
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in '{}': {}", file, e))?
    } else {
        serde_yml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid YAML in '{}': {}", file, e))?
    };

    let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
    println!("{}", color::header("Import VM"));
    println!("  File:      {}", color::value(&file));
    println!("  VM:        {}", color::vm_name(vm_name));
    println!("  Namespace: {}", color::value(namespace));

    if dry_run {
        println!();
        println!("{}", color::success("Dry run: VM manifest is valid"));
        println!("{}", serde_yml::to_string(&vm)?);
        return Ok(());
    }

    let sp = spinner(&format!("Importing VM '{}'...", vm_name));
    let client = kube::KubeClient::new().await?;

    let kube_client = client.client();
    let api: ::kube::api::Api<crate::kube::types::VirtualMachine> =
        ::kube::api::Api::namespaced(kube_client, namespace);
    api.create(&::kube::api::PostParams::default(), &vm).await?;
    sp.finish_and_clear();

    println!(
        "{}",
        color::success(&format!("VM '{}' imported successfully", vm_name))
    );

    if start {
        client.start_vm(namespace, vm_name).await?;
        println!("{}", color::success(&format!("VM '{}' started", vm_name)));
    }

    Ok(())
}

pub async fn handle_clusters_list(output: String) -> Result<()> {
    use crate::output::table::CliTable;

    let manager = crate::multi_cluster::MultiClusterManager::new();

    if output == "json" || output == "yaml" {
        let text = if output == "json" {
            serde_json::to_string_pretty(&manager)?
        } else {
            serde_yml::to_string(&manager)?
        };
        println!("{}", text);
        return Ok(());
    }

    println!("{}", color::header("Kubernetes Clusters"));
    println!();

    if manager.clusters.is_empty() {
        println!(
            "  {}",
            color::muted(
                "No clusters configured. Run 'vmrogue clusters-discover' to scan kubeconfig."
            )
        );
        return Ok(());
    }

    let mut table = CliTable::new(vec![
        "Name",
        "Context",
        "Environment",
        "Region",
        "Health",
        "Primary",
    ]);
    for cluster in &manager.clusters {
        table.add_row(vec![
            cluster.name.clone(),
            cluster.context.clone(),
            format!("{:?}", cluster.environment),
            cluster.region.clone(),
            format!("{:?}", cluster.health),
            if cluster.is_primary { "✓" } else { "" }.to_string(),
        ]);
    }
    table.print();
    Ok(())
}

pub async fn handle_clusters_discover() -> Result<()> {
    println!("{}", color::header("Cluster Discovery"));
    println!();

    let mut manager = crate::multi_cluster::MultiClusterManager::new();
    match manager.discover_from_kubeconfig().await {
        Ok(count) => {
            println!(
                "{}",
                color::success(&format!("Discovered {} cluster(s):", count))
            );
            for cluster in &manager.clusters {
                println!(
                    "  {} ({}) - {:?}",
                    color::vm_name(&cluster.name),
                    color::muted(&cluster.context),
                    cluster.environment,
                );
            }
        }
        Err(e) => {
            println!(
                "{}",
                color::error(&format!("Failed to discover clusters: {}", e))
            );
        }
    }
    Ok(())
}

// ========== CAPACITY & PLACEMENT ==========

pub async fn handle_capacity(detailed: bool, output: String, namespace: &str) -> Result<()> {
    use crate::kube;
    use crate::output::table::CliTable;

    let client = kube::KubeClient::new().await?;
    let nodes = client.list_nodes().await?;
    let vms = client.list_vms(namespace).await?;

    if output == "json" {
        let info = serde_json::json!({
            "nodes": nodes.len(),
            "vms": vms.len(),
            "running": vms.iter().filter(|v| v.spec.running.unwrap_or(false)).count(),
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("{}", color::header("Cluster Capacity"));
    println!();
    println!("  Nodes: {}", color::value(&nodes.len().to_string()));
    println!("  VMs:   {}", color::value(&vms.len().to_string()));

    let running_count = vms
        .iter()
        .filter(|v| v.spec.running.unwrap_or(false))
        .count();
    println!("  Running: {}", color::value(&running_count.to_string()));
    println!();

    let mut total_cpu = 0u64;
    let mut total_mem_gi = 0f64;
    for node in &nodes {
        let cap = node.status.as_ref().and_then(|s| s.capacity.as_ref());
        if let Some(cpu) = cap
            .and_then(|c| c.get("cpu"))
            .and_then(|v| v.0.parse::<u64>().ok())
        {
            total_cpu += cpu;
        }
        if let Some(mem) = cap.and_then(|c| c.get("memory")).map(|v| &v.0) {
            total_mem_gi += crate::utils::parse_memory_gib(mem);
        }
    }

    let mut vm_cpu = 0u64;
    let mut vm_mem_gi = 0f64;
    for vm in &vms {
        if let Some(ref cpu) = vm.spec.template.spec.domain.cpu {
            vm_cpu += cpu.cores.unwrap_or(0) as u64;
        }
        if let Some(ref mem) = vm.spec.template.spec.domain.memory {
            if let Some(ref guest) = mem.guest {
                vm_mem_gi += crate::utils::parse_memory_gib(guest);
            }
        }
    }

    let cpu_pct = if total_cpu > 0 {
        (vm_cpu as f64 / total_cpu as f64 * 100.0).round()
    } else {
        0.0
    };
    let mem_pct = if total_mem_gi > 0.0 {
        (vm_mem_gi / total_mem_gi * 100.0).round()
    } else {
        0.0
    };

    println!(
        "  CPU:    {}/{} cores allocated ({:.0}%)",
        vm_cpu, total_cpu, cpu_pct
    );
    println!(
        "  Memory: {:.1}/{:.1} GiB allocated ({:.0}%)",
        vm_mem_gi, total_mem_gi, mem_pct
    );

    if detailed {
        println!();
        let mut table = CliTable::new(vec!["Node", "Status", "CPU", "Memory", "Roles"]);
        for node in &nodes {
            let name = node.metadata.name.as_deref().unwrap_or("");
            let status = node
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
                .map(|c| {
                    if c.status == "True" {
                        "Ready"
                    } else {
                        "NotReady"
                    }
                })
                .unwrap_or("Unknown");
            let cpu = node
                .status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("cpu"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let mem = node
                .status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("memory"))
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let roles: Vec<String> = node
                .metadata
                .labels
                .as_ref()
                .map(|l| {
                    l.keys()
                        .filter_map(|k| {
                            k.strip_prefix("node-role.kubernetes.io/").map(String::from)
                        })
                        .collect()
                })
                .unwrap_or_default();

            table.add_row(vec![
                name.to_string(),
                status.to_string(),
                cpu,
                mem,
                if roles.is_empty() {
                    "<none>".to_string()
                } else {
                    roles.join(",")
                },
            ]);
        }
        table.print();
    }

    Ok(())
}

pub async fn handle_placement(name: String, strategy: String, namespace: &str) -> Result<()> {
    use crate::kube;

    println!("{}", color::header(&format!("Placement: {}", name)));
    println!("  Strategy: {}", color::value(&strategy));
    println!();

    let client = kube::KubeClient::new().await?;
    let _vm = client.get_vm(namespace, &name).await?;
    let nodes = client.list_nodes().await?;

    if nodes.is_empty() {
        println!("{}", color::warning("No nodes available for placement"));
        return Ok(());
    }

    let mut scored: Vec<(String, f64, String)> = nodes
        .iter()
        .filter_map(|node| {
            let node_name = node.metadata.name.as_deref()?;
            let is_ready = node
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .and_then(|c| c.iter().find(|c| c.type_ == "Ready"))
                .map(|c| c.status == "True")
                .unwrap_or(false);
            if !is_ready {
                return None;
            }

            let cpu_cap = node
                .status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("cpu"))
                .and_then(|v| v.0.parse::<f64>().ok())
                .unwrap_or(0.0);
            let cpu_alloc = node
                .status
                .as_ref()
                .and_then(|s| s.allocatable.as_ref())
                .and_then(|a| a.get("cpu"))
                .and_then(|v| v.0.parse::<f64>().ok())
                .unwrap_or(0.0);

            let score = match strategy.as_str() {
                "spread" => cpu_alloc,
                "binpack" => cpu_cap - cpu_alloc,
                _ => cpu_alloc,
            };
            let reason = format!("{:.1} CPU available of {:.0}", cpu_alloc, cpu_cap);
            Some((node_name.to_string(), score, reason))
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("  Recommended placement (ranked):");
    for (i, (node_name, score, reason)) in scored.iter().enumerate() {
        let prefix = if i == 0 { "  → " } else { "    " };
        println!(
            "{}{} (score: {:.1}) — {}",
            prefix,
            color::vm_name(node_name),
            score,
            color::muted(reason)
        );
    }

    Ok(())
}

pub async fn handle_list_json(namespace: &str) -> Result<()> {
    use crate::kube;

    let client = kube::KubeClient::new().await?;
    let vms = client.list_vms(namespace).await?;

    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("");
        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            .unwrap_or("Unknown");
        let running = vm.spec.running.unwrap_or(false);

        let line = serde_json::json!({
            "name": name,
            "namespace": ns,
            "status": status,
            "running": running,
        });
        println!("{}", serde_json::to_string(&line)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DiskSource, NetworkType};
    use crate::kube::types::*;
    use std::collections::BTreeMap;

    /// Helper to build a minimal VirtualMachine for testing vm_to_config
    fn build_test_vm(
        name: &str,
        cores: u32,
        sockets: u32,
        threads: u32,
        memory: &str,
    ) -> VirtualMachine {
        VirtualMachine {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some("test-ns".to_string()),
                labels: None,
                annotations: None,
                ..Default::default()
            },
            spec: VirtualMachineSpec {
                running: Some(false),
                run_strategy: None,
                instancetype: None,
                preference: None,
                template: VirtualMachineInstanceTemplateSpec {
                    metadata: None,
                    spec: VirtualMachineInstanceSpec {
                        domain: DomainSpec {
                            resources: ResourceRequirements {
                                requests: Some({
                                    let mut m = BTreeMap::new();
                                    m.insert("memory".to_string(), memory.to_string());
                                    m
                                }),
                                limits: None,
                            },
                            cpu: Some(CPU {
                                cores: Some(cores),
                                sockets: Some(sockets),
                                threads: Some(threads),
                                model: Some("host-passthrough".to_string()),
                                dedicated_cpu_placement: None,
                                isolate_emulator_thread: None,
                                numa: None,
                                realtime: None,
                            }),
                            memory: Some(Memory {
                                guest: Some(memory.to_string()),
                                hugepages: None,
                                max_guest: None,
                            }),
                            devices: None,
                            filesystems: None,
                            features: None,
                            clock: None,
                            firmware: None,
                            machine: None,
                            io_threads_policy: None,
                        },
                        volumes: None,
                        networks: None,
                        termination_grace_period_seconds: None,
                        eviction_strategy: None,
                        node_selector: None,
                        priority_class_name: None,
                        affinity: None,
                        tolerations: None,
                        topology_spread_constraints: None,
                        scheduler_name: None,
                        access_credentials: None,
                    },
                },
            },
            status: None,
        }
    }

    #[test]
    fn test_vm_to_config_basic() {
        let vm = build_test_vm("my-vm", 4, 2, 2, "8Gi");
        let config = vm_to_config(&vm, "production");

        assert_eq!(config.name, "my-vm");
        assert_eq!(config.namespace, "production");
        assert_eq!(config.cpu.cores, 4);
        assert_eq!(config.cpu.sockets, 2);
        assert_eq!(config.cpu.threads, 2);
        assert_eq!(config.memory.size, "8Gi");
    }

    #[test]
    fn test_vm_to_config_cpu_model() {
        let vm = build_test_vm("model-vm", 2, 1, 1, "4Gi");
        let config = vm_to_config(&vm, "default");
        assert_eq!(config.cpu.model, Some("host-passthrough".to_string()));
    }

    #[test]
    fn test_vm_to_config_no_cpu_defaults_to_1() {
        let mut vm = build_test_vm("no-cpu", 1, 1, 1, "2Gi");
        vm.spec.template.spec.domain.cpu = None;
        let config = vm_to_config(&vm, "default");

        assert_eq!(config.cpu.cores, 1);
        assert_eq!(config.cpu.sockets, 1);
        assert_eq!(config.cpu.threads, 1);
    }

    #[test]
    fn test_vm_to_config_memory_from_resources() {
        let mut vm = build_test_vm("mem-vm", 1, 1, 1, "16Gi");
        vm.spec.template.spec.domain.memory = None;
        let config = vm_to_config(&vm, "default");
        assert_eq!(config.memory.size, "16Gi");
    }

    #[test]
    fn test_vm_to_config_with_container_disk() {
        let mut vm = build_test_vm("disk-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.volumes = Some(vec![Volume {
            name: "boot".to_string(),
            container_disk: Some(ContainerDiskSource {
                image: "quay.io/kubevirt/fedora:latest".to_string(),
                image_pull_policy: None,
            }),
            persistent_volume_claim: None,
            empty_disk: None,
            cloud_init_no_cloud: None,
            cloud_init_config_drive: None,
            data_volume: None,
        }]);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.disks.len(), 1);
        assert_eq!(config.disks[0].name, "boot");
        match &config.disks[0].source {
            DiskSource::ContainerDisk { image } => {
                assert_eq!(image, "quay.io/kubevirt/fedora:latest");
            }
            other => panic!("Expected ContainerDisk, got {:?}", other),
        }
    }

    #[test]
    fn test_vm_to_config_with_empty_disk() {
        let mut vm = build_test_vm("empty-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.volumes = Some(vec![Volume {
            name: "data".to_string(),
            container_disk: None,
            persistent_volume_claim: None,
            empty_disk: Some(EmptyDiskSource {
                capacity: "50Gi".to_string(),
            }),
            cloud_init_no_cloud: None,
            cloud_init_config_drive: None,
            data_volume: None,
        }]);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.disks.len(), 1);
        assert_eq!(config.disks[0].name, "data");
        assert_eq!(config.disks[0].source, DiskSource::Blank);
    }

    #[test]
    fn test_vm_to_config_with_pvc() {
        let mut vm = build_test_vm("pvc-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.volumes = Some(vec![Volume {
            name: "root".to_string(),
            container_disk: None,
            persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                claim_name: "my-pvc".to_string(),
            }),
            empty_disk: None,
            cloud_init_no_cloud: None,
            cloud_init_config_drive: None,
            data_volume: None,
        }]);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.disks.len(), 1);
        match &config.disks[0].source {
            DiskSource::PVC { name } => assert_eq!(name, "my-pvc"),
            other => panic!("Expected PVC, got {:?}", other),
        }
    }

    #[test]
    fn test_vm_to_config_with_cloud_init() {
        let mut vm = build_test_vm("cloud-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.volumes = Some(vec![Volume {
            name: "cloudinitdisk".to_string(),
            container_disk: None,
            persistent_volume_claim: None,
            empty_disk: None,
            cloud_init_no_cloud: Some(CloudInitNoCloudSource {
                user_data: Some("#cloud-config\npackages:\n  - vim\n".to_string()),
                network_data: None,
            }),
            cloud_init_config_drive: None,
            data_volume: None,
        }]);

        let config = vm_to_config(&vm, "default");
        assert!(config.cloud_init.is_some());
        let ci = config.cloud_init.as_ref().unwrap();
        assert!(ci.user_data.contains("vim"));
    }

    #[test]
    fn test_vm_to_config_with_pod_network() {
        let mut vm = build_test_vm("net-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.domain.devices = Some(Devices {
            disks: None,
            interfaces: Some(vec![Interface {
                name: "default".to_string(),
                model: Some("virtio".to_string()),
                mac_address: None,
                masquerade: None,
                bridge: None,
                sriov: None,
                ports: None,
                boot_order: None,
            }]),
            tpm: None,
            rng: None,
            inputs: None,
            host_devices: None,
            watchdog: None,
            autoattach_graphics_device: None,
            autoattach_mem_balloon: None,
            network_interface_multiqueue: None,
            video: None,
        });
        vm.spec.template.spec.networks = Some(vec![Network {
            name: "default".to_string(),
            pod: Some(BTreeMap::new()),
            multus: None,
        }]);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.interfaces.len(), 1);
        assert_eq!(config.interfaces[0].name, "default");
        assert_eq!(config.interfaces[0].model, "virtio");
        assert_eq!(config.interfaces[0].network_type, NetworkType::Pod);
    }

    #[test]
    fn test_vm_to_config_with_multus_network() {
        let mut vm = build_test_vm("multus-vm", 2, 1, 1, "4Gi");
        vm.spec.template.spec.domain.devices = Some(Devices {
            disks: None,
            interfaces: Some(vec![Interface {
                name: "data-net".to_string(),
                model: Some("virtio".to_string()),
                mac_address: None,
                masquerade: None,
                bridge: None,
                sriov: None,
                ports: None,
                boot_order: None,
            }]),
            tpm: None,
            rng: None,
            inputs: None,
            host_devices: None,
            watchdog: None,
            autoattach_graphics_device: None,
            autoattach_mem_balloon: None,
            network_interface_multiqueue: None,
            video: None,
        });
        vm.spec.template.spec.networks = Some(vec![Network {
            name: "data-net".to_string(),
            pod: None,
            multus: Some(MultusNetwork {
                network_name: "nad-data".to_string(),
            }),
        }]);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.interfaces.len(), 1);
        match &config.interfaces[0].network_type {
            NetworkType::Multus { name } => assert_eq!(name, "nad-data"),
            other => panic!("Expected Multus, got {:?}", other),
        }
    }

    #[test]
    fn test_vm_to_config_with_labels() {
        let mut vm = build_test_vm("labeled-vm", 2, 1, 1, "4Gi");
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("app".to_string(), "database".to_string());
        labels.insert("env".to_string(), "staging".to_string());
        vm.metadata.labels = Some(labels);

        let config = vm_to_config(&vm, "default");
        assert_eq!(config.labels.get("app"), Some(&"database".to_string()));
        assert_eq!(config.labels.get("env"), Some(&"staging".to_string()));
    }

    #[test]
    fn test_vm_to_config_no_name_defaults_empty() {
        let mut vm = build_test_vm("", 1, 1, 1, "2Gi");
        vm.metadata.name = None;
        let config = vm_to_config(&vm, "default");
        assert_eq!(config.name, "");
    }

    #[test]
    fn test_load_or_create_config_default() {
        let config = load_or_create_config("test-vm", "staging", None, None).unwrap();
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.namespace, "staging");
        assert_eq!(config.cpu.cores, 2);
        assert_eq!(config.memory.size, "4Gi");
        assert!(!config.disks.is_empty());
        assert!(!config.interfaces.is_empty());
    }

    #[test]
    fn test_load_or_create_config_from_template() {
        let config =
            load_or_create_config("my-ubuntu", "prod", Some("ubuntu".to_string()), None).unwrap();
        assert_eq!(config.name, "my-ubuntu");
        assert_eq!(config.namespace, "prod");
    }

    #[test]
    fn test_load_or_create_config_unknown_template() {
        let result = load_or_create_config(
            "vm",
            "default",
            Some("nonexistent-template-xyz".to_string()),
            None,
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Template not found"));
    }
}
