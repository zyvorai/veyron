// Migration Assistant - Guided multi-step migration wizard

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAssistant {
    pub current_step: MigrationStep,
    pub selected_vms: Vec<String>,
    pub target_node: Option<String>,
    pub pre_check_results: Vec<PreCheckResult>,
    pub post_check_results: Vec<PostCheckResult>,
    pub config: AssistantConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStep { SelectTarget, PreCheck, Confirm, Migrating, PostCheck, Complete }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCheckResult {
    pub check_name: String,
    pub check_type: CheckType,
    pub status: CheckStatus,
    pub severity: CheckSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType { Resource, Network, Storage, Compatibility, Health }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus { Passed, Warning, Failed, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckSeverity { Required, Recommended, Optional }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCheckResult {
    pub check_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantConfig {
    pub auto_pre_check: bool,
    pub auto_post_check: bool,
    pub require_all_pre_checks: bool,
    pub timeout_secs: u64,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self { auto_pre_check: true, auto_post_check: true, require_all_pre_checks: false, timeout_secs: 600 }
    }
}

impl MigrationAssistant {
    pub fn new() -> Self {
        Self { current_step: MigrationStep::SelectTarget, selected_vms: Vec::new(),
            target_node: None, pre_check_results: Vec::new(), post_check_results: Vec::new(),
            config: AssistantConfig::default() }
    }

    pub fn select_vms(&mut self, vms: Vec<String>) { self.selected_vms = vms; }
    pub fn select_target(&mut self, node: &str) { self.target_node = Some(node.to_string()); }

    /// Run pre-migration checks against the Kubernetes cluster.
    ///
    /// Checks target node exists and is schedulable, VM is running,
    /// and storage is accessible.
    pub async fn run_pre_checks(&mut self, namespace: &str) -> bool {
        self.pre_check_results.clear();

        let client = match crate::kube::KubeClient::new().await {
            Ok(c) => c,
            Err(e) => {
                self.pre_check_results.push(PreCheckResult {
                    check_name: "Cluster connectivity".to_string(),
                    check_type: CheckType::Network,
                    status: CheckStatus::Failed,
                    severity: CheckSeverity::Required,
                    message: format!("Cannot connect to cluster: {}", e),
                });
                self.current_step = MigrationStep::PreCheck;
                return false;
            }
        };

        // Check target node exists and is schedulable
        if let Some(ref target) = self.target_node {
            let k8s = client.client();
            let nodes_api: kube::api::Api<k8s_openapi::api::core::v1::Node> =
                kube::api::Api::all(k8s);
            match nodes_api.get(target).await {
                Ok(node) => {
                    let unschedulable = node.spec.as_ref()
                        .and_then(|s| s.unschedulable)
                        .unwrap_or(false);
                    if unschedulable {
                        self.pre_check_results.push(PreCheckResult {
                            check_name: "Target node schedulable".to_string(),
                            check_type: CheckType::Resource,
                            status: CheckStatus::Failed,
                            severity: CheckSeverity::Required,
                            message: format!("Target node '{}' is cordoned/unschedulable", target),
                        });
                    } else {
                        self.pre_check_results.push(PreCheckResult {
                            check_name: "Target node schedulable".to_string(),
                            check_type: CheckType::Resource,
                            status: CheckStatus::Passed,
                            severity: CheckSeverity::Required,
                            message: format!("Target node '{}' is schedulable", target),
                        });
                    }
                }
                Err(_) => {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: "Target node exists".to_string(),
                        check_type: CheckType::Resource,
                        status: CheckStatus::Failed,
                        severity: CheckSeverity::Required,
                        message: format!("Target node '{}' not found", target),
                    });
                }
            }
        }

        // Check each selected VM is running
        for vm_name in &self.selected_vms {
            match client.is_running(namespace, vm_name).await {
                Ok(true) => {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: format!("VM '{}' running", vm_name),
                        check_type: CheckType::Health,
                        status: CheckStatus::Passed,
                        severity: CheckSeverity::Required,
                        message: format!("VM '{}' is running and ready for live migration", vm_name),
                    });
                }
                Ok(false) => {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: format!("VM '{}' running", vm_name),
                        check_type: CheckType::Health,
                        status: CheckStatus::Failed,
                        severity: CheckSeverity::Required,
                        message: format!("VM '{}' is not running — cannot live-migrate a stopped VM", vm_name),
                    });
                }
                Err(e) => {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: format!("VM '{}' status", vm_name),
                        check_type: CheckType::Health,
                        status: CheckStatus::Warning,
                        severity: CheckSeverity::Required,
                        message: format!("Could not check VM '{}' status: {}", vm_name, e),
                    });
                }
            }
        }

        // Check storage accessibility (VM has shared storage, not local)
        for vm_name in &self.selected_vms {
            if let Ok(vm) = client.get_vm(namespace, vm_name).await {
                let has_local_only = vm.spec.template.spec.volumes.as_ref()
                    .map(|vols| vols.iter().all(|v| v.empty_disk.is_some() || v.container_disk.is_some()))
                    .unwrap_or(true);
                if has_local_only {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: format!("VM '{}' storage", vm_name),
                        check_type: CheckType::Storage,
                        status: CheckStatus::Warning,
                        severity: CheckSeverity::Recommended,
                        message: format!("VM '{}' uses only local/ephemeral storage — data may not persist after migration", vm_name),
                    });
                } else {
                    self.pre_check_results.push(PreCheckResult {
                        check_name: format!("VM '{}' storage", vm_name),
                        check_type: CheckType::Storage,
                        status: CheckStatus::Passed,
                        severity: CheckSeverity::Required,
                        message: format!("VM '{}' uses shared storage (PVC/DataVolume)", vm_name),
                    });
                }
            }
        }

        self.current_step = MigrationStep::PreCheck;
        self.pre_checks_passed()
    }

    pub fn pre_checks_passed(&self) -> bool {
        !self.pre_check_results.iter().any(|r| r.status == CheckStatus::Failed)
    }

    pub fn confirm(&mut self) { self.current_step = MigrationStep::Confirm; }
    pub fn start_migration(&mut self) { self.current_step = MigrationStep::Migrating; }

    /// Run post-migration checks — verify VMs are running and have IPs.
    pub async fn run_post_checks(&mut self, namespace: &str) {
        self.post_check_results.clear();

        let client = match crate::kube::KubeClient::new().await {
            Ok(c) => c,
            Err(e) => {
                self.post_check_results.push(PostCheckResult {
                    check_name: "Cluster connectivity".to_string(),
                    passed: false,
                    message: format!("Cannot connect to cluster: {}", e),
                });
                self.current_step = MigrationStep::PostCheck;
                return;
            }
        };

        for vm_name in &self.selected_vms {
            // Check VM is running
            let running = client.is_running(namespace, vm_name).await.unwrap_or(false);
            self.post_check_results.push(PostCheckResult {
                check_name: format!("VM '{}' running", vm_name),
                passed: running,
                message: if running {
                    format!("VM '{}' is running after migration", vm_name)
                } else {
                    format!("VM '{}' is NOT running after migration", vm_name)
                },
            });

            // Check VM has network access (IP assigned)
            let has_ip = client.get_vm_ip(namespace, vm_name).await
                .ok()
                .flatten()
                .is_some();
            self.post_check_results.push(PostCheckResult {
                check_name: format!("VM '{}' network", vm_name),
                passed: has_ip,
                message: if has_ip {
                    format!("VM '{}' has a network IP assigned", vm_name)
                } else {
                    format!("VM '{}' does not have an IP yet (may still be initializing)", vm_name)
                },
            });

            // Check VM is on the target node (if specified)
            if let Some(ref target) = self.target_node {
                let on_target = client.get_vm_node(namespace, vm_name).await
                    .ok()
                    .flatten()
                    .map(|n| n == *target)
                    .unwrap_or(false);
                self.post_check_results.push(PostCheckResult {
                    check_name: format!("VM '{}' on target node", vm_name),
                    passed: on_target,
                    message: if on_target {
                        format!("VM '{}' is running on target node '{}'", vm_name, target)
                    } else {
                        format!("VM '{}' is NOT on target node '{}'", vm_name, target)
                    },
                });
            }
        }

        self.current_step = MigrationStep::PostCheck;
    }

    pub fn complete(&mut self) { self.current_step = MigrationStep::Complete; }
    pub fn reset(&mut self) { *self = Self::new(); }
}

impl Default for MigrationAssistant {
    fn default() -> Self { Self::new() }
}
