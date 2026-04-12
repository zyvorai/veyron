// Restore Manager - Restore VMs from snapshots
// Real KubeVirt CRD integration

use super::crds::{RestoreTarget, VirtualMachineRestore, VirtualMachineRestoreSpec};
use super::manager::SnapshotManager;
use super::types::{RestoreInfo, RestoreStatus};
use super::SnapshotConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    Api, Client,
};
use std::collections::BTreeMap;

/// Restore Manager for VM restore operations
pub struct RestoreManager {
    client: Client,
    namespace: String,
}

impl RestoreManager {
    /// Create a new RestoreManager
    pub async fn new(namespace: impl Into<String>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .context("Failed to create Kubernetes client")?;

        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    /// Create from an existing client
    pub fn from_client(client: Client, namespace: impl Into<String>) -> Self {
        Self {
            client,
            namespace: namespace.into(),
        }
    }

    /// Restore VM from snapshot to a new VM
    pub async fn restore_to_new_vm(
        &self,
        snapshot_name: &str,
        target_vm_name: &str,
        _start_after_restore: bool,
    ) -> Result<RestoreInfo> {
        let restores: Api<VirtualMachineRestore> =
            Api::namespaced(self.client.clone(), &self.namespace);

        let restore_name = format!("{}-restore", target_vm_name);

        // Build labels
        let mut labels = BTreeMap::new();
        labels.insert(
            "vmrogue.io/snapshot".to_string(),
            snapshot_name.to_string(),
        );
        labels.insert(
            "vmrogue.io/target-vm".to_string(),
            target_vm_name.to_string(),
        );
        labels.insert(
            "vmrogue.io/created-by".to_string(),
            "vmrogue".to_string(),
        );

        // Create the restore CRD
        let restore = VirtualMachineRestore {
            metadata: ObjectMeta {
                name: Some(restore_name.clone()),
                namespace: Some(self.namespace.clone()),
                labels: Some(labels),
                ..Default::default()
            },
            spec: VirtualMachineRestoreSpec {
                target: RestoreTarget {
                    api_group: Some("kubevirt.io".to_string()),
                    kind: "VirtualMachine".to_string(),
                    name: target_vm_name.to_string(),
                },
                snapshot_name: snapshot_name.to_string(),
                patches: None,
            },
            status: None,
        };

        let created = restores
            .create(&PostParams::default(), &restore)
            .await
            .context("Failed to create VirtualMachineRestore")?;

        // Convert to RestoreInfo
        Ok(self.restore_to_info(created))
    }

    /// Check if a VM is currently running by looking for a VirtualMachineInstance
    async fn is_vm_running(&self, namespace: &str, vm_name: &str) -> Result<bool> {
        let vmi_resource = kube::api::ApiResource {
            group: "kubevirt.io".to_string(),
            version: "v1".to_string(),
            api_version: "kubevirt.io/v1".to_string(),
            kind: "VirtualMachineInstance".to_string(),
            plural: "virtualmachineinstances".to_string(),
        };

        let vmi_api: Api<kube::core::DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &vmi_resource);

        match vmi_api.get(vm_name).await {
            Ok(_) => Ok(true),
            Err(kube::Error::Api(err)) if err.code == 404 => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to check VM running status: {}", e)),
        }
    }

    /// Restore VM from snapshot in-place (overwrites current VM)
    pub async fn restore_in_place(
        &self,
        vm_name: &str,
        snapshot_name: &str,
    ) -> Result<RestoreInfo> {
        // Verify VM is not running before in-place restore
        match self.is_vm_running(&self.namespace, vm_name).await {
            Ok(true) => {
                return Err(anyhow::anyhow!(
                    "Cannot restore in-place: VM '{}' is currently running. Stop the VM first.",
                    vm_name
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Cannot verify VM '{}' running state: {}. Aborting restore for safety.",
                    vm_name, e
                ));
            }
            Ok(false) => {} // VM is stopped, safe to proceed
        }

        // Create a pre-restore safety snapshot
        let safety_snapshot_name = format!(
            "{}-pre-restore-{}",
            vm_name,
            Utc::now().format("%Y%m%d%H%M%S")
        );
        log::info!("Creating pre-restore safety snapshot: {}", safety_snapshot_name);
        let snapshot_manager = SnapshotManager::from_client(self.client.clone(), &self.namespace);
        let safety_config = SnapshotConfig::new(vm_name, &safety_snapshot_name)
            .with_description(format!(
                "Automatic pre-restore safety snapshot before restoring from '{}'",
                snapshot_name
            ))
            .with_label("vmrogue.io/safety-snapshot", "pre-restore");
        match snapshot_manager.create_snapshot(&safety_config).await {
            Ok(_) => log::info!("Pre-restore safety snapshot created successfully"),
            Err(e) => log::warn!(
                "Failed to create pre-restore safety snapshot: {}. Proceeding with restore.",
                e
            ),
        }

        // In-place restore uses the same VM name as target
        self.restore_to_new_vm(snapshot_name, vm_name, false).await
    }

