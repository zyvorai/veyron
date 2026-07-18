// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use anyhow::Result;
use kube::{
    Api, Client,
    api::{DeleteParams, ListParams, PostParams},
};

use crate::operator_crds::*;

pub async fn handle_vrvm_list(namespace: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronVM> = Api::namespaced(client, namespace);
    let list = api.list(&ListParams::default()).await?;

    if list.items.is_empty() {
        println!("No VeyronVM resources found in namespace '{}'", namespace);
        return Ok(());
    }

    println!(
        "{:<25} {:<12} {:<20} {:<15} {:<15}",
        "NAME", "PHASE", "VM", "NODE", "IP"
    );
    for vm in &list.items {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let phase = vm
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("-");
        let kv_vm = vm
            .status
            .as_ref()
            .and_then(|s| s.kubevirt_vm_name.as_deref())
            .unwrap_or("-");
        let node = vm
            .status
            .as_ref()
            .and_then(|s| s.node_name.as_deref())
            .unwrap_or("-");
        let ip = vm
            .status
            .as_ref()
            .and_then(|s| s.ip_address.as_deref())
            .unwrap_or("-");
        println!(
            "{:<25} {:<12} {:<20} {:<15} {:<15}",
            name, phase, kv_vm, node, ip
        );
    }

    Ok(())
}

pub async fn handle_vrvm_get(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronVM> = Api::namespaced(client, namespace);
    let vm = api.get(name).await?;

    println!("Name:       {}", vm.metadata.name.as_deref().unwrap_or(""));
    println!(
        "Namespace:  {}",
        vm.metadata.namespace.as_deref().unwrap_or("")
    );
    println!("Template:   {}", vm.spec.template.as_deref().unwrap_or("-"));
    println!("CPU:        {} cores", vm.spec.cpu.cores);
    println!("Memory:     {}", vm.spec.memory.size);
    println!("TPM:        {}", vm.spec.enable_tpm);
    println!("RNG:        {}", vm.spec.enable_rng);

    if let Some(status) = &vm.status {
        println!("\nStatus:");
        println!("  Phase:    {}", status.phase.as_deref().unwrap_or("-"));
        println!(
            "  VM:       {}",
            status.kubevirt_vm_name.as_deref().unwrap_or("-")
        );
        println!("  Node:     {}", status.node_name.as_deref().unwrap_or("-"));
        println!(
            "  IP:       {}",
            status.ip_address.as_deref().unwrap_or("-")
        );
    }

    Ok(())
}

/// Arguments for vrvm-create with full hardware configuration.
pub struct VrvmCreateArgs {
    pub template: Option<String>,
    pub cpus: u32,
    pub memory: String,
    pub disks: Vec<String>,
    pub cdrom: Option<String>,
    pub networks: Vec<String>,
    pub cloud_init: Option<String>,
    pub firmware: Option<String>,
    pub secure_boot: bool,
    pub tpm: bool,
    pub no_rng: bool,
    pub machine_type: Option<String>,
    pub eviction_strategy: Option<String>,
    pub labels: Vec<String>,
    pub from_file: Option<String>,
    pub dry_run: bool,
    pub start: bool,
    pub no_internet: bool,
}

