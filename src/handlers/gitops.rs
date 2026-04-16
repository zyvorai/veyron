use crate::tui::colors::cli as color;
use anyhow::Result;

/// Export all VMs as VMRogueVM CRD manifests to a directory.
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
        let vm_name = vm
            .metadata
            .name
            .as_deref()
            .unwrap_or("unknown");
        let vm_namespace = vm
            .metadata
            .namespace
            .as_deref()
            .unwrap_or(namespace);

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
            .and_then(|l| l.get("vmrogue.io/template").cloned())
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
                                    vol.container_disk
                                        .as_ref()
                                        .map(|cd| cd.image.clone()),
                                )
                            } else if vol.persistent_volume_claim.is_some() {
                                ("pvc".to_string(), None)
                            } else if vol.data_volume.is_some() {
                                ("dataVolume".to_string(), None)
                            } else if vol.empty_disk.is_some() {
                                ("blank".to_string(), None)
                            } else {
                                ("blank".to_string(), None)
                            }
                        } else {
                            ("blank".to_string(), None)
                        };

                        let size = volume
                            .and_then(|v| v.empty_disk.as_ref().map(|e| e.capacity.clone()))
                            .unwrap_or_else(|| "20Gi".to_string());

                        let boot_order = d
                            .boot_order
                            .unwrap_or(0);

                        CRDDiskSpec {
                            name: disk_name,
                            size,
                            storage_class: None,
                            boot_order,
                            source: CRDDiskSource {
                                source_type,
                                name: None,
                                image: source_image,
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
            "apiVersion": "vmrogue.io/v1alpha1",
            "kind": "VMRogueVM",
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
            "Exported {} VM(s) as VMRogueVM CRD manifests:",
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_handler_module_compiles() {
        // Verify the module compiles correctly
        assert!(true);
    }
}
