// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Create PVC + hot-plug data disk to a KubeVirt VM (Windows D:/E: or Linux /mnt/data).

use anyhow::{Result, anyhow};
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use kube::api::{Api, Patch, PatchParams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::sleep;

use super::KubeClient;
use super::guest_os::{GuestOsFamily, detect_guest_os_family};

#[derive(Debug, Deserialize)]
pub struct AddDataDiskRequest {
    pub disk_name: Option<String>,
    pub pvc_name: Option<String>,
    pub size_gi: u32,
    pub storage_class: Option<String>,
    pub bus: Option<String>,
    pub drive_letter: Option<String>,
    pub mount_path: Option<String>,
    pub filesystem: Option<String>,
    #[serde(default = "default_true")]
    pub wait_bound: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct GuestDiskInit {
    pub summary: String,
    pub steps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powershell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_letter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DataDiskDefaults {
    pub suggested_disk_name: String,
    pub suggested_pvc_name: String,
    pub suggested_bus: String,
    pub guest_os: String,
    pub is_windows: bool,
    pub is_linux: bool,
    pub root_disk_bus: Option<String>,
    pub storage_class: Option<String>,
    pub storage_classes: Vec<String>,
    pub suggested_mount_path: Option<String>,
    pub suggested_filesystem: Option<String>,
    pub default_size_gi: u32,
}

#[derive(Debug, Serialize)]
pub struct AddDataDiskResponse {
    pub success: bool,
    pub message: String,
    pub pvc_name: String,
    pub disk_name: String,
    pub bus: String,
    pub size: String,
    pub guest_os: String,
    pub is_windows: bool,
    pub is_linux: bool,
    pub guest_init: GuestDiskInit,
}

pub fn suggest_next_disk_name(family: GuestOsFamily, existing: &[String]) -> String {
    match family {
        GuestOsFamily::Linux => {
            for n in 1..=99 {
                let candidate = format!("data-{n}");
                if !existing.iter().any(|name| name == &candidate) {
                    return candidate;
                }
            }
            "data-extra".into()
        }
        GuestOsFamily::Windows | GuestOsFamily::Unknown => {
            let letters = ['d', 'e', 'f', 'g', 'h', 'i', 'j', 'k'];
            for letter in letters {
                let candidate = format!("data-{}", letter);
                if !existing.iter().any(|n| n == &candidate) {
                    return candidate;
                }
            }
            let mut n = 2u32;
            loop {
                let candidate = format!("data-{}", n);
                if !existing.iter().any(|name| name == &candidate) {
                    return candidate;
                }
                n += 1;
            }
        }
    }
}

impl KubeClient {
    pub async fn data_disk_defaults(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<DataDiskDefaults> {
        let ctx = self.load_storage_context(namespace, vm_name).await?;
        let suggested_disk_name = suggest_next_disk_name(ctx.guest_os, &ctx.existing_volume_names);
        let storage_classes = self.list_storage_class_names().await?;
        let (suggested_mount_path, suggested_filesystem) =
            linux_mount_defaults(ctx.guest_os, &suggested_disk_name);
        Ok(DataDiskDefaults {
            suggested_disk_name: suggested_disk_name.clone(),
            suggested_pvc_name: format!("{vm_name}-{suggested_disk_name}"),
            suggested_bus: ctx.suggested_bus.clone(),
            guest_os: ctx.guest_os.as_str().to_string(),
            is_windows: ctx.guest_os == GuestOsFamily::Windows,
            is_linux: ctx.guest_os == GuestOsFamily::Linux,
            root_disk_bus: ctx.root_disk_bus,
            storage_class: ctx.storage_class.clone(),
            storage_classes,
            suggested_mount_path,
            suggested_filesystem: suggested_filesystem.map(str::to_string),
            default_size_gi: default_size_gi(ctx.guest_os),
        })
    }

    pub async fn add_data_disk(
        &self,
        namespace: &str,
        vm_name: &str,
        req: AddDataDiskRequest,
    ) -> Result<AddDataDiskResponse> {
        if req.size_gi == 0 {
            return Err(anyhow!("size_gi must be at least 1"));
        }

        let ctx = self.load_storage_context(namespace, vm_name).await?;
        let disk_name = req
            .disk_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| suggest_next_disk_name(ctx.guest_os, &ctx.existing_volume_names));

        let pvc_name = req
            .pvc_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{vm_name}-{disk_name}"));

        if ctx.existing_volume_names.iter().any(|n| n == &disk_name) {
            return Err(anyhow!("Volume '{}' already attached", disk_name));
        }

        let bus = req
            .bus
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| ctx.suggested_bus.clone());

        let storage_class = req
            .storage_class
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| ctx.storage_class.clone());

        let size = format!("{}Gi", req.size_gi);

        self.create_pvc(namespace, &pvc_name, &size, storage_class.as_deref())
            .await?;

        if req.wait_bound {
            self.wait_pvc_bound(namespace, &pvc_name, Duration::from_secs(120))
                .await?;
        }

        self.patch_vm_hotplug_disk(namespace, vm_name, &disk_name, &pvc_name, &bus)
            .await?;

        let guest_init = guest_init_hints(
            ctx.guest_os,
            req.drive_letter.as_deref(),
            req.mount_path.as_deref(),
            req.filesystem.as_deref(),
        );

        Ok(AddDataDiskResponse {
            success: true,
            message: format!(
                "Created PVC {pvc_name} and attached disk '{disk_name}' ({bus}, {size})"
            ),
            pvc_name,
            disk_name,
            bus,
            size,
            guest_os: ctx.guest_os.as_str().to_string(),
            is_windows: ctx.guest_os == GuestOsFamily::Windows,
            is_linux: ctx.guest_os == GuestOsFamily::Linux,
            guest_init,
        })
    }

    async fn load_storage_context(&self, namespace: &str, name: &str) -> Result<StorageContext> {
        let vm = self.get_vm(namespace, name).await?;
        let vm_json = serde_json::to_value(&vm)?;

        let vmi_json = self
            .get_vmi(namespace, name)
            .await
            .ok()
            .and_then(|vmi| serde_json::to_value(vmi).ok());

        let existing_volume_names = vm_json
            .pointer("/spec/template/spec/volumes")
            .and_then(|v| v.as_array())
            .map(|vols| {
                vols.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let root_disk_bus = detect_root_disk_bus(&vm_json);
        let guest_os = detect_guest_os_family(Some(&vm_json), vmi_json.as_ref());
        let suggested_bus = bus_or_default(None, root_disk_bus.as_deref(), guest_os);
        let storage_class = root_pvc_storage_class(self, namespace, &vm_json).await;

        Ok(StorageContext {
            existing_volume_names,
            suggested_bus,
            guest_os,
            root_disk_bus,
            storage_class,
        })
    }

    async fn list_storage_class_names(&self) -> Result<Vec<String>> {
        use k8s_openapi::api::storage::v1::StorageClass;
        let sc_api: Api<StorageClass> = Api::all(self.client.clone());
        let list = sc_api
            .list(&kube::api::ListParams::default().limit(200))
            .await?;
        let mut names: Vec<String> = list
            .items
            .into_iter()
            .filter_map(|sc| sc.metadata.name)
            .collect();
        names.sort();
        Ok(names)
    }

    async fn wait_pvc_bound(&self, namespace: &str, name: &str, timeout: Duration) -> Result<()> {
        let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), namespace);
        let started = std::time::Instant::now();
        loop {
            let pvc = pvc_api.get(name).await?;
            let phase = pvc
                .status
                .as_ref()
                .and_then(|s| s.phase.clone())
                .unwrap_or_default();
            if phase == "Bound" {
                return Ok(());
            }
            if started.elapsed() >= timeout {
                return Err(anyhow!(
                    "PVC {name} not Bound within {:?} (phase={phase})",
                    timeout
                ));
            }
            sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn patch_vm_hotplug_disk(
        &self,
        namespace: &str,
        vm_name: &str,
        disk_name: &str,
        pvc_name: &str,
        bus: &str,
    ) -> Result<()> {
        use super::VirtualMachine;

        let vms: Api<VirtualMachine> = self.vm_api(namespace);
        let vm = vms.get(vm_name).await?;
        let vm_json = serde_json::to_value(&vm)?;

        let mut volumes: Vec<Value> = vm_json
            .pointer("/spec/template/spec/volumes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut disks: Vec<Value> = vm_json
            .pointer("/spec/template/spec/domain/devices/disks")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        if volumes
            .iter()
            .any(|v| v.get("name").and_then(|n| n.as_str()) == Some(disk_name))
        {
            return Err(anyhow!("Volume '{}' already exists", disk_name));
        }

        volumes.push(json!({
            "name": disk_name,
            "persistentVolumeClaim": {
                "claimName": pvc_name,
                "hotpluggable": true
            }
        }));

        disks.push(json!({
            "name": disk_name,
            "disk": { "bus": bus },
            "cache": "none"
        }));

        let patch = json!({
            "spec": {
                "template": {
                    "spec": {
                        "volumes": volumes,
                        "domain": {
                            "devices": {
                                "disks": disks
                            }
                        }
                    }
                }
            }
        });

        let pp = PatchParams::apply("veyron");
        vms.patch(vm_name, &pp, &Patch::Merge(patch)).await?;
        Ok(())
    }
}

struct StorageContext {
    existing_volume_names: Vec<String>,
    suggested_bus: String,
    guest_os: GuestOsFamily,
    root_disk_bus: Option<String>,
    storage_class: Option<String>,
}

fn detect_root_disk_bus(vm: &Value) -> Option<String> {
    let disks = vm
        .pointer("/spec/template/spec/domain/devices/disks")?
        .as_array()?;
    let root = disks
        .iter()
        .find(|d| d.get("name").and_then(|n| n.as_str()) == Some("rootdisk"))
        .or_else(|| disks.first())?;
    root.get("disk")
        .and_then(|d| d.get("bus"))
        .and_then(|b| b.as_str())
        .map(str::to_string)
}

fn bus_or_default(
    requested: Option<&str>,
    root_bus: Option<&str>,
    family: GuestOsFamily,
) -> String {
    if let Some(b) = requested.filter(|s| !s.is_empty()) {
        return b.to_string();
    }
    if let Some(b) = root_bus.filter(|s| !s.is_empty()) {
        return b.to_string();
    }
    match family {
        GuestOsFamily::Windows => "sata".to_string(),
        GuestOsFamily::Linux | GuestOsFamily::Unknown => "virtio".to_string(),
    }
}

fn linux_mount_defaults(
    family: GuestOsFamily,
    disk_name: &str,
) -> (Option<String>, Option<&'static str>) {
    if family != GuestOsFamily::Linux {
        return (None, None);
    }
    let mount = if disk_name == "data-1" {
        "/mnt/data".to_string()
    } else if let Some(n) = disk_name.strip_prefix("data-") {
        if n.chars().all(|c| c.is_ascii_digit()) {
            format!("/mnt/data{n}")
        } else {
            format!("/mnt/{disk_name}")
        }
    } else {
        format!("/mnt/{disk_name}")
    };
    (Some(mount), Some("ext4"))
}

fn default_size_gi(family: GuestOsFamily) -> u32 {
    match family {
        GuestOsFamily::Linux => 50,
        GuestOsFamily::Windows => 100,
        GuestOsFamily::Unknown => 50,
    }
}

async fn root_pvc_storage_class(
    client: &KubeClient,
    namespace: &str,
    vm: &Value,
) -> Option<String> {
    let claim = vm
        .pointer("/spec/template/spec/volumes")?
        .as_array()?
        .iter()
        .find_map(|v| {
            v.get("persistentVolumeClaim")
                .and_then(|p| p.get("claimName"))
                .and_then(|c| c.as_str())
        })?;
    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.client(), namespace);
    pvc_api
        .get(claim)
        .await
        .ok()
        .and_then(|pvc| pvc.spec.and_then(|s| s.storage_class_name))
}

fn guest_init_hints(
    family: GuestOsFamily,
    drive_letter: Option<&str>,
    mount_path: Option<&str>,
    filesystem: Option<&str>,
) -> GuestDiskInit {
    match family {
        GuestOsFamily::Windows => {
            let letter = drive_letter.unwrap_or("D");
            let ps = format!(
                r##"# Run in elevated PowerShell after the new disk appears
Get-Disk | Where-Object PartitionStyle -eq 'RAW' | Format-Table Number, Size, FriendlyName
$disk = Get-Disk | Where-Object PartitionStyle -eq 'RAW' | Select-Object -First 1
if ($disk) {{
  Initialize-Disk -Number $disk.Number -PartitionStyle GPT
  New-Partition -DiskNumber $disk.Number -UseMaximumSize -DriveLetter {letter}
  Format-Volume -DriveLetter {letter} -FileSystem NTFS -NewFileSystemLabel ('Data-' + '{letter}') -Confirm:$false
}}
Get-Volume {letter}"##,
                letter = letter
            );
            GuestDiskInit {
                summary: format!(
                    "Disk attached in Kubernetes. In Windows (RDP), initialize and format as drive {letter}:."
                ),
                steps: vec![
                    "Open Disk Management (Win+X → Disk Management).".into(),
                    "Select the new disk → Initialize (GPT) → OK.".into(),
                    format!(
                        "New Simple Volume → assign drive letter {letter}: → NTFS quick format."
                    ),
                    "Or run the PowerShell snippet below in an elevated session.".into(),
                ],
                powershell: Some(ps),
                drive_letter: Some(letter.to_string()),
                shell_script: None,
                mount_path: None,
                filesystem: None,
            }
        }
        GuestOsFamily::Linux | GuestOsFamily::Unknown => {
            let mount = mount_path.filter(|s| !s.is_empty()).unwrap_or("/mnt/data");
            let fs = match filesystem.map(str::trim).filter(|s| !s.is_empty()) {
                Some("xfs") | Some("XFS") => "xfs",
                _ => "ext4",
            };
            let shell = format!(
                r#"#!/bin/bash
set -euo pipefail
MOUNT={mount:?}
FS={fs:?}
DISK=$(lsblk -dpno NAME,TYPE | awk '$2=="disk" {{print $1}}' | tail -1)
[ -b "$DISK" ] || {{ echo "No disk; set DISK=/dev/vdX" >&2; exit 1; }}
parted -s "$DISK" mklabel gpt mkpart primary 0% 100%
PART="${{DISK}}1"
[[ "$DISK" == *"nvme"* ]] && PART="${{DISK}}p1"
if [ "$FS" = "xfs" ]; then mkfs.xfs -f "$PART"; else mkfs.ext4 -F "$PART"; fi
mkdir -p "$MOUNT"
grep -q "$PART" /etc/fstab || echo "$PART $MOUNT $FS defaults 0 2" >> /etc/fstab
mount "$PART" "$MOUNT"
df -h "$MOUNT"
"#,
                mount = mount,
                fs = fs
            );
            GuestDiskInit {
                summary: format!(
                    "Disk attached. SSH into the VM and run the script below to mount at {mount}."
                ),
                steps: vec![
                    "Confirm the new disk: lsblk".into(),
                    "Run the bash script below as root (or with sudo).".into(),
                    format!("Verify: df -h {mount}"),
                ],
                powershell: None,
                drive_letter: None,
                shell_script: Some(shell),
                mount_path: Some(mount.to_string()),
                filesystem: Some(fs.to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_data_d_then_e() {
        assert_eq!(
            suggest_next_disk_name(GuestOsFamily::Windows, &[]),
            "data-d"
        );
    }

    #[test]
    fn suggests_data_1_for_linux() {
        assert_eq!(suggest_next_disk_name(GuestOsFamily::Linux, &[]), "data-1");
    }
}