pub async fn handle_vrvm_create(namespace: &str, name: &str, args: VrvmCreateArgs) -> Result<()> {
    // If --from-file is specified, load spec from YAML
    let spec = if let Some(ref path) = args.from_file {
        let content = std::fs::read_to_string(path)?;
        serde_yml::from_str::<VeyronVMSpec>(&content)?
    } else {
        build_spec_from_args(name, &args)?
    };

    if args.dry_run {
        let vm = VeyronVM::new(name, spec);
        println!("{}", serde_yml::to_string(&vm)?);
        return Ok(());
    }

    let client = Client::try_default().await?;
    let api: Api<VeyronVM> = Api::namespaced(client, namespace);
    let vm = VeyronVM::new(name, spec);
    let created = api.create(&PostParams::default(), &vm).await?;
    println!(
        "VeyronVM '{}' created in namespace '{}'",
        created.metadata.name.unwrap_or_default(),
        namespace
    );

    // Print hardware summary
    let s = &created.spec;
    println!("  CPU:    {} cores", s.cpu.cores);
    println!("  Memory: {}", s.memory.size);
    println!(
        "  Disks:  {}",
        if s.disks.is_empty() {
            "none (template default)".to_string()
        } else {
            s.disks
                .iter()
                .map(|d| format!("{}({})", d.name, d.size))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    println!(
        "  NICs:   {}",
        s.interfaces
            .iter()
            .map(|i| format!("{}({})", i.name, i.network_type.net_type))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if s.enable_tpm {
        println!("  TPM:    enabled");
    }
    if let Some(fw) = &s.firmware {
        println!(
            "  Firmware: {}{}",
            fw.bootloader,
            if fw.secure_boot { " + SecureBoot" } else { "" }
        );
    }
    if args.start {
        println!("  Status: starting (operator will reconcile)");
    }

    Ok(())
}

fn build_spec_from_args(name: &str, args: &VrvmCreateArgs) -> Result<VeyronVMSpec> {
    // Parse disks
    let mut disks: Vec<CRDDiskSpec> = args
        .disks
        .iter()
        .map(|s| parse_disk_spec(s))
        .collect::<Result<Vec<_>>>()?;

    // Add CDROM if specified
    if let Some(ref image) = args.cdrom {
        disks.push(CRDDiskSpec {
            name: "cdrom".to_string(),
            size: "0".to_string(),
            storage_class: None,
            boot_order: 2,
            source: CRDDiskSource {
                source_type: "containerDisk".to_string(),
                name: None,
                image: Some(image.clone()),
                namespace: None,
                from_pvc: false,
            },
            device_type: "cdrom".to_string(),
            bus: Some("sata".to_string()),
            cache: None,
            io: None,
        });
    }

    // If template is specified and no disks provided, auto-populate from template
    if disks.is_empty() {
        if let Some(ref tmpl_name) = args.template {
            if let Some(tmpl_config) = crate::templates::TEMPLATES.get(tmpl_name) {
                disks = tmpl_config
                    .disks
                    .iter()
                    .map(|d| {
                        let (source_type, src_name, image, src_ns, src_from_pvc) = match &d.source {
                            crate::config::DiskSource::Blank => {
                                ("blank".to_string(), None, None, None, false)
                            }
                            crate::config::DiskSource::PVC { name } => {
                                ("pvc".to_string(), Some(name.clone()), None, None, false)
                            }
                            crate::config::DiskSource::ContainerDisk { image } => (
                                "containerDisk".to_string(),
                                None,
                                Some(image.clone()),
                                None,
                                false,
                            ),
                            crate::config::DiskSource::DataVolume { name } => (
                                "dataVolume".to_string(),
                                Some(name.clone()),
                                None,
                                None,
                                false,
                            ),
                            crate::config::DiskSource::GoldenImage {
                                name,
                                namespace,
                                from_pvc,
                                ..
                            } => (
                                "dataSource".to_string(),
                                Some(name.clone()),
                                None,
                                Some(namespace.clone()),
                                *from_pvc,
                            ),
                        };
                        let dt = match d.device_type {
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
                                name: src_name,
                                image,
                                namespace: src_ns,
                                from_pvc: src_from_pvc,
                            },
                            device_type: dt.to_string(),
                            bus: d.bus.clone(),
                            cache: d.cache.clone(),
                            io: d.io.clone(),
                        }
                    })
                    .collect();
            }
        }
    }

    // Parse networks (default to pod if none specified)
    let interfaces: Vec<CRDInterfaceSpec> = if args.networks.is_empty() {
        vec![CRDInterfaceSpec {
            name: "default".to_string(),
            network: "default".to_string(),
            model: "virtio".to_string(),
            network_type: CRDNetworkType {
                net_type: "pod".to_string(),
                name: None,
            },
            mac_address: None,
        }]
    } else {
        args.networks
            .iter()
            .enumerate()
            .map(|(i, s)| parse_network_spec(s, i))
            .collect::<Result<Vec<_>>>()?
    };

    // Read cloud-init from file
    let cloud_init = if let Some(ref path) = args.cloud_init {
        let data = std::fs::read_to_string(path)?;
        Some(CRDCloudInitSpec {
            user_data: data,
            user_data_secret_ref: None,
            network_data: None,
            delivery: None,
        })
    } else if args.template.is_some() {
        // Auto-generate basic cloud-init with hostname
        Some(CRDCloudInitSpec {
            user_data: format!("#cloud-config\nhostname: {}\n", name),
            user_data_secret_ref: None,
            network_data: None,
            delivery: None,
        })
    } else {
        None
    };

    // Firmware
    let firmware = if args.secure_boot {
        Some(CRDFirmwareSpec {
            bootloader: "efi".to_string(),
            secure_boot: true,
            persistent: true,
        })
    } else {
        args.firmware.as_ref().map(|f| CRDFirmwareSpec {
            bootloader: f.clone(),
            secure_boot: false,
            persistent: f == "efi",
        })
    };

    // Labels
    let mut labels = std::collections::HashMap::new();
    for l in &args.labels {
        if let Some((k, v)) = l.split_once('=') {
            labels.insert(k.to_string(), v.to_string());
        }
    }

    Ok(VeyronVMSpec {
        template: args.template.clone(),
        profile: None,
        cpu: CRDCPUSpec {
            cores: args.cpus,
            sockets: 1,
            threads: 1,
            model: None,
            dedicated_cpu_placement: None,
            isolate_emulator_thread: None,
        },
        memory: CRDMemorySpec {
            size: args.memory.clone(),
            hugepages_page_size: None,
            max_guest: None,
        },
        disks,
        interfaces,
        cloud_init,
        features: Some(CRDFeaturesSpec {
            acpi: true,
            apic: false,
            hyperv: None,
            kvm_hidden: None,
            smm: if args.secure_boot { Some(true) } else { None },
        }),
        firmware,
        clock: None,
        eviction_strategy: args.eviction_strategy.clone(),
        termination_grace_period: None,
        enable_tpm: args.tpm,
        enable_rng: !args.no_rng,
        machine_type: args.machine_type.clone(),
        running: Some(args.start),
        labels,
        annotations: std::collections::HashMap::new(),
        allow_internet: !args.no_internet,
        windows: None,
        gpus: Vec::new(),
        host_devices: Vec::new(),
    })
}

/// Parse a disk spec string like "name=root,size=20Gi,source=containerDisk,image=quay.io/...,boot=1"
fn parse_disk_spec(spec: &str) -> Result<CRDDiskSpec> {
    let mut name = String::new();
    let mut size = "20Gi".to_string();
    let mut source_type = "blank".to_string();
    let mut image = None;
    let mut pvc_name = None;
    let mut boot_order: u32 = 0;
    let mut bus = None;
    let mut device_type = "disk".to_string();

    for part in spec.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "name" => name = v.to_string(),
                "size" => size = v.to_string(),
                "source" => source_type = v.to_string(),
                "image" => image = Some(v.to_string()),
                "pvc" => {
                    pvc_name = Some(v.to_string());
                    source_type = "pvc".to_string();
                }
                "boot" => boot_order = v.parse().unwrap_or(0),
                "bus" => bus = Some(v.to_string()),
                "type" => device_type = v.to_string(),
                _ => {}
            }
        }
    }

    if name.is_empty() {
        anyhow::bail!("Disk spec missing 'name': {}", spec);
    }

    Ok(CRDDiskSpec {
        name,
        size,
        storage_class: None,
        boot_order,
        source: CRDDiskSource {
            source_type,
            name: pvc_name,
            image,
            namespace: None,
            from_pvc: false,
        },
        device_type,
        bus,
        cache: None,
        io: None,
    })
}

