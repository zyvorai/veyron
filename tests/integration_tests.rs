use vmrogue::blueprints::BLUEPRINTS;
use vmrogue::config::{validate_vm_config, DiskSource, NetworkType, VMConfig, VMConfigBuilder};
use vmrogue::kube::vm_config_to_kubevirt;
use vmrogue::output::{to_json, to_yaml};
use vmrogue::profiles::PROFILES;
use vmrogue::storage::{format_bytes, parse_size_to_bytes, AccessMode, PvcSpec};
use vmrogue::templates::TEMPLATES;

#[test]
fn test_template_to_kubevirt_conversion() {
    let template = TEMPLATES.get("ubuntu").expect("ubuntu template exists");
    let vm = vm_config_to_kubevirt(&template).unwrap();

    assert_eq!(vm.metadata.name, Some("ubuntu-vm".to_string()));
    assert_eq!(vm.spec.running, Some(false));
    assert!(vm.spec.template.spec.volumes.is_some());

    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();
    assert!(volumes.len() >= 2); // rootdisk + datadisk + cloudinit
}

#[test]
fn test_complex_vm_build() {
    let config = VMConfigBuilder::new("complex-vm")
        .namespace("production")
        .cpu(8, 2, 2)
        .cpu_model("host-passthrough")
        .memory("32Gi")
        .add_blank_disk("rootdisk", "100Gi", 1)
        .add_blank_disk("datadisk", "500Gi", 2)
        .add_container_disk("cdrom", "registry.example.com/iso:latest", 3)
        .add_pod_network("management")
        .label("app", "database")
        .label("tier", "backend")
        .label("environment", "production")
        .cloud_init(
            r#"#cloud-config
users:
  - name: admin
    sudo: ALL=(ALL) NOPASSWD:ALL
"#,
        )
        .build();

    // Validate
    vmrogue::config::validate_vm_config(&config).unwrap();

    // Convert to KubeVirt
    let vm = vm_config_to_kubevirt(&config).unwrap();

    assert_eq!(
        vm.spec.template.spec.domain.cpu.as_ref().unwrap().cores,
        Some(8)
    );
    assert_eq!(
        vm.spec.template.spec.domain.cpu.as_ref().unwrap().sockets,
        Some(2)
    );
    assert_eq!(
        vm.spec.template.spec.domain.cpu.as_ref().unwrap().threads,
        Some(2)
    );
    assert_eq!(
        vm.spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("host-passthrough")
    );

    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();
    assert_eq!(volumes.len(), 4); // 3 disks + cloudinit

    // Check cloud-init volume exists
    let cloudinit_vol = volumes.iter().find(|v| v.name == "cloudinitdisk");
    assert!(cloudinit_vol.is_some());
    assert!(cloudinit_vol
        .unwrap()
        .cloud_init_no_cloud
        .as_ref()
        .unwrap()
        .user_data
        .is_some());
}

#[test]
fn test_yaml_serialization_roundtrip() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "40Gi", 1)
        .add_pod_network("default")
        .build();

    // Serialize to YAML
    let yaml = to_yaml(&config).unwrap();

    // Deserialize back
    let deserialized: vmrogue::config::VMConfig = serde_yml::from_str(&yaml).unwrap();

    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.namespace, deserialized.namespace);
    assert_eq!(config.cpu.cores, deserialized.cpu.cores);
    assert_eq!(config.memory.size, deserialized.memory.size);
    assert_eq!(config.disks.len(), deserialized.disks.len());
}

#[test]
fn test_json_serialization_roundtrip() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .build();

    // Serialize to JSON
    let json = to_json(&config).unwrap();

    // Deserialize back
    let deserialized: vmrogue::config::VMConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.namespace, deserialized.namespace);
}

