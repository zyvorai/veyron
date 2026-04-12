// Disk Expansion - PVC resizing and VM disk expansion

use super::DiskConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use k8s_openapi::api::storage::v1::StorageClass;
use kube::{
    api::{Api, Patch, PatchParams},
    Client,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Validate that a PVC name conforms to Kubernetes naming rules
fn validate_pvc_name(name: &str) -> Result<()> {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[a-z0-9]([a-z0-9\-]*[a-z0-9])?$").unwrap());
    if name.is_empty() || name.len() > 253 || !RE.is_match(name) {
        anyhow::bail!("Invalid PVC name: {}", name);
    }
    Ok(())
}

/// Validate that a size string matches Kubernetes quantity format (e.g., "10Gi", "500Mi")
fn validate_k8s_quantity(size: &str) -> Result<()> {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[0-9]+(\.[0-9]+)?(Ki|Mi|Gi|Ti|Pi|Ei|k|M|G|T|P|E)?$").unwrap());
    if size.is_empty() || !RE.is_match(size) {
        anyhow::bail!("Invalid Kubernetes quantity format: {}", size);
    }
    Ok(())
}

/// Expansion status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExpansionStatus {
    Pending,
    PVCResizing,
    VMRescanNeeded,
    FilesystemResizeNeeded,
    Completed,
    Failed,
}

impl std::fmt::Display for ExpansionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpansionStatus::Pending => write!(f, "Pending"),
            ExpansionStatus::PVCResizing => write!(f, "PVC Resizing"),
            ExpansionStatus::VMRescanNeeded => write!(f, "VM Rescan Needed"),
            ExpansionStatus::FilesystemResizeNeeded => write!(f, "Filesystem Resize Needed"),
            ExpansionStatus::Completed => write!(f, "Completed"),
            ExpansionStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Expansion plan for a disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionPlan {
    pub vm_name: String,
    pub disk_name: String,
    pub current_size: String,
    pub target_size: String,
    pub pvc_name: String,
    pub steps: Vec<ExpansionStep>,
    pub status: ExpansionStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ExpansionPlan {
    pub fn new(vm_name: impl Into<String>, config: &DiskConfig) -> Result<Self> {
        validate_pvc_name(&config.pvc_name)?;
        if !config.target_size.is_empty() {
            validate_k8s_quantity(&config.target_size)?;
        }

        let steps = vec![
            ExpansionStep {
                step_number: 1,
                description: "Resize PVC in Kubernetes".to_string(),
                command: format!("kubectl patch pvc {} -p '{{\"spec\":{{\"resources\":{{\"requests\":{{\"storage\":\"{}\"}}}}}}}}'",
                    config.pvc_name, config.target_size),
                completed: false,
            },
            ExpansionStep {
                step_number: 2,
                description: "Rescan disk in VM".to_string(),
                command: format!(
                    "echo 1 | sudo tee /sys/class/block/{}/device/rescan",
                    config.name.trim_start_matches("/dev/")
                ),
                completed: false,
            },
            ExpansionStep {
                step_number: 3,
                description: "Extend LVM physical volume (if using LVM)".to_string(),
                command: "sudo pvresize /dev/<partition>  # Adjust partition for your VM layout".to_string(),
                completed: false,
            },
            ExpansionStep {
                step_number: 4,
                description: "Extend LVM logical volume".to_string(),
                command: "sudo lvextend -l +100%FREE /dev/<vg>/<lv>  # Adjust VG/LV names for your VM".to_string(),
                completed: false,
            },
            ExpansionStep {
                step_number: 5,
                description: "Resize filesystem".to_string(),
                command: "sudo resize2fs /dev/<vg>/<lv>  # Or xfs_growfs / for XFS".to_string(),
                completed: false,
            },
        ];

        Ok(Self {
            vm_name: vm_name.into(),
            disk_name: config.name.clone(),
            current_size: config.current_size.clone(),
            target_size: config.target_size.clone(),
            pvc_name: config.pvc_name.clone(),
            steps,
            status: ExpansionStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        })
    }

    /// Mark a step as completed
    pub fn complete_step(&mut self, step_number: usize) {
        if let Some(step) = self.steps.get_mut(step_number - 1) {
            step.completed = true;
        }
        self.update_status();
    }

    /// Update overall status based on completed steps
    fn update_status(&mut self) {
        let completed_count = self.steps.iter().filter(|s| s.completed).count();

        if completed_count == 0 {
            self.status = ExpansionStatus::Pending;
        } else if completed_count == 1 {
            self.status = ExpansionStatus::PVCResizing;
        } else if completed_count == 2 {
            self.status = ExpansionStatus::VMRescanNeeded;
        } else if completed_count < self.steps.len() {
            self.status = ExpansionStatus::FilesystemResizeNeeded;
        } else {
            self.status = ExpansionStatus::Completed;
            self.completed_at = Some(Utc::now());
        }
    }

    /// Get next pending step
    pub fn next_step(&self) -> Option<&ExpansionStep> {
        self.steps.iter().find(|s| !s.completed)
    }

    /// Calculate progress percentage
    pub fn progress(&self) -> u8 {
        if self.steps.is_empty() {
            return 100; // No steps means nothing to do
        }
        let completed = self.steps.iter().filter(|s| s.completed).count();
        ((completed as f64 / self.steps.len() as f64) * 100.0) as u8
    }
}

/// Individual expansion step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionStep {
    pub step_number: usize,
    pub description: String,
    pub command: String,
    pub completed: bool,
}