/// Parse a network spec string like "type=pod" or "type=multus,name=br-net"
fn parse_network_spec(spec: &str, index: usize) -> Result<CRDInterfaceSpec> {
    let mut net_type = "pod".to_string();
    let mut net_name = None;

    for part in spec.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "type" => net_type = v.to_string(),
                "name" => net_name = Some(v.to_string()),
                _ => {}
            }
        }
    }

    let iface_name = if index == 0 {
        "default".to_string()
    } else {
        format!("net{}", index)
    };
    let network_name = if index == 0 {
        "default".to_string()
    } else {
        net_name.clone().unwrap_or_else(|| format!("net{}", index))
    };

    Ok(CRDInterfaceSpec {
        name: iface_name,
        network: network_name,
        model: "virtio".to_string(),
        network_type: CRDNetworkType {
            net_type,
            name: net_name,
        },
        mac_address: None,
    })
}

pub async fn handle_vrvm_apply(namespace: &str, file: &str, dry_run: bool) -> Result<()> {
    let content = std::fs::read_to_string(file)?;
    let vm: VeyronVM = serde_yml::from_str(&content)?;

    let name = vm
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| "unnamed".to_string());
    let ns = vm
        .metadata
        .namespace
        .clone()
        .unwrap_or_else(|| namespace.to_string());

    if dry_run {
        println!("{}", serde_yml::to_string(&vm)?);
        return Ok(());
    }

    let client = Client::try_default().await?;
    let api: Api<VeyronVM> = Api::namespaced(client, &ns);

    // Try to get existing — if exists, replace; otherwise create
    match api.get(&name).await {
        Ok(existing) => {
            let mut updated = vm;
            updated.metadata.resource_version = existing.metadata.resource_version;
            api.replace(&name, &PostParams::default(), &updated).await?;
            println!("VeyronVM '{}' updated in namespace '{}'", name, ns);
        }
        Err(_) => {
            api.create(&PostParams::default(), &vm).await?;
            println!("VeyronVM '{}' created in namespace '{}'", name, ns);
        }
    }

    Ok(())
}

