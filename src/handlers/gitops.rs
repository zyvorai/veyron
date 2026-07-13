// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use crate::tui::colors::cli as color;
use anyhow::Result;

/// Export all VMs as VeyronVM CRD manifests to a directory.
pub async fn handle_gitops_export(directory: String, namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;
    use crate::operator_crds::{CRDDiskSource, CRDDiskSpec};

    println!("{}", color::header("GitOps Export"));
    println!("  Directory: {}", color::value(&directory));
    println!("  Namespace: {}", color::value(namespace));
    println!();

    // Ensure output directory exists
    let dir_path = std::path::Path::new(&directory);
    if !dir_path.exists() {
        std::fs::create_dir_all(dir_path)?;
        println!(
            "  {}",
            color::info(&format!("Created directory: {}", directory))
        );
    }

    let client = KubeClient::new().await?;
    let vms = client.list_all_vms().await?;

    if vms.is_empty() {
        println!("  {}", color::muted("No VMs found to export."));
        return Ok(());
    }

    let mut exported_files = Vec::new();

    for vm in &vms {
        let vm_name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let vm_namespace = vm.metadata.namespace.as_deref().unwrap_or(namespace);

        // Extract CPU cores from the VM spec
        let cpu_cores = vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .map(|c| c.cores.unwrap_or(1))
            .unwrap_or(1);

        // Extract memory from VM spec
        let memory_size = vm
            .spec
            .template
            .spec
            .domain
            .memory
            .as_ref()
            .and_then(|m| m.guest.as_deref())
            .or_else(|| {
                vm.spec
                    .template
                    .spec
                    .domain
                    .resources
                    .requests
                    .as_ref()
                    .and_then(|r| r.get("memory"))
                    .map(|v| v.as_str())
            })
            .unwrap_or("2Gi")
            .to_string();

        // Extract template name from labels if available
        let template_name = vm
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("veyron.io/template").cloned())
            .or_else(|| {
                vm.metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("vm.kubevirt.io/template").cloned())
            });

        // Build disks config from VM volumes
        if vm.spec.template.spec.domain.devices.is_none() {
            log::warn!(
                "VM '{}' has no devices section — disk info will be empty in exported manifest",
                vm_name
            );
        }
        let disks: Vec<CRDDiskSpec> = vm
            .spec
            .template
            .spec
            .domain
            .devices
            .as_ref()
            .and_then(|d| d.disks.as_ref())
            .map(|disk_list| {
                disk_list
                    .iter()
                    .map(|d| {
                        let disk_name = d.name.clone();
                        // Try to find a matching volume for sizing info
                        let volume = vm
                            .spec
                            .template
                            .spec
                            .volumes
                            .as_ref()
                            .and_then(|vols| vols.iter().find(|v| v.name == disk_name));

                        let (source_type, source_image) = if let Some(vol) = volume {
                            if vol.container_disk.is_some() {
                                (
                                    "containerDisk".to_string(),
                                    vol.container_disk.as_ref().map(|cd| cd.image.clone()),
                                )
                            } else if vol.persistent_volume_claim.is_some() {
                                ("pvc".to_string(), None)
                            } else if vol.data_volume.is_some() {
                                ("dataVolume".to_string(), None)
                            } else {
                                ("blank".to_string(), None)
                            }
                        } else {
                            ("blank".to_string(), None)
                        };

                        let size = volume
                            .and_then(|v| v.empty_disk.as_ref().map(|e| e.capacity.clone()))
                            .unwrap_or_else(|| "20Gi".to_string());

                        let boot_order = d.boot_order.unwrap_or(0);

                        CRDDiskSpec {
                            name: disk_name,
                            size,
                            storage_class: None,
                            boot_order,
                            source: CRDDiskSource {
                                source_type,
                                name: None,
                                image: source_image,
                                namespace: None,
                                from_pvc: false,
                            },
                            device_type: "disk".to_string(),
                            bus: d.disk.as_ref().and_then(|dd| dd.bus.clone()),
                            cache: None,
                            io: None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build the CRD manifest as JSON value
        let crd = serde_json::json!({
            "apiVersion": "veyron.io/v1alpha1",
            "kind": "VeyronVM",
            "metadata": {
                "name": vm_name,
                "namespace": vm_namespace,
            },
            "spec": {
                "template": template_name,
                "cpu": { "cores": cpu_cores },
                "memory": { "size": memory_size },
                "disks": disks,
            }
        });

        // Write as YAML file
        let file_name = format!("{}-{}.yaml", vm_namespace, vm_name);
        let file_path = dir_path.join(&file_name);
        let yaml_content = serde_yml::to_string(&crd)?;
        std::fs::write(&file_path, &yaml_content)?;
        exported_files.push(file_name);
    }

    // Print summary
    println!(
        "{}",
        color::success(&format!(
            "Exported {} VM(s) as VeyronVM CRD manifests:",
            exported_files.len()
        ))
    );
    for f in &exported_files {
        println!("  - {}", color::value(f));
    }
    println!();
    println!(
        "{}",
        color::info("These manifests can be committed to a Git repository for GitOps workflows.")
    );

    Ok(())
}

/// Show GitOps diff: compare exported manifests with cluster state.
pub async fn handle_gitops_diff(directory: String, namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;

    println!("{}", color::header("GitOps Diff"));

    let client = KubeClient::new().await?;
    let vms = client.list_vms(namespace).await?;
    let dir_path = std::path::Path::new(&directory);

    if !dir_path.exists() {
        println!(
            "  {}",
            color::warning("Directory does not exist. Run 'veyron gitops-export' first.")
        );
        return Ok(());
    }

    let mut in_cluster_only = Vec::new();
    let mut in_dir_only = Vec::new();
    let mut in_both = 0u32;

    // Cluster VMs
    let cluster_names: std::collections::HashSet<String> = vms
        .iter()
        .filter_map(|vm| vm.metadata.name.clone())
        .collect();

    // Directory manifests
    let mut dir_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "yaml" || e == "yml")
                .unwrap_or(false)
            {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    dir_names.insert(stem.to_string());
                }
            }
        }
    }

    for name in &cluster_names {
        if dir_names.contains(name) {
            in_both += 1;
        } else {
            in_cluster_only.push(name.clone());
        }
    }
    for name in &dir_names {
        if !cluster_names.contains(name) {
            in_dir_only.push(name.clone());
        }
    }

    println!("  Synced:       {} VMs", in_both);
    if !in_cluster_only.is_empty() {
        println!(
            "  {}",
            color::warning(&format!(
                "Cluster only: {} VMs (not exported)",
                in_cluster_only.len()
            ))
        );
        for name in &in_cluster_only {
            println!("    + {}", color::vm_name(name));
        }
    }
    if !in_dir_only.is_empty() {
        println!(
            "  {}",
            color::warning(&format!(
                "Directory only: {} manifests (not in cluster)",
                in_dir_only.len()
            ))
        );
        for name in &in_dir_only {
            println!("    - {}", name);
        }
    }
    if in_cluster_only.is_empty() && in_dir_only.is_empty() {
        println!("  {}", color::success("All VMs are in sync"));
    }

    Ok(())
}

/// Show GitOps sync status.
pub async fn handle_gitops_status(namespace: &str) -> Result<()> {
    use crate::kube::KubeClient;

    println!("{}", color::header("GitOps Status"));

    let client = KubeClient::new().await?;
    let vms = client.list_vms(namespace).await?;

    println!("  Namespace: {}", color::value(namespace));
    println!("  VMs:       {}", color::value(&vms.len().to_string()));

    // Check for common GitOps directories
    let git_dirs = [".", "deploy", "manifests", "gitops"];
    for dir in &git_dirs {
        let path = std::path::Path::new(dir);
        if path.exists() {
            let yaml_count = std::fs::read_dir(path)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "yaml" || ext == "yml")
                                .unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0);
            if yaml_count > 0 {
                println!("  Manifests: {} YAML files in {}/", yaml_count, dir);
            }
        }
    }

    Ok(())
}
