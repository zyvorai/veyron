// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! `VMConfig` → Kairon `Machine`.
//!
//! `VMConfig` stays the template/CLI/API schema; this is the one place it is
//! lowered onto Kairon. Device-model knobs Kairon does not expose (Hyper-V
//! enlightenments, clock timers, bus types, video model, watchdog) are
//! accepted and ignored — FluxVM picks its own device model per backend.

use super::types::*;
use crate::config::{
    BootloaderType, DiskConfig, DiskDeviceType, DiskSource, NetworkType, VMConfig,
};
use anyhow::{Result, bail};
use std::collections::BTreeMap;

/// Convert a [`VMConfig`] to a Kairon `Machine` custom resource.
pub fn machine_from_config(config: &VMConfig) -> Result<Machine> {
    if config.name.trim().is_empty() {
        bail!("machine name is required");
    }

    let mut spec = MachineSpec {
        instance_type_name: config.instancetype.as_ref().map(|m| m.name.clone()),
        ..Default::default()
    };

    // Explicit sizing wins; an instance type alone leaves resources for
    // kairon-controller to resolve at create time.
    let vcpus = config.cpu.cores.max(1) * config.cpu.sockets.max(1) * config.cpu.threads.max(1);
    let explicit_size = config.instancetype.is_none() || !config.memory.size.is_empty();
    if explicit_size {
        spec.resources = MachineResources {
            cpu: vcpus.to_string(),
            memory: config.memory.size.clone(),
            max_memory: config.memory.max_guest.clone(),
            hugepages: config.memory.hugepages_page_size.is_some(),
            cpu_pinning: config.cpu.dedicated_cpu_placement.unwrap_or(false),
            ..Default::default()
        };
    }

    apply_disks(config, &mut spec)?;

    for fs in &config.virtio_fs {
        if spec.volumes.is_empty() {
            bail!(
                "virtiofs share '{}' needs a PVC boot disk: Kairon treats volumes[0] as the boot disk",
                fs.name
            );
        }
        spec.volumes.push(MachineVolume {
            name: fs.name.clone(),
            claim_name: fs.pvc_name.clone(),
            guest_path: Some(format!("/mnt/{}", fs.name)),
            ..Default::default()
        });
    }

    if let Some(ci) = &config.cloud_init {
        let parsed = parse_cloud_config(&ci.user_data);
        if parsed != CloudInitSpec::default() {
            spec.cloud_init = Some(parsed);
        }
    }

    let secure_boot = matches!(
        config.firmware.as_ref().map(|f| &f.bootloader),
        Some(BootloaderType::EFI {
            secure_boot: true,
            ..
        })
    );
    if secure_boot || config.enable_tpm {
        spec.security = Some(SecuritySpec {
            secure_boot,
            tpm: config.enable_tpm,
        });
    }

    spec.network = network_spec(config);

    if let Some(sched) = &config.scheduling {
        let mut placement = PlacementSpec::default();
        if let Some(ns) = &sched.node_selector {
            placement.node_selector = ns.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        }
        if let Some(t) = &sched.tolerations {
            placement.tolerations = t.clone();
        }
        if let Some(t) = &sched.topology_spread_constraints {
            placement.topology_spread_constraints = t.clone();
        }
        if placement != PlacementSpec::default() {
            spec.placement = Some(placement);
        }
    }

    spec.device_claims = config
        .gpus
        .iter()
        .map(|g| gpu_claim_name(&config.name, &g.name))
        .chain(
            config
                .host_devices
                .iter()
                .map(|h| gpu_claim_name(&config.name, &h.name)),
        )
        .map(|name| DeviceClaimReference { name })
        .collect();

    spec.guest_agent = Some(GuestAgentSpec {
        enabled: true,
        console: false,
    });

    spec.power_state = Some(
        config
            .run_strategy
            .as_deref()
            .and_then(crate::kairon::power_state_for_run_strategy)
            .unwrap_or(crate::kairon::POWER_RUNNING)
            .to_string(),
    );

    let mut machine = Machine::new(&config.name, spec);
    machine.metadata.namespace = Some(config.namespace.clone());
    let mut labels: BTreeMap<String, String> = config
        .labels
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    labels
        .entry(crate::kairon::LABEL_MANAGED_BY.to_string())
        .or_insert_with(|| "veyron".to_string());
    machine.metadata.labels = Some(labels);
    if !config.annotations.is_empty() {
        machine.metadata.annotations = Some(
            config
                .annotations
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
    }
    Ok(machine)
}

/// DRA `ResourceClaim` name Veyron creates for a GPU/host device on a Machine.
pub fn gpu_claim_name(vm: &str, device: &str) -> String {
    let mut s = format!("{vm}-{device}")
        .to_ascii_lowercase()
        .replace(|c: char| !(c.is_ascii_alphanumeric() || c == '-'), "-");
    s.truncate(63);
    s.trim_end_matches('-').to_string()
}

fn apply_disks(config: &VMConfig, spec: &mut MachineSpec) -> Result<()> {
    let mut disks: Vec<_> = config
        .disks
        .iter()
        .filter(|d| d.device_type == DiskDeviceType::Disk)
        .collect();
    disks.sort_by_key(|d| {
        if d.boot_order == 0 {
            u32::MAX
        } else {
            d.boot_order
        }
    });

    let mut boot_set = false;
    let mut boot_from_volume = false;
    let mut blank_root: Option<&DiskConfig> = None;
    for disk in disks {
        match &disk.source {
            DiskSource::ContainerDisk { image } if !boot_set => {
                let image = resolve_container_disk_image(image);
                spec.image.digest = oci_digest_of(&image);
                spec.image.source = Some(ImageSource {
                    oci: Some(image),
                    ..Default::default()
                });
                spec.image.disk_size = disk_size(&disk.size);
                boot_set = true;
            }
            DiskSource::GoldenImage { name, .. } if !boot_set => {
                spec.image.image_ref = Some(name.clone());
                spec.image.disk_size = disk_size(&disk.size);
                boot_set = true;
            }
            DiskSource::PVC { name } | DiskSource::DataVolume { name } if !boot_set => {
                spec.volumes.insert(
                    0,
                    MachineVolume {
                        name: disk.name.clone(),
                        claim_name: name.clone(),
                        ..Default::default()
                    },
                );
                boot_set = true;
                boot_from_volume = true;
            }
            DiskSource::PVC { name } | DiskSource::DataVolume { name } => {
                spec.disks.push(MachineDisk {
                    name: sanitize_disk_name(&disk.name),
                    claim_name: name.clone(),
                    ..Default::default()
                });
            }
            // A blank disk only matters as the target of an ISO install.
            DiskSource::Blank => {
                if blank_root.is_none() {
                    blank_root = Some(disk);
                }
            }
            DiskSource::ContainerDisk { image } => {
                bail!(
                    "disk '{}': only the boot disk may be an OCI image ({image})",
                    disk.name
                )
            }
            DiskSource::GoldenImage { name, .. } => {
                bail!(
                    "disk '{}': only the boot disk may come from catalog image '{name}'",
                    disk.name
                )
            }
        }
    }

    // Install media: catalog ISOs only. Template CD-ROMs that are KubeVirt
    // containerDisks (the VirtIO driver CD, cloud-init, sysprep) don't apply.
    for disk in config
        .disks
        .iter()
        .filter(|d| d.device_type == DiskDeviceType::CDROM)
    {
        if let DiskSource::GoldenImage { name, .. } = &disk.source {
            spec.cdroms.push(MachineCdrom {
                name: sanitize_disk_name(&disk.name),
                image_ref: Some(name.clone()),
                ..Default::default()
            });
        }
    }
    if !spec.cdroms.is_empty() && !boot_set {
        spec.image.blank = true;
        spec.image.disk_size = Some(
            blank_root
                .and_then(|d| disk_size(&d.size))
                .unwrap_or_else(|| DEFAULT_INSTALL_DISK_SIZE.to_string()),
        );
    }

    // Root disk on a PVC: Kairon seeds the empty claim from the image (or
    // creates it blank) on first start, then boots an overlay on top of it.
    if let Some(claim) = config
        .annotations
        .get(ROOT_VOLUME_ANNOTATION)
        .filter(|c| !c.is_empty())
    {
        if boot_from_volume {
            bail!("root volume '{claim}' conflicts with a PVC boot disk");
        }
        if spec.image.source.is_none() && spec.image.image_ref.is_none() && !spec.image.blank {
            bail!("root volume '{claim}' needs an image or ISO install to seed it from");
        }
        spec.volumes.insert(
            0,
            MachineVolume {
                name: "root".into(),
                claim_name: claim.clone(),
                ..Default::default()
            },
        );
    }
    Ok(())
}

/// VMConfig annotation naming a PVC to hold the root disk.
pub const ROOT_VOLUME_ANNOTATION: &str = "veyron.io/root-volume";

/// Root disk for an ISO install when the template's blank disk has no size.
const DEFAULT_INSTALL_DISK_SIZE: &str = "60Gi";

/// A disk size worth sending to Kairon: templates use `"0"` (or nothing) for
/// "whatever the image is", which must not shrink the root disk.
fn disk_size(size: &str) -> Option<String> {
    let s = size.trim();
    let digits: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    match digits.parse::<f64>() {
        Ok(n) if n > 0.0 => Some(s.to_string()),
        _ => None,
    }
}

/// Kairon disk names: `^[a-z0-9][a-z0-9-]{0,31}$`.
fn sanitize_disk_name(name: &str) -> String {
    let mut s: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    s = s.trim_start_matches('-').to_string();
    s.truncate(32);
    if s.is_empty() { "data".to_string() } else { s }
}

fn network_spec(config: &VMConfig) -> Option<NetworkSpec> {
    let mut net = NetworkSpec::default();
    for (i, iface) in config.interfaces.iter().enumerate() {
        let bridge = match &iface.network_type {
            NetworkType::Pod => None,
            NetworkType::Bridge => Some(iface.network.clone()),
            NetworkType::Multus { name }
            | NetworkType::SRIOV { name }
            | NetworkType::Ovn { name } => Some(name.clone()),
        };
        match (i, bridge) {
            (0, None) => net.mac = iface.mac_address.clone(),
            (0, Some(b)) => {
                net.mode = Some("tap".into());
                net.bridge = Some(b);
                net.mac = iface.mac_address.clone();
            }
            (_, Some(b)) => net.extra_interfaces.push(ExtraInterface {
                name: sanitize_disk_name(&iface.name),
                bridge: b,
                mac: iface.mac_address.clone(),
            }),
            (_, None) => {}
        }
    }
    if let Some(expose) = config.expose.as_ref().filter(|e| e.enabled) {
        for p in &expose.ports {
            if let Some(host) = p.node_port.and_then(|n| u16::try_from(n).ok()) {
                net.forwards.push(PortForward {
                    host_port: host,
                    guest_port: u16::try_from(p.target_port).unwrap_or(0),
                    protocol: Some(p.protocol.to_ascii_lowercase()).filter(|s| s == "udp"),
                });
            }
        }
    }
    if net == NetworkSpec::default() {
        None
    } else {
        Some(net)
    }
}

/// Lower a `#cloud-config` document onto Kairon's structured `cloudInit`.
///
/// Kairon renders NoCloud itself from these fields; keys it has no field for
/// are dropped, and a non-YAML (shell script) user-data becomes one `runCmd`.
pub fn parse_cloud_config(user_data: &str) -> CloudInitSpec {
    let trimmed = user_data.trim();
    if trimmed.is_empty() {
        return CloudInitSpec::default();
    }
    if trimmed.starts_with("#!") {
        return CloudInitSpec {
            write_files: vec![CloudInitFile {
                path: "/var/lib/veyron/user-data.sh".into(),
                content: user_data.to_string(),
                permissions: Some("0755".into()),
            }],
            run_cmd: vec!["/var/lib/veyron/user-data.sh".into()],
            ..Default::default()
        };
    }
    let Ok(doc) = serde_yml::from_str::<serde_json::Value>(trimmed) else {
        return CloudInitSpec::default();
    };
    let str_of = |v: &serde_json::Value| -> Option<String> {
        match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Array(parts) => Some(
                parts
                    .iter()
                    .map(|p| match p {
                        serde_json::Value::String(s) => shell_quote(s),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            serde_json::Value::Null => None,
            other => Some(other.to_string()),
        }
    };
    let list = |key: &str| -> Vec<String> {
        doc.get(key)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(str_of).collect())
            .unwrap_or_default()
    };

    let mut spec = CloudInitSpec {
        hostname: doc
            .get("hostname")
            .and_then(|v| v.as_str())
            .map(String::from),
        packages: list("packages"),
        run_cmd: list("runcmd"),
        ssh_authorized_keys: list("ssh_authorized_keys"),
        ..Default::default()
    };

    if let Some(users) = doc.get("users").and_then(|v| v.as_array()) {
        for u in users {
            if let Some(name) = u.get("name").and_then(|v| v.as_str()) {
                if spec.user.is_none() {
                    spec.user = Some(name.to_string());
                }
                if let Some(keys) = u.get("ssh_authorized_keys").and_then(|v| v.as_array()) {
                    spec.ssh_authorized_keys
                        .extend(keys.iter().filter_map(|k| k.as_str().map(String::from)));
                }
            }
        }
    }
    if spec.user.is_none() {
        spec.user = doc.get("user").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => {
                o.get("name").and_then(|n| n.as_str()).map(String::from)
            }
            _ => None,
        });
    }
    if let Some(files) = doc.get("write_files").and_then(|v| v.as_array()) {
        for f in files {
            if let (Some(path), Some(content)) = (
                f.get("path").and_then(|v| v.as_str()),
                f.get("content").and_then(|v| v.as_str()),
            ) {
                spec.write_files.push(CloudInitFile {
                    path: path.to_string(),
                    content: content.to_string(),
                    permissions: f
                        .get("permissions")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                });
            }
        }
    }
    spec.ssh_authorized_keys.dedup();
    spec
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// `sha256:…` digest pinned in an OCI reference (`repo@sha256:…`), if any.
pub fn oci_digest_of(image: &str) -> Option<String> {
    image
        .split_once('@')
        .map(|(_, d)| d.to_string())
        .filter(|d| d.starts_with("sha256:"))
}

/// Apply `VEYRON_DISK_IMAGE_REGISTRY` (legacy `ZEUS_OS_KUBEVIRT_DISK_IMAGE_REGISTRY`)
/// to mirror quay.io/* images to a private prefix, and rewrite known-dead public refs.
pub fn resolve_container_disk_image(image: &str) -> String {
    let image = rewrite_broken_container_disk(image);
    let registry = std::env::var("VEYRON_DISK_IMAGE_REGISTRY")
        .or_else(|_| std::env::var("ZEUS_OS_KUBEVIRT_DISK_IMAGE_REGISTRY"));
    if let Ok(registry) = registry {
        let reg = registry.trim_end_matches('/');
        if let Some(path) = image.strip_prefix("quay.io/") {
            return format!("{}/{}", reg, path);
        }
    }
    image
}

/// Public, pullable substitutes for quay.io/containerdisks images that are no longer public.
fn rewrite_broken_container_disk(image: &str) -> String {
    if image.starts_with("quay.io/containerdisks/alpine:")
        || image == "quay.io/containerdisks/alpine"
    {
        return "quay.io/kubevirt/cirros-container-disk-demo:latest".to_string();
    }
    if image.starts_with("quay.io/containerdisks/rockylinux:")
        || image == "quay.io/containerdisks/rockylinux"
    {
        return "quay.io/containerdisks/almalinux:9".to_string();
    }
    if image == "quay.io/containerdisks/almalinux:8"
        || image.starts_with("quay.io/containerdisks/almalinux:8-")
    {
        return "quay.io/containerdisks/almalinux:9".to_string();
    }
    image.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfigBuilder;

    #[test]
    fn container_disk_boot_becomes_oci_image() {
        let cfg = VMConfigBuilder::new("web")
            .namespace("apps")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_container_disk("root", "quay.io/containerdisks/ubuntu:24.04@sha256:abc", 1)
            .add_pod_network("default")
            .build();
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.metadata.namespace.as_deref(), Some("apps"));
        assert_eq!(m.spec.resources.cpu, "2");
        assert_eq!(m.spec.resources.memory, "4Gi");
        let src = m.spec.image.source.as_ref().unwrap();
        assert_eq!(
            src.oci.as_deref(),
            Some("quay.io/containerdisks/ubuntu:24.04@sha256:abc")
        );
        assert_eq!(m.spec.image.digest.as_deref(), Some("sha256:abc"));
        assert_eq!(m.spec.power_state.as_deref(), Some("Running"));
        assert!(m.spec.network.is_none());
    }

    #[test]
    fn pvc_boot_and_data_disks() {
        let cfg = VMConfigBuilder::new("db")
            .cpu(4, 1, 1)
            .memory("8Gi")
            .add_pvc_disk("root", "db-root", 1)
            .add_pvc_disk("Data_1", "db-data", 2)
            .build();
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.spec.volumes.len(), 1);
        assert_eq!(m.spec.volumes[0].claim_name, "db-root");
        assert_eq!(m.spec.disks.len(), 1);
        assert_eq!(m.spec.disks[0].name, "data-1");
        assert_eq!(m.spec.disks[0].claim_name, "db-data");
    }

    fn catalog(name: &str, device: DiskDeviceType) -> DiskConfig {
        DiskConfig {
            name: if device == DiskDeviceType::CDROM {
                "install".into()
            } else {
                "root".into()
            },
            size: "0".into(),
            storage_class: None,
            boot_order: if device == DiskDeviceType::Disk { 1 } else { 0 },
            source: DiskSource::GoldenImage {
                name: name.into(),
                namespace: String::new(),
                from_pvc: false,
                storage_class: None,
                volume_mode: None,
            },
            device_type: device,
            bus: None,
            cache: None,
            io: None,
        }
    }

    #[test]
    fn catalog_image_becomes_image_ref_without_shrinking() {
        let cfg = VMConfigBuilder::new("w")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_disk(catalog("windows-2022", DiskDeviceType::Disk))
            .build();
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.spec.image.image_ref.as_deref(), Some("windows-2022"));
        assert_eq!(m.spec.image.disk_size, None);
        assert!(m.spec.cdroms.is_empty());

        let mut root = catalog("windows-2022", DiskDeviceType::Disk);
        root.size = "80Gi".into();
        let cfg = VMConfigBuilder::new("w").add_disk(root).build();
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.spec.image.disk_size.as_deref(), Some("80Gi"));
    }

    #[test]
    fn iso_install_is_blank_root_plus_cdroms() {
        let mut drivers = catalog("virtio-win", DiskDeviceType::CDROM);
        drivers.name = "drivers".into();
        let cfg = VMConfigBuilder::new("w")
            .add_blank_disk("rootdisk", "", 1)
            .add_disk(catalog("win2022-iso", DiskDeviceType::CDROM))
            .add_disk(drivers)
            .add_cdrom("virtio", "quay.io/kubevirt/virtio-container-disk", 3)
            .build();
        let m = machine_from_config(&cfg).unwrap();
        assert!(m.spec.image.blank);
        assert_eq!(
            m.spec.image.disk_size.as_deref(),
            Some(DEFAULT_INSTALL_DISK_SIZE)
        );
        assert!(m.spec.image.source.is_none());
        let names: Vec<_> = m.spec.cdroms.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["install", "drivers"]);
        assert_eq!(m.spec.cdroms[0].image_ref.as_deref(), Some("win2022-iso"));
    }

    #[test]
    fn root_volume_is_seeded_from_image_or_blank() {
        let mut cfg = VMConfigBuilder::new("w")
            .add_disk(catalog("ubuntu-24.04", DiskDeviceType::Disk))
            .build();
        cfg.annotations
            .insert(ROOT_VOLUME_ANNOTATION.into(), "w-root".into());
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.spec.volumes[0].claim_name, "w-root");
        assert_eq!(m.spec.image.image_ref.as_deref(), Some("ubuntu-24.04"));

        let mut cfg = VMConfigBuilder::new("w")
            .add_blank_disk("rootdisk", "40Gi", 1)
            .build();
        cfg.annotations
            .insert(ROOT_VOLUME_ANNOTATION.into(), "w-root".into());
        assert!(machine_from_config(&cfg).is_err());

        let mut cfg = VMConfigBuilder::new("w")
            .add_pvc_disk("root", "db-root", 1)
            .build();
        cfg.annotations
            .insert(ROOT_VOLUME_ANNOTATION.into(), "w-root".into());
        assert!(machine_from_config(&cfg).is_err());
    }

    #[test]
    fn disk_size_ignores_placeholders() {
        assert_eq!(disk_size("0"), None);
        assert_eq!(disk_size(""), None);
        assert_eq!(disk_size("0Gi"), None);
        assert_eq!(disk_size("20Gi").as_deref(), Some("20Gi"));
    }

    #[test]
    fn cloud_config_is_structured() {
        let ud = "#cloud-config\nhostname: web\nusers:\n  - name: ops\n    ssh_authorized_keys: [\"ssh-ed25519 AAA\"]\npackages: [nginx]\nruncmd:\n  - [systemctl, enable, --now, nginx]\n  - echo hi\nwrite_files:\n  - path: /etc/motd\n    content: hello\n";
        let ci = parse_cloud_config(ud);
        assert_eq!(ci.hostname.as_deref(), Some("web"));
        assert_eq!(ci.user.as_deref(), Some("ops"));
        assert_eq!(ci.ssh_authorized_keys, vec!["ssh-ed25519 AAA"]);
        assert_eq!(ci.packages, vec!["nginx"]);
        assert_eq!(ci.run_cmd, vec!["systemctl enable --now nginx", "echo hi"]);
        assert_eq!(ci.write_files[0].path, "/etc/motd");
    }

    #[test]
    fn shell_script_user_data_runs_once() {
        let ci = parse_cloud_config("#!/bin/sh\necho hi\n");
        assert_eq!(ci.run_cmd.len(), 1);
        assert_eq!(ci.write_files[0].permissions.as_deref(), Some("0755"));
    }

    #[test]
    fn halted_run_strategy_creates_stopped_machine() {
        let mut cfg = VMConfigBuilder::new("x").memory("1Gi").build();
        cfg.run_strategy = Some("Halted".into());
        let m = machine_from_config(&cfg).unwrap();
        assert_eq!(m.spec.power_state.as_deref(), Some("Stopped"));
    }

    #[test]
    fn rewrite_broken_quay_containerdisks() {
        assert_eq!(
            resolve_container_disk_image("quay.io/containerdisks/alpine:3.19"),
            "quay.io/kubevirt/cirros-container-disk-demo:latest"
        );
        assert_eq!(
            resolve_container_disk_image("quay.io/containerdisks/almalinux:8"),
            "quay.io/containerdisks/almalinux:9"
        );
    }

    #[test]
    fn gpu_claim_names_are_dns_safe() {
        assert_eq!(gpu_claim_name("VM_1", "gpu0"), "vm-1-gpu0");
    }
}