pub async fn handle_vrvm_delete(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronVM> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default()).await?;
    println!("VeyronVM '{}' deleted", name);
    Ok(())
}

pub async fn handle_vrbp_list(namespace: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronBlueprint> = Api::namespaced(client, namespace);
    let list = api.list(&ListParams::default()).await?;

    if list.items.is_empty() {
        println!(
            "No VeyronBlueprint resources found in namespace '{}'",
            namespace
        );
        return Ok(());
    }

    println!(
        "{:<25} {:<12} {:<6} {:<6} {:<30}",
        "NAME", "PHASE", "VMs", "READY", "TAGS"
    );
    for bp in &list.items {
        let name = bp.metadata.name.as_deref().unwrap_or("");
        let phase = bp
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("-");
        let total = bp.spec.vms.len();
        let ready = bp.status.as_ref().and_then(|s| s.ready_vms).unwrap_or(0);
        let tags = bp.spec.tags.join(", ");
        println!(
            "{:<25} {:<12} {:<6} {:<6} {:<30}",
            name, phase, total, ready, tags
        );
    }

    Ok(())
}

pub async fn handle_vrbp_get(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronBlueprint> = Api::namespaced(client, namespace);
    let bp = api.get(name).await?;

    println!("Name:        {}", bp.metadata.name.as_deref().unwrap_or(""));
    println!(
        "Description: {}",
        bp.spec.description.as_deref().unwrap_or("-")
    );
    println!("Tags:        {}", bp.spec.tags.join(", "));
    println!("\nVMs:");
    for vm in &bp.spec.vms {
        let deps = if vm.depends_on.is_empty() {
            "-".to_string()
        } else {
            vm.depends_on.join(", ")
        };
        println!("  {} (template: {}, deps: {})", vm.name, vm.template, deps);
    }

    if let Some(status) = &bp.status {
        println!("\nStatus:");
        println!("  Phase: {}", status.phase.as_deref().unwrap_or("-"));
        println!(
            "  Ready: {}/{}",
            status.ready_vms.unwrap_or(0),
            status.total_vms.unwrap_or(0)
        );
    }

    Ok(())
}

pub async fn handle_vrbp_delete(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronBlueprint> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default()).await?;
    println!("VeyronBlueprint '{}' deleted", name);
    Ok(())
}