/// Disk expansion manager
pub struct DiskExpansion {
    namespace: String,
    client: Option<Client>,
}

impl DiskExpansion {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            client: None,
        }
    }

    /// Create with a Kubernetes client for real operations
    pub async fn with_kube(namespace: impl Into<String>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .context("Failed to create Kubernetes client")?;
        Ok(Self {
            namespace: namespace.into(),
            client: Some(client),
        })
    }

    /// Get or create a Kubernetes client
    async fn get_client(&self) -> Result<Client> {
        if let Some(ref client) = self.client {
            Ok(client.clone())
        } else {
            Client::try_default()
                .await
                .context("Failed to create Kubernetes client for PVC operations")
        }
    }

    /// Create expansion plan
    pub fn create_plan(&self, vm_name: &str, config: &DiskConfig) -> Result<ExpansionPlan> {
        ExpansionPlan::new(vm_name, config)
    }

    /// Resize PVC in Kubernetes
    pub async fn resize_pvc(&self, pvc_name: &str, new_size: &str) -> Result<()> {
        let client = self.get_client().await?;
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client, &self.namespace);

        let patch = json!({
            "spec": {
                "resources": {
                    "requests": {
                        "storage": new_size
                    }
                }
            }
        });

        pvcs.patch(pvc_name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
            .with_context(|| format!("Failed to resize PVC '{}' to {}", pvc_name, new_size))?;

        Ok(())
    }

    /// Check if PVC supports expansion
    pub async fn can_expand_pvc(&self, pvc_name: &str) -> Result<bool> {
        let client = self.get_client().await?;
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), &self.namespace);

        let pvc = pvcs
            .get(pvc_name)
            .await
            .with_context(|| format!("Failed to get PVC '{}'", pvc_name))?;

        // Get the storage class name from the PVC
        let sc_name = match pvc
            .spec
            .as_ref()
            .and_then(|s| s.storage_class_name.as_ref())
        {
            Some(name) => name.clone(),
            None => return Ok(false), // No storage class, cannot determine expandability
        };

        // Check if the StorageClass allows volume expansion
        let storage_classes: Api<StorageClass> = Api::all(client);
        let sc = storage_classes
            .get(&sc_name)
            .await
            .with_context(|| format!("Failed to get StorageClass '{}'", sc_name))?;

        Ok(sc.allow_volume_expansion.unwrap_or(false))
    }

    /// Estimate expansion time
    pub fn estimate_duration(&self, size_increase_gi: u64) -> String {
        // Simple estimation: ~1 minute per 10Gi for PVC resize
        // Plus ~2-5 minutes for filesystem operations
        let pvc_minutes = (size_increase_gi as f64 / 10.0).ceil() as u64;
        let total_minutes = pvc_minutes + 5;

        if total_minutes < 60 {
            format!("~{} minutes", total_minutes)
        } else {
            format!(
                "~{} hours {} minutes",
                total_minutes / 60,
                total_minutes % 60
            )
        }
    }

    /// Verify expansion completed for a specific PVC
    pub async fn verify_expansion(&self, pvc_name: &str, expected_size: &str) -> Result<bool> {
        let client = self.get_client().await?;
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client, &self.namespace);

        let pvc = pvcs
            .get(pvc_name)
            .await
            .with_context(|| format!("Failed to get PVC '{}' for verification", pvc_name))?;

        let expected_bytes = super::DiskInfo::parse_size(expected_size);

        let current_size = pvc
            .spec
            .as_ref()
            .and_then(|s| s.resources.as_ref())
            .and_then(|r| r.requests.as_ref())
            .and_then(|req| req.get("storage"))
            .map(|q| q.0.as_str())
            .unwrap_or("0");

        let current_bytes = super::DiskInfo::parse_size(current_size);
        Ok(current_bytes >= expected_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expansion_plan_creation() {
        let config = DiskConfig::new("disk1", "my-pvc").with_sizes("20Gi", "40Gi");

        let plan = ExpansionPlan::new("test-vm", &config).unwrap();

        assert_eq!(plan.vm_name, "test-vm");
        assert_eq!(plan.current_size, "20Gi");
        assert_eq!(plan.target_size, "40Gi");
        assert_eq!(plan.status, ExpansionStatus::Pending);
        assert_eq!(plan.steps.len(), 5);
    }

    #[test]
    fn test_expansion_rejects_invalid_pvc_name() {
        let config = DiskConfig::new("disk1", "INVALID_PVC!").with_sizes("20Gi", "40Gi");
        assert!(ExpansionPlan::new("test-vm", &config).is_err());
    }

    #[test]
    fn test_expansion_rejects_invalid_target_size() {
        let config = DiskConfig::new("disk1", "my-pvc").with_sizes("20Gi", "not-a-size");
        assert!(ExpansionPlan::new("test-vm", &config).is_err());
    }

    #[test]
    fn test_expansion_progress() {
        let config = DiskConfig::new("disk1", "my-pvc").with_sizes("20Gi", "40Gi");

        let mut plan = ExpansionPlan::new("test-vm", &config).unwrap();

        assert_eq!(plan.progress(), 0);

        plan.complete_step(1);
        assert_eq!(plan.progress(), 20);

        plan.complete_step(2);
        plan.complete_step(3);
        assert_eq!(plan.progress(), 60);

        plan.complete_step(4);
        plan.complete_step(5);
        assert_eq!(plan.progress(), 100);
        assert_eq!(plan.status, ExpansionStatus::Completed);
    }

    #[test]
    fn test_next_step() {
        let config = DiskConfig::new("disk1", "my-pvc");
        let mut plan = ExpansionPlan::new("test-vm", &config).unwrap();

        let next = plan.next_step().unwrap();
        assert_eq!(next.step_number, 1);

        plan.complete_step(1);
        let next = plan.next_step().unwrap();
        assert_eq!(next.step_number, 2);
    }

    #[test]
    fn test_estimate_duration() {
        let expansion = DiskExpansion::new("default");

        assert!(expansion.estimate_duration(10).contains("minutes"));
        assert!(expansion.estimate_duration(100).contains("minutes"));
    }

    #[tokio::test]
    async fn test_resize_pvc_without_cluster() {
        // Without a K8s cluster, resize_pvc should fail with a client error
        let expansion = DiskExpansion::new("default");
        let result = expansion.resize_pvc("test-pvc", "50Gi").await;
        // Expect error when no cluster is available
        assert!(result.is_err());
    }
}