#[test]
fn test_all_disk_types_conversion() {
    let mut config = VMConfigBuilder::new("all-disks")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .build();

    // Add all disk types
    config.disks.push(vmrogue::config::DiskConfig {
        name: "blank-disk".to_string(),
        size: "10Gi".to_string(),
        storage_class: Some("fast-ssd".to_string()),
        boot_order: 1,
        source: DiskSource::Blank,
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    config.disks.push(vmrogue::config::DiskConfig {
        name: "pvc-disk".to_string(),
        size: "20Gi".to_string(),
        storage_class: None,
        boot_order: 2,
        source: DiskSource::PVC {
            name: "existing-pvc".to_string(),
        },
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    config.disks.push(vmrogue::config::DiskConfig {
        name: "container-disk".to_string(),
        size: "0".to_string(),
        storage_class: None,
        boot_order: 3,
        source: DiskSource::ContainerDisk {
            image: "registry.io/os:latest".to_string(),
        },
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    config.disks.push(vmrogue::config::DiskConfig {
        name: "dv-disk".to_string(),
        size: "50Gi".to_string(),
        storage_class: None,
        boot_order: 4,
        source: DiskSource::DataVolume {
            name: "my-datavolume".to_string(),
        },
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    config.interfaces.push(vmrogue::config::InterfaceConfig {
        name: "default".to_string(),
        network: "default".to_string(),
        model: "virtio".to_string(),
        network_type: NetworkType::Pod,
        mac_address: None,
    });

    let vm = vm_config_to_kubevirt(&config).unwrap();
    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();

    assert_eq!(volumes.len(), 4);

    // Verify each disk type was converted
    let blank = volumes.iter().find(|v| v.name == "blank-disk");
    assert!(blank.is_some());
    assert!(blank.unwrap().empty_disk.is_some());

    let pvc = volumes.iter().find(|v| v.name == "pvc-disk");
    assert!(pvc.is_some());
    assert!(pvc.unwrap().persistent_volume_claim.is_some());

    let container = volumes.iter().find(|v| v.name == "container-disk");
    assert!(container.is_some());
    assert!(container.unwrap().container_disk.is_some());

    let dv = volumes.iter().find(|v| v.name == "dv-disk");
    assert!(dv.is_some());
    assert!(dv.unwrap().data_volume.is_some());
}

#[test]
fn test_network_types_conversion() {
    let mut config = VMConfigBuilder::new("network-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .build();

    config.interfaces.push(vmrogue::config::InterfaceConfig {
        name: "pod-net".to_string(),
        network: "default".to_string(),
        model: "virtio".to_string(),
        network_type: NetworkType::Pod,
        mac_address: None,
    });

    config.interfaces.push(vmrogue::config::InterfaceConfig {
        name: "bridge-net".to_string(),
        network: "br0".to_string(),
        model: "e1000".to_string(),
        network_type: NetworkType::Bridge,
        mac_address: None,
    });

    config.interfaces.push(vmrogue::config::InterfaceConfig {
        name: "multus-net".to_string(),
        network: "data-network".to_string(),
        model: "virtio".to_string(),
        network_type: NetworkType::Multus {
            name: "nad-data-network".to_string(),
        },
        mac_address: None,
    });

    let vm = vm_config_to_kubevirt(&config).unwrap();
    let interfaces = vm
        .spec
        .template
        .spec
        .domain
        .devices
        .as_ref()
        .unwrap()
        .interfaces
        .as_ref()
        .unwrap();
    let networks = vm.spec.template.spec.networks.as_ref().unwrap();

    assert_eq!(interfaces.len(), 3);
    assert_eq!(networks.len(), 3);

    // Check Multus network was converted
    let multus_net = networks.iter().find(|n| n.name == "multus-net");
    assert!(multus_net.is_some());
    assert!(multus_net.unwrap().multus.is_some());
    assert_eq!(
        multus_net.unwrap().multus.as_ref().unwrap().network_name,
        "nad-data-network"
    );
}

// ========== TEMPLATE TESTS ==========

#[test]
fn test_all_templates_valid() {
    let template_names = TEMPLATES.list();
    assert!(
        template_names.len() >= 40,
        "Expected at least 40 templates, got {}",
        template_names.len()
    );

    for name in &template_names {
        let config = TEMPLATES.get(name);
        assert!(config.is_some(), "Template '{}' should exist", name);

        let config = config.unwrap();
        assert!(
            !config.name.is_empty(),
            "Template '{}' should have a name",
            name
        );
        assert!(
            config.cpu.cores >= 1,
            "Template '{}' should have at least 1 CPU core",
            name
        );
        assert!(
            !config.memory.size.is_empty(),
            "Template '{}' should have memory size",
            name
        );
    }
}

#[test]
fn test_all_templates_convert_to_kubevirt() {
    let template_names = TEMPLATES.list();

    for name in &template_names {
        let config = TEMPLATES.get(name).unwrap();
        let result = vm_config_to_kubevirt(&config);
        assert!(
            result.is_ok(),
            "Template '{}' should convert to KubeVirt: {:?}",
            name,
            result.err()
        );
    }
}

#[test]
fn test_template_families() {
    let families = ["ubuntu", "fedora", "centos", "debian", "rhel", "windows"];
    for family in families {
        let config = TEMPLATES.get(family);
        assert!(
            config.is_some(),
            "Template family '{}' should have a default template",
            family
        );
    }
}

// ========== PROFILE TESTS ==========

#[test]
fn test_builtin_profiles_exist() {
    let manager = PROFILES.read().expect("profiles lock");
    let profiles = manager.list();

    let expected = [
        "minimal",
        "dev",
        "test",
        "web",
        "prod",
        "database",
        "microservice",
        "high-perf",
    ];
    for name in expected {
        assert!(
            manager.exists(name),
            "Built-in profile '{}' should exist",
            name
        );
        assert!(
            manager.is_builtin(name),
            "Profile '{}' should be built-in",
            name
        );

        let profile = manager.get(name);
        assert!(
            profile.is_some(),
            "Profile '{}' should be retrievable",
            name
        );
        assert!(
            profile.unwrap().cpu_cores >= 1,
            "Profile '{}' should have at least 1 CPU",
            name
        );
    }

    assert!(
        profiles.len() >= 8,
        "Expected at least 8 profiles, got {}",
        profiles.len()
    );
}

#[test]
fn test_profile_resource_ordering() {
    let manager = PROFILES.read().expect("profiles lock");

    let minimal = manager.get("minimal").unwrap();
    let dev = manager.get("dev").unwrap();
    let prod = manager.get("prod").unwrap();
    let high_perf = manager.get("high-perf").unwrap();

    // Resources should scale up
    assert!(minimal.cpu_cores <= dev.cpu_cores);
    assert!(dev.cpu_cores <= prod.cpu_cores);
    assert!(prod.cpu_cores <= high_perf.cpu_cores);
}

// ========== BLUEPRINT TESTS ==========

#[test]
fn test_builtin_blueprints_exist() {
    let manager = BLUEPRINTS.read().expect("blueprints lock");
    let blueprints = manager.list();

    let expected = ["lamp", "k8s-cluster", "3tier", "cicd", "dev-stack"];
    for name in expected {
        assert!(
            manager.exists(name),
            "Built-in blueprint '{}' should exist",
            name
        );
        assert!(
            manager.is_builtin(name),
            "Blueprint '{}' should be built-in",
            name
        );

        let bp = manager.get(name);
        assert!(bp.is_some(), "Blueprint '{}' should be retrievable", name);
        assert!(
            !bp.unwrap().vms.is_empty(),
            "Blueprint '{}' should have VMs",
            name
        );
    }

    assert!(
        blueprints.len() >= 5,
        "Expected at least 5 blueprints, got {}",
        blueprints.len()
    );
}

#[test]
fn test_blueprint_vm_templates_valid() {
    let manager = BLUEPRINTS.read().expect("blueprints lock");

    for bp in manager.list() {
        for vm_spec in &bp.vms {
            let template = TEMPLATES.get(&vm_spec.template);
            assert!(
                template.is_some(),
                "Blueprint '{}' VM '{}' references template '{}' which doesn't exist",
                bp.name,
                vm_spec.name,
                vm_spec.template
            );
        }
    }
}

// ========== VALIDATION TESTS ==========

#[test]
fn test_example_files_valid_yaml() {
    let example_files = ["examples/basic-vm.yaml", "examples/ubuntu-cloud-init.yaml"];

    for file in example_files {
        let content = std::fs::read_to_string(file);
        assert!(
            content.is_ok(),
            "Example file '{}' should be readable",
            file
        );

        let config: Result<VMConfig, _> = serde_yml::from_str(&content.unwrap());
        assert!(
            config.is_ok(),
            "Example file '{}' should parse as VMConfig: {:?}",
            file,
            config.err()
        );
    }
}

#[test]
fn test_validation_catches_invalid_name() {
    let config = VMConfigBuilder::new("INVALID_NAME!")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(result.is_err(), "Should reject invalid VM name");
}

#[test]
fn test_validation_catches_zero_cpu() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(0, 1, 1)
        .memory("1Gi")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(result.is_err(), "Should reject 0 CPU cores");
}

#[test]
fn test_validation_catches_invalid_memory() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("invalid")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(result.is_err(), "Should reject invalid memory format");
}

// ========== STORAGE UTILITY TESTS ==========

#[test]
fn test_storage_size_conversions() {
    assert_eq!(parse_size_to_bytes("100Gi"), Some(100 * 1024 * 1024 * 1024));
    assert_eq!(parse_size_to_bytes("512Mi"), Some(512 * 1024 * 1024));

    let bytes = parse_size_to_bytes("10Gi").unwrap();
    let formatted = format_bytes(bytes);
    assert_eq!(formatted, "10.0Gi");
}

#[test]
fn test_pvc_spec_for_vm_disk() {
    let pvc = PvcSpec::new("my-vm-rootdisk", "100Gi")
        .with_namespace("production")
        .with_storage_class("ceph-rbd")
        .with_access_mode(AccessMode::ReadWriteOnce)
        .with_label("app", "database")
        .with_label("managed-by", "vmrogue");

    assert_eq!(pvc.name, "my-vm-rootdisk");
    assert_eq!(pvc.size, "100Gi");
    assert_eq!(pvc.namespace, "production");
    assert_eq!(pvc.storage_class.as_deref(), Some("ceph-rbd"));
    assert_eq!(pvc.labels.len(), 2);
}

// ========== END-TO-END WORKFLOW TESTS ==========

#[test]
fn test_template_to_config_to_kubevirt_workflow() {
    // Simulate: user picks a template, customizes it, converts to KubeVirt
    let mut config = TEMPLATES.get("ubuntu").unwrap();
    config.name = "my-web-server".to_string();
    config.namespace = "production".to_string();
    config.cpu.cores = 4;
    config.memory.size = "16Gi".to_string();
    config.labels.insert("app".to_string(), "web".to_string());

    // Validate
    validate_vm_config(&config).unwrap();

    // Convert to KubeVirt
    let vm = vm_config_to_kubevirt(&config).unwrap();
    assert_eq!(vm.metadata.name, Some("my-web-server".to_string()));
    assert_eq!(vm.metadata.namespace, Some("production".to_string()));
    assert_eq!(
        vm.spec.template.spec.domain.cpu.as_ref().unwrap().cores,
        Some(4)
    );

    // Serialize to YAML (what kubectl would apply)
    let yaml = to_yaml(&vm).unwrap();
    assert!(yaml.contains("my-web-server"));
    assert!(yaml.contains("production"));
}

#[test]
fn test_profile_applied_to_template() {
    let manager = PROFILES.read().expect("profiles lock");
    let profile = manager.get("database").unwrap();

    let mut config = TEMPLATES.get("ubuntu").unwrap();
    config.name = "db-server".to_string();
    config.cpu.cores = profile.cpu_cores;
    config.cpu.sockets = profile.cpu_sockets;
    config.memory.size = profile.memory.clone();

    validate_vm_config(&config).unwrap();

    assert_eq!(config.cpu.cores, profile.cpu_cores);
    assert_eq!(config.memory.size, profile.memory);
}

// ========== MULTI-DISK VM CONVERSION ==========

#[test]
fn test_multi_disk_vm_with_all_types() {
    let mut config = VMConfigBuilder::new("multi-disk-vm")
        .namespace("default")
        .cpu(4, 1, 1)
        .memory("8Gi")
        .add_blank_disk("rootdisk", "40Gi", 1)
        .add_container_disk("cdrom", "registry.example.com/iso:latest", 2)
        .add_pod_network("default")
        .build();

    config.disks.push(vmrogue::config::DiskConfig {
        name: "pvc-disk".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("ceph-rbd".to_string()),
        boot_order: 3,
        source: DiskSource::PVC {
            name: "existing-pvc".to_string(),
        },
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    config.disks.push(vmrogue::config::DiskConfig {
        name: "dv-disk".to_string(),
        size: "200Gi".to_string(),
        storage_class: None,
        boot_order: 4,
        source: DiskSource::DataVolume {
            name: "my-dv".to_string(),
        },
        device_type: vmrogue::config::DiskDeviceType::default(),
        bus: None,
        cache: None,
        io: None,
    });

    validate_vm_config(&config).unwrap();

    let vm = vm_config_to_kubevirt(&config).unwrap();
    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();
    assert_eq!(volumes.len(), 4);

    // Verify blank (emptyDisk)
    assert!(volumes
        .iter()
        .any(|v| v.name == "rootdisk" && v.empty_disk.is_some()));
    // Verify container disk
    assert!(volumes
        .iter()
        .any(|v| v.name == "cdrom" && v.container_disk.is_some()));
    // Verify PVC
    assert!(volumes
        .iter()
        .any(|v| v.name == "pvc-disk" && v.persistent_volume_claim.is_some()));
    // Verify DataVolume
    assert!(volumes
        .iter()
        .any(|v| v.name == "dv-disk" && v.data_volume.is_some()));
}

// ========== CLOUD-INIT IN KUBEVIRT CONVERSION ==========

#[test]
fn test_cloud_init_in_kubevirt_conversion() {
    let config = VMConfigBuilder::new("cloud-init-vm")
        .namespace("default")
        .cpu(2, 1, 1)
        .memory("4Gi")
        .add_blank_disk("rootdisk", "20Gi", 1)
        .add_pod_network("default")
        .cloud_init("#cloud-config\npackages:\n  - nginx\n  - vim\n")
        .build();

    validate_vm_config(&config).unwrap();
    let vm = vm_config_to_kubevirt(&config).unwrap();

    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();
    let cloudinit_vol = volumes.iter().find(|v| v.name == "cloudinitdisk");
    assert!(cloudinit_vol.is_some(), "Cloud-init volume should exist");

    let ci = cloudinit_vol.unwrap().cloud_init_no_cloud.as_ref().unwrap();
    let user_data = ci.user_data.as_ref().unwrap();
    assert!(user_data.contains("nginx"));
    assert!(user_data.contains("vim"));
}

// ========== ERROR PATH TESTS ==========

#[test]
fn test_validation_empty_name() {
    let config = VMConfigBuilder::new("")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(result.is_err(), "Empty name should fail validation");
}

#[test]
fn test_validation_zero_cpu_rejected() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(0, 1, 1)
        .memory("2Gi")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(result.is_err(), "Zero CPU cores should fail validation");
}

#[test]
fn test_validation_invalid_memory_format() {
    let config = VMConfigBuilder::new("test-vm")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("not-a-size")
        .add_blank_disk("root", "10Gi", 1)
        .add_pod_network("default")
        .build();

    let result = validate_vm_config(&config);
    assert!(
        result.is_err(),
        "Invalid memory format should fail validation"
    );
}

// ========== SNAPSHOT CONFIG ==========

#[test]
fn test_snapshot_config_creation_and_serialization() {
    use vmrogue::snapshots::{RetentionPolicy, SnapshotConfig};

    let config = SnapshotConfig::new("my-vm", "snap-20240101")
        .with_description("Daily snapshot")
        .with_label("env", "production")
        .with_label("team", "platform");

    assert_eq!(config.vm_name, "my-vm");
    assert_eq!(config.snapshot_name, "snap-20240101");
    assert_eq!(config.description, Some("Daily snapshot".to_string()));
    assert_eq!(config.labels.len(), 2);

    // Serialization roundtrip
    let yaml = serde_yml::to_string(&config).unwrap();
    let deserialized: SnapshotConfig = serde_yml::from_str(&yaml).unwrap();
    assert_eq!(deserialized.vm_name, "my-vm");
    assert_eq!(deserialized.snapshot_name, "snap-20240101");
    assert_eq!(
        deserialized.labels.get("env"),
        Some(&"production".to_string())
    );

    // Default retention
    let retention = RetentionPolicy::default();
    assert_eq!(retention.max_snapshots, Some(10));
    assert_eq!(retention.max_age_days, Some(30));
}

// ========== BACKUP CONFIG ==========

#[test]
fn test_backup_config_types_and_retention() {
    use vmrogue::backup::{BackupConfig, BackupType, CompressionType, RetentionPolicy};

    // Full backup with defaults
    let config = BackupConfig::new("db-vm", "db-backup-001");
    assert_eq!(config.backup_type, BackupType::Full);
    assert_eq!(config.compression, CompressionType::Gzip);
    assert!(config.encryption_enabled);

    // Incremental with zstd
    let config2 = BackupConfig::new("app-vm", "app-backup-002")
        .with_type(BackupType::Incremental)
        .without_encryption();
    assert_eq!(config2.backup_type, BackupType::Incremental);
    assert!(!config2.encryption_enabled);

    // Retention policies
    let short = RetentionPolicy::short_term();
    assert_eq!(short.keep_daily, 3);
    assert_eq!(short.max_age_days, Some(30));

    let long = RetentionPolicy::long_term();
    assert_eq!(long.keep_yearly, 10);
    assert_eq!(long.max_age_days, None);

    // Serialization roundtrip
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: BackupConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.vm_name, "db-vm");
    assert_eq!(deserialized.backup_name, "db-backup-001");
}

// ========== NETWORK POLICY ==========

#[test]
fn test_network_policy_rule_creation() {
    use vmrogue::networking::policies::{
        NetworkPolicyRule, PolicyAction, PortRange, TrafficDirection,
    };
    use vmrogue::networking::NetworkProtocol;

    let mut rule =
        NetworkPolicyRule::new("allow-http", PolicyAction::Allow, TrafficDirection::Ingress)
            .with_protocol(NetworkProtocol::TCP)
            .with_priority(10);
    rule.add_destination_port(PortRange::single(80));
    rule.add_destination_port(PortRange::single(443));
    rule.add_source_cidr("10.0.0.0/8");

    assert_eq!(rule.name, "allow-http");
    assert_eq!(rule.action, PolicyAction::Allow);
    assert_eq!(rule.destination_ports.len(), 2);
    assert!(rule.destination_ports[0].contains(80));
    assert!(!rule.destination_ports[0].contains(81));
    assert_eq!(rule.source_cidrs.len(), 1);

    let deny = NetworkPolicyRule::new("deny-ssh", PolicyAction::Deny, TrafficDirection::Ingress);
    assert_eq!(deny.action, PolicyAction::Deny);
}

// ========== SECURITY ASSESSMENT ==========

#[test]
fn test_security_assessment_vulnerability_scoring() {
    use vmrogue::security::{RiskLevel, SecurityAssessment, Severity, Vulnerability};

    let mut assessment = SecurityAssessment::new("production-db");

    // No vulns -> score 100, Low risk
    assessment.calculate_score();
    // Initial score with no vulns: 100
    // But risk_level depends on score: 90-100 = Low
    // Empty vulns means score stays 100

    // Add various severity vulnerabilities
    assessment.add_vulnerability(
        Vulnerability::new("V1", "Critical CVE", Severity::Critical)
            .with_cvss(9.8)
            .with_cve("CVE-2024-0001"),
    );
    assessment
        .add_vulnerability(Vulnerability::new("V2", "High CVE", Severity::High).with_cvss(7.5));
    assessment.add_vulnerability(
        Vulnerability::new("V3", "Medium issue", Severity::Medium).with_cvss(5.0),
    );

    assessment.calculate_score();
    // 100 - 20 (critical) - 10 (high) - 5 (medium) = 65
    assert_eq!(assessment.overall_score, 65);
    assert_eq!(assessment.risk_level, RiskLevel::High);
    assert_eq!(assessment.critical_count(), 1);
    assert_eq!(assessment.high_count(), 1);
    assert_eq!(assessment.vulnerabilities.len(), 3);
}

// ========== COST TRACKING ==========

#[test]
fn test_cost_tracking_entries_and_allocation() {
    use vmrogue::cost::{CostCalculator, CostSummary};

    let calculator = CostCalculator::default();

    // Calculate costs for different VMs
    let vm1_cost = calculator.calculate_vm_cost("web-vm", "prod", 2, 4, 20, 730.0);
    let vm2_cost = calculator.calculate_vm_cost("db-vm", "prod", 8, 32, 500, 730.0);
    let vm3_cost = calculator.calculate_vm_cost("dev-vm", "dev", 1, 2, 10, 365.0);

    // Aggregate into summary
    let mut summary = CostSummary::new();
    summary.add_vm_cost(&vm1_cost);
    summary.add_vm_cost(&vm2_cost);
    summary.add_vm_cost(&vm3_cost);

    assert_eq!(summary.vm_count, 3);
    assert!(summary.total_cost > 0.0);
    assert!(
        vm2_cost.total_cost > vm1_cost.total_cost,
        "DB VM should cost more"
    );
    assert!(
        vm3_cost.total_cost < vm1_cost.total_cost,
        "Dev VM with half runtime should cost less"
    );

    // Network and snapshot costs
    let net_cost = calculator.calculate_network_cost(100.0);
    assert!(net_cost > 0.0);
    let snap_cost = calculator.calculate_snapshot_cost(50, 1.0);
    assert!(snap_cost > 0.0);
}

// ========== HA CONFIG ==========

#[test]
fn test_ha_priority_sorting_and_checks() {
    use vmrogue::migration::ha::{EvictionStrategy, HAConfig, HAManager, HAPriority};

    let mut manager = HAManager::new();

    manager.add_config(HAConfig::new("low-vm").with_priority(HAPriority::Low));
    manager.add_config(
        HAConfig::new("critical-vm")
            .with_priority(HAPriority::Critical)
            .with_eviction_strategy(EvictionStrategy::LiveMigrate),
    );
    manager.add_config(HAConfig::new("normal-vm").with_priority(HAPriority::Normal));
    manager.add_config(
        HAConfig::new("disabled-vm")
            .with_priority(HAPriority::High)
            .with_eviction_strategy(EvictionStrategy::None),
    );

    // Priority sorting
    let sorted = manager.get_by_priority();
    assert_eq!(sorted[0].vm_name, "critical-vm");
    assert_eq!(sorted[1].vm_name, "disabled-vm"); // High priority
    assert_eq!(sorted[2].vm_name, "normal-vm");
    assert_eq!(sorted[3].vm_name, "low-vm");

    // Migration checks
    assert!(manager.should_migrate_on_failure("critical-vm"));
    assert!(!manager.should_migrate_on_failure("disabled-vm")); // EvictionStrategy::None

    // Restart checks
    assert!(manager.should_restart("critical-vm", 0));
    assert!(manager.should_restart("critical-vm", 2));
    assert!(!manager.should_restart("critical-vm", 5)); // Exceeds max_restart_attempts (3)
    assert!(!manager.should_restart("nonexistent-vm", 0));
}

// ========== WORKFLOW EXECUTION ==========

#[test]
fn test_workflow_creation_and_execution() {
    use vmrogue::automation::workflows::{Workflow, WorkflowExecutor, WorkflowStep};
    use vmrogue::automation::{Action, ActionType};

    let workflow = Workflow::new("deploy-workflow")
        .with_description("Deploy and verify")
        .add_step(WorkflowStep::new(
            1,
            "create-snapshot",
            Action::new(ActionType::CreateSnapshot {
                vm_name: "test-vm".to_string(),
                snapshot_name: Some("pre-deploy".to_string()),
            }),
        ))
        .add_step(
            WorkflowStep::new(
                2,
                "restart-vm",
                Action::new(ActionType::RestartVM {
                    vm_name: "test-vm".to_string(),
                }),
            )
            .depends_on(1),
        );

    assert_eq!(workflow.name, "deploy-workflow");
    assert_eq!(workflow.step_count(), 2);

    // Execute the workflow
    let execution = WorkflowExecutor::execute(&workflow);
    assert!(execution.completed_at.is_some());
    assert_eq!(execution.step_results.len(), 2);
}

// ========== MIGRATION STRATEGY ==========

#[test]
fn test_migration_strategy_node_scoring() {
    use vmrogue::migration::strategy::{
        MigrationStrategy, NodeInfo, NodeSelector, SelectionCriterion,
    };

    let selector = NodeSelector::new()
        .add_criterion(SelectionCriterion::MinimumMemory(4 * 1024 * 1024 * 1024))
        .add_criterion(SelectionCriterion::MinimumCPU(2000))
        .add_criterion(SelectionCriterion::PreferLowLoad);

    let strategy = MigrationStrategy::new(selector);

    let nodes = vec![
        NodeInfo::new("node-big")
            .with_resources(32 * 1024 * 1024 * 1024, 16000)
            .with_load(0.1),
        NodeInfo::new("node-medium")
            .with_resources(16 * 1024 * 1024 * 1024, 8000)
            .with_load(0.5),
        NodeInfo::new("node-small")
            .with_resources(2 * 1024 * 1024 * 1024, 1000)
            .with_load(0.1),
        NodeInfo::new("node-busy")
            .with_resources(16 * 1024 * 1024 * 1024, 8000)
            .with_load(0.9),
    ];

    let target = strategy.select_target(&nodes);
    assert!(target.is_some());
    // node-big has most resources and lowest load
    assert_eq!(target.unwrap(), "node-big");

    let ranked = strategy.rank_nodes(&nodes);
    // node-small should be excluded (insufficient resources)
    assert!(!ranked.iter().any(|(name, _)| name == "node-small"));
    // All others should be present
    assert_eq!(ranked.len(), 3);
    // First should be highest score (node-big: low load, high resources)
    assert_eq!(ranked[0].0, "node-big");
}

// ========== OBSERVABILITY METRICS ==========

#[test]
fn test_vm_metrics_collection_and_aggregation() {
    use vmrogue::observability::metrics::{
        Metric, MetricAggregator, MetricCollector, MetricType,
    };

    // Create and record metrics
    let mut collector = MetricCollector::new();
    collector.gauge("cpu_usage", 45.0);
    collector.gauge("memory_usage", 72.5);
    collector.counter("requests_total", 1500.0);
    collector.histogram("request_latency", 0.25);

    let metrics = collector.get_metrics();
    assert_eq!(metrics.len(), 4);

    let cpu = metrics.iter().find(|m| m.name == "cpu_usage").unwrap();
    assert_eq!(cpu.metric_type, MetricType::Gauge);
    assert!((cpu.value - 45.0).abs() < 0.01);

    let mem = metrics.iter().find(|m| m.name == "memory_usage").unwrap();
    assert!((mem.value - 72.5).abs() < 0.01);

    // Aggregation
    let values = vec![
        Metric::new("test", MetricType::Gauge, 10.0),
        Metric::new("test", MetricType::Gauge, 20.0),
        Metric::new("test", MetricType::Gauge, 30.0),
        Metric::new("test", MetricType::Gauge, 40.0),
        Metric::new("test", MetricType::Gauge, 50.0),
    ];

    let avg = MetricAggregator::average(&values);
    assert!((avg - 30.0).abs() < 0.01);

    let max = MetricAggregator::max(&values).unwrap();
    assert!((max - 50.0).abs() < 0.01);

    let min = MetricAggregator::min(&values).unwrap();
    assert!((min - 10.0).abs() < 0.01);

    let sum = MetricAggregator::sum(&values);
    assert!((sum - 150.0).abs() < 0.01);
}

// ========== SCHEDULING TESTS ==========

#[test]
fn test_backup_schedule_types() {
    use vmrogue::backup::schedule::{BackupSchedule, ScheduleType};

    let hourly = BackupSchedule::new("hourly-backup", ScheduleType::hourly(30));
    assert!(hourly.enabled);
    let next = hourly.calculate_next_run(chrono::Utc::now());
    assert!(next.is_some());

    let disabled = BackupSchedule::new("disabled", ScheduleType::daily(2, 0)).disable();
    assert!(!disabled.enabled);
    assert!(disabled.calculate_next_run(chrono::Utc::now()).is_none());
}

#[test]
fn test_shared_scheduling_functions() {
    use chrono::{NaiveTime, Utc};
    use vmrogue::utils::schedule::{last_day_of_month, next_daily, next_hourly};

    // February in a leap year
    assert_eq!(last_day_of_month(2024, 2), 29);
    // February in a non-leap year
    assert_eq!(last_day_of_month(2023, 2), 28);
    // December
    assert_eq!(last_day_of_month(2024, 12), 31);

    let now = Utc::now();
    let next_h = next_hourly(now, 30);
    assert!(next_h > now);

    let time = NaiveTime::from_hms_opt(3, 0, 0).unwrap();
    let next_d = next_daily(now, &time);
    assert!(next_d > now);
}

// ========== ERROR TYPE TESTS ==========

#[test]
fn test_vmrogue_error_display() {
    use vmrogue::VMRogueError;

    let not_found = VMRogueError::VmNotFound("my-vm".to_string());
    assert_eq!(not_found.to_string(), "VM 'my-vm' not found");

    let exists = VMRogueError::VmExists("my-vm".to_string());
    assert_eq!(exists.to_string(), "VM 'my-vm' already exists");
}

// ========== BUILDER EDGE CASES ==========

#[test]
fn test_builder_with_all_options() {
    let config = VMConfigBuilder::new("full-vm")
        .namespace("production")
        .cpu(4, 2, 2)
        .memory("16Gi")
        .add_container_disk("boot", "registry.io/image:v1", 1)
        .add_blank_disk("data", "100Gi", 2)
        .add_pod_network("eth0")
        .label("app", "database")
        .label("env", "prod")
        .annotation("description", "Production database")
        .cloud_init("#cloud-config\npackages:\n  - vim")
        .build();

    assert_eq!(config.name, "full-vm");
    assert_eq!(config.namespace, "production");
    assert_eq!(config.cpu.cores, 4);
    assert_eq!(config.cpu.sockets, 2);
    assert_eq!(config.cpu.threads, 2);
    assert_eq!(config.memory.size, "16Gi");
    assert_eq!(config.disks.len(), 2);
    assert_eq!(config.interfaces.len(), 1);
    assert_eq!(config.labels.len(), 2);
    assert_eq!(config.annotations.len(), 1);
    assert!(config.cloud_init.is_some());

    // Should pass validation
    validate_vm_config(&config).unwrap();

    // Should convert to KubeVirt
    let vm = vm_config_to_kubevirt(&config).unwrap();
    let volumes = vm.spec.template.spec.volumes.as_ref().unwrap();
    assert!(volumes.len() >= 3); // boot + data + cloudinit
}

#[test]
fn test_builder_validated() {
    // Valid config (needs disk + interface to pass validation)
    let result = VMConfigBuilder::new("valid-vm")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .add_blank_disk("root", "20Gi", 1)
        .add_pod_network("eth0")
        .build_validated();
    assert!(result.is_ok(), "Validation error: {:?}", result.err());

    // Invalid: empty name
    let result = VMConfigBuilder::new("")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build_validated();
    assert!(result.is_err());
}

// ========== HEALTH CHECK TESTS ==========

#[test]
fn test_health_check_report() {
    use vmrogue::health::{HealthCheck, HealthStatus, VMHealthReport};

    let mut report = VMHealthReport::new("test-vm".to_string());
    assert_eq!(report.vm_name, "test-vm");

    report.add_check(HealthCheck {
        name: "CPU".to_string(),
        status: HealthStatus::Healthy,
        message: "CPU usage normal".to_string(),
        recommendation: None,
    });
    report.add_check(HealthCheck {
        name: "Memory".to_string(),
        status: HealthStatus::Warning,
        message: "Memory at 78%".to_string(),
        recommendation: Some("Consider increasing memory".to_string()),
    });

    assert_eq!(report.overall_status, HealthStatus::Warning);
    assert_eq!(report.checks.len(), 2);
}

// ========== DISK MANAGEMENT TESTS ==========

#[test]
fn test_disk_info_parse_and_format() {
    use vmrogue::disk::DiskInfo;

    // Parse various sizes
    assert_eq!(DiskInfo::parse_size("10Gi"), 10 * 1024 * 1024 * 1024);
    assert_eq!(DiskInfo::parse_size("100Mi"), 100 * 1024 * 1024);
    assert_eq!(DiskInfo::parse_size("1Ti"), 1024 * 1024 * 1024 * 1024);
    assert_eq!(DiskInfo::parse_size("10G"), 10_000_000_000);

    // Invalid input returns 0
    assert_eq!(DiskInfo::parse_size("invalid"), 0);
    assert_eq!(DiskInfo::parse_size(""), 0);

    // Needs expansion check
    let mut disk = DiskInfo::new("test-disk");
    disk.usage_percent = 85.0;
    assert!(disk.needs_expansion(80.0));
    assert!(!disk.needs_expansion(90.0));
}

// ========== CRON EXPRESSION TESTS ==========

#[test]
fn test_cron_expression_parsing() {
    use chrono::Utc;
    use vmrogue::utils::cron::next_cron_time;

    let now = Utc::now();

    // Every minute should find next time quickly
    let next = next_cron_time(now, "* * * * *");
    assert!(next.is_some());
    let next_time = next.unwrap();
    assert!(next_time > now);

    // Specific time
    let next = next_cron_time(now, "30 2 * * *");
    assert!(next.is_some());
}

// ========== FORMAT BYTES UTILITY TESTS ==========

#[test]
fn test_format_bytes_utility() {
    use vmrogue::format_bytes;

    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1024), "1.00 KiB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    assert_eq!(format_bytes(1024u64 * 1024 * 1024 * 1024), "1.00 TiB");
}

// ========== GENERATE_ID UTILITY TESTS ==========

#[test]
fn test_generate_id_utility() {
    use vmrogue::generate_id;

    let id1 = generate_id("vm", "test");
    let id2 = generate_id("vm", "test");
    assert!(id1.starts_with("vm-"));
    assert!(id2.starts_with("vm-"));
    assert_ne!(id1, id2); // Should be unique

    let snap_id = generate_id("snap", "backup");
    assert!(snap_id.starts_with("snap-"));
}

// ========== PERCENT_TO_U8 UTILITY TESTS ==========

#[test]
fn test_percent_to_u8_utility() {
    use vmrogue::percent_to_u8;

    assert_eq!(percent_to_u8(0.0), 0);
    assert_eq!(percent_to_u8(50.0), 50);
    assert_eq!(percent_to_u8(100.0), 100);
    assert_eq!(percent_to_u8(150.0), 100); // Clamped
    assert_eq!(percent_to_u8(-10.0), 0); // Clamped
    assert_eq!(percent_to_u8(f64::INFINITY), 100);
    // NaN behavior: NaN comparisons return false, so it falls to else branch
    let nan_result = percent_to_u8(f64::NAN);
    assert!(nan_result <= 100); // Just ensure no panic
}

// ========== KUBEVIRT CONVERSION EDGE CASES ==========

#[test]
fn test_kubevirt_conversion_preserves_labels() {
    let config = VMConfigBuilder::new("label-test")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .label("env", "prod")
        .label("team", "platform")
        .build();

    let vm = vm_config_to_kubevirt(&config).unwrap();
    let labels = vm.metadata.labels.unwrap();
    assert_eq!(labels.get("env"), Some(&"prod".to_string()));
    assert_eq!(labels.get("team"), Some(&"platform".to_string()));
    // Auto-added kubevirt.io/vm label
    assert_eq!(
        labels.get("kubevirt.io/vm"),
        Some(&"label-test".to_string())
    );
}

#[test]
fn test_kubevirt_conversion_sets_running_false() {
    let config = VMConfigBuilder::new("run-test")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();

    let vm = vm_config_to_kubevirt(&config).unwrap();
    assert_eq!(vm.spec.running, Some(false));
}

#[test]
fn test_validation_rejects_name_with_leading_hyphen() {
    let config = VMConfigBuilder::new("-invalid")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();
    assert!(validate_vm_config(&config).is_err());
}

#[test]
fn test_validation_rejects_name_with_trailing_dot() {
    let config = VMConfigBuilder::new("invalid.")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();
    assert!(validate_vm_config(&config).is_err());
}

#[test]
fn test_validation_rejects_name_with_uppercase() {
    let config = VMConfigBuilder::new("InvalidName")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .build();
    assert!(validate_vm_config(&config).is_err());
}

#[test]
fn test_validation_accepts_name_with_dots_and_hyphens() {
    let config = VMConfigBuilder::new("my-vm.test.01")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("1Gi")
        .add_blank_disk("root", "20Gi", 1)
        .add_pod_network("eth0")
        .build();
    assert!(
        validate_vm_config(&config).is_ok(),
        "Should accept dots and hyphens in name"
    );
}

// ========== MULTUS NETWORK TESTS ==========

#[test]
fn test_multus_network_kubevirt_conversion() {
    let config = VMConfigBuilder::new("multus-test")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("2Gi")
        .add_blank_disk("root", "20Gi", 1)
        .add_pod_network("eth0")
        .add_multus_network("net1", "my-bridge-net")
        .build();

    let vm = vm_config_to_kubevirt(&config).unwrap();

    // Check interfaces: pod should use masquerade, multus should use bridge
    let devices = vm.spec.template.spec.domain.devices.as_ref().unwrap();
    let interfaces = devices.interfaces.as_ref().unwrap();
    assert_eq!(interfaces.len(), 2);
    assert!(interfaces[0].masquerade.is_some()); // Pod uses masquerade
    assert!(interfaces[1].bridge.is_some()); // Multus uses bridge

    // Check networks
    let networks = vm.spec.template.spec.networks.as_ref().unwrap();
    assert_eq!(networks.len(), 2);
    assert!(networks[0].pod.is_some()); // Pod network
    assert!(networks[1].multus.is_some()); // Multus network
    assert_eq!(
        networks[1].multus.as_ref().unwrap().network_name,
        "my-bridge-net"
    );
}

#[test]
fn test_bridge_network_kubevirt_conversion() {
    let config = VMConfigBuilder::new("bridge-test")
        .namespace("default")
        .cpu(1, 1, 1)
        .memory("2Gi")
        .add_blank_disk("root", "20Gi", 1)
        .add_bridge_network("br0")
        .build();

    let vm = vm_config_to_kubevirt(&config).unwrap();

    let devices = vm.spec.template.spec.domain.devices.as_ref().unwrap();
    let interfaces = devices.interfaces.as_ref().unwrap();
    assert_eq!(interfaces.len(), 1);
    assert!(interfaces[0].bridge.is_some()); // Bridge binding
    assert!(interfaces[0].masquerade.is_none());
}