pub async fn handle_vrpol_list(namespace: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronPolicy> = Api::namespaced(client, namespace);
    let list = api.list(&ListParams::default()).await?;

    if list.items.is_empty() {
        println!(
            "No VeyronPolicy resources found in namespace '{}'",
            namespace
        );
        return Ok(());
    }

    println!(
        "{:<25} {:<10} {:<10} {:<10} {:<8} {:<10}",
        "NAME", "ACTION", "SEVERITY", "RULES", "ENABLED", "VIOLATIONS"
    );
    for p in &list.items {
        let name = p.metadata.name.as_deref().unwrap_or("");
        let violations = p.status.as_ref().and_then(|s| s.violating_vms).unwrap_or(0);
        println!(
            "{:<25} {:<10} {:<10} {:<10} {:<8} {:<10}",
            name,
            p.spec.enforcement_action,
            p.spec.severity,
            p.spec.rules.len(),
            p.spec.enabled,
            violations
        );
    }

    Ok(())
}

pub async fn handle_vrpol_get(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronPolicy> = Api::namespaced(client, namespace);
    let p = api.get(name).await?;

    println!("Name:        {}", p.metadata.name.as_deref().unwrap_or(""));
    println!("Enabled:     {}", p.spec.enabled);
    println!("Enforcement: {}", p.spec.enforcement_action);
    println!("Severity:    {}", p.spec.severity);
    println!(
        "Framework:   {}",
        p.spec.framework.as_deref().unwrap_or("-")
    );
    println!("\nRules:");
    for rule in &p.spec.rules {
        println!("  {} — {} ({})", rule.name, rule.message, rule.condition);
    }

    if let Some(status) = &p.status {
        println!("\nStatus:");
        println!("  Matching: {}", status.matching_vms.unwrap_or(0));
        println!("  Compliant: {}", status.compliant_vms.unwrap_or(0));
        println!("  Violating: {}", status.violating_vms.unwrap_or(0));
    }

    Ok(())
}

pub async fn handle_vrpol_delete(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronPolicy> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default()).await?;
    println!("VeyronPolicy '{}' deleted", name);
    Ok(())
}

pub async fn handle_vrin_list(namespace: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronInsight> = Api::namespaced(client, namespace);
    let list = api.list(&ListParams::default()).await?;

    if list.items.is_empty() {
        println!(
            "No VeyronInsight resources found in namespace '{}'",
            namespace
        );
        return Ok(());
    }

    println!(
        "{:<25} {:<12} {:<10} {:<20} {:<10} {:<30}",
        "NAME", "TYPE", "SEVERITY", "VM", "STATE", "TITLE"
    );
    for i in &list.items {
        let name = i.metadata.name.as_deref().unwrap_or("");
        let state = i
            .status
            .as_ref()
            .and_then(|s| s.state.as_deref())
            .unwrap_or("-");
        let vm = i.spec.vm_ref.as_deref().unwrap_or("-");
        println!(
            "{:<25} {:<12} {:<10} {:<20} {:<10} {:<30}",
            name, i.spec.insight_type, i.spec.severity, vm, state, i.spec.title
        );
    }

    Ok(())
}

pub async fn handle_vract_list(namespace: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronAction> = Api::namespaced(client, namespace);
    let list = api.list(&ListParams::default()).await?;

    if list.items.is_empty() {
        println!(
            "No VeyronAction resources found in namespace '{}'",
            namespace
        );
        return Ok(());
    }

    println!(
        "{:<25} {:<15} {:<20} {:<10} {:<10}",
        "NAME", "ACTION", "VM", "APPROVED", "PHASE"
    );
    for a in &list.items {
        let name = a.metadata.name.as_deref().unwrap_or("");
        let vm = a.spec.vm_ref.as_deref().unwrap_or("-");
        let phase = a
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("-");
        println!(
            "{:<25} {:<15} {:<20} {:<10} {:<10}",
            name, a.spec.action_type, vm, a.spec.approved, phase
        );
    }

    Ok(())
}

pub async fn handle_vract_approve(namespace: &str, name: &str) -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<VeyronAction> = Api::namespaced(client, namespace);

    let mut action = api.get(name).await?;
    action.spec.approved = true;
    api.replace(name, &PostParams::default(), &action).await?;
    println!("VeyronAction '{}' approved — operator will execute", name);
    Ok(())
}