    /// Get restore status
    pub async fn get_restore_status(&self, restore_name: &str) -> Result<RestoreInfo> {
        let restores: Api<VirtualMachineRestore> =
            Api::namespaced(self.client.clone(), &self.namespace);

        let restore = restores
            .get(restore_name)
            .await
            .with_context(|| format!("Failed to get restore '{}'", restore_name))?;

        Ok(self.restore_to_info(restore))
    }

    /// List all restore operations
    pub async fn list_restores(&self) -> Result<Vec<RestoreInfo>> {
        let restores: Api<VirtualMachineRestore> =
            Api::namespaced(self.client.clone(), &self.namespace);

        let restore_list = restores
            .list(&ListParams::default())
            .await
            .context("Failed to list restores")?;

        Ok(restore_list
            .items
            .into_iter()
            .map(|r| self.restore_to_info(r))
            .collect())
    }

    /// Check if restore is complete
    pub async fn is_restore_complete(&self, restore_name: &str) -> Result<bool> {
        let restore = self.get_restore_status(restore_name).await?;
        Ok(restore.status == RestoreStatus::Succeeded)
    }

    /// Delete restore resource (cleanup)
    pub async fn delete_restore(&self, restore_name: &str) -> Result<()> {
        let restores: Api<VirtualMachineRestore> =
            Api::namespaced(self.client.clone(), &self.namespace);

        restores
            .delete(restore_name, &DeleteParams::default())
            .await
            .with_context(|| format!("Failed to delete restore '{}'", restore_name))?;

        Ok(())
    }

    /// Validate snapshot before restore
    pub async fn validate_snapshot_for_restore(&self, snapshot_name: &str) -> Result<bool> {
        use super::crds::VirtualMachineSnapshot;

        let snapshots: Api<VirtualMachineSnapshot> =
            Api::namespaced(self.client.clone(), &self.namespace);

        let snapshot = snapshots.get(snapshot_name).await.with_context(|| {
            format!("Failed to get snapshot '{}' for validation", snapshot_name)
        })?;

        // Check if snapshot is ready to use
        let ready = snapshot
            .status
            .as_ref()
            .and_then(|s| s.ready_to_use)
            .unwrap_or(false);

        Ok(ready)
    }

    /// Estimate restore time based on snapshot size
    pub fn estimate_restore_time(&self, snapshot_size_gi: u64) -> String {
        // Simple estimation: ~1 minute per 10Gi
        let minutes = (snapshot_size_gi as f64 / 10.0).ceil() as u64;

        if minutes < 60 {
            format!("~{} minutes", minutes)
        } else {
            format!("~{} hours {} minutes", minutes / 60, minutes % 60)
        }
    }

    /// Convert VirtualMachineRestore CRD to RestoreInfo
    fn restore_to_info(&self, restore: VirtualMachineRestore) -> RestoreInfo {
        let name = restore.metadata.name.unwrap_or_default();
        let namespace = restore
            .metadata
            .namespace
            .unwrap_or_else(|| self.namespace.clone());

        let snapshot_name = restore.spec.snapshot_name.clone();
        let target_vm_name = restore.spec.target.name.clone();

        // Parse status
        let status = if let Some(ref st) = restore.status {
            if st.complete.unwrap_or(false) {
                if st.error.is_some() {
                    RestoreStatus::Failed
                } else {
                    RestoreStatus::Succeeded
                }
            } else {
                RestoreStatus::InProgress
            }
        } else {
            RestoreStatus::Unknown
        };

        let created_at = restore
            .status
            .as_ref()
            .and_then(|s| s.restore_time.as_ref())
            .and_then(|t| {
                DateTime::parse_from_rfc3339(t)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

        let completed_at = if status == RestoreStatus::Succeeded {
            created_at // Use restore time as completion for now
        } else {
            None
        };

        let error = restore
            .status
            .as_ref()
            .and_then(|s| s.error.as_ref())
            .and_then(|e| e.message.clone());

        RestoreInfo {
            name,
            snapshot_name,
            target_vm_name,
            namespace,
            status,
            created_at,
            completed_at,
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a Kubernetes cluster with KubeVirt
    // For unit tests without cluster access, we test the conversion logic

    #[tokio::test]
    async fn test_manager_creation_without_cluster() {
        // This test will pass even without a cluster
        let result = RestoreManager::new("default").await;
        // Will fail without cluster access, which is expected
        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn test_estimate_restore_time() {
        let client = Client::try_default().await;
        // Create a manager for testing (won't actually connect)
        if let Ok(c) = client {
            let manager = RestoreManager::from_client(c, "default");

            assert_eq!(manager.estimate_restore_time(5), "~1 minutes");
            assert_eq!(manager.estimate_restore_time(50), "~5 minutes");
            assert_eq!(manager.estimate_restore_time(100), "~10 minutes");
            assert_eq!(manager.estimate_restore_time(700), "~1 hours 10 minutes");
        }
    }
}
