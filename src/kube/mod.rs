pub mod converter;
pub mod status;
pub mod types;

use crate::config::VMConfig;
use crate::utils::VMRogueError;
use anyhow::Result;
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
    Client, Config,
};
use serde_json::json;
use std::sync::OnceLock;

pub use converter::vm_config_to_kubevirt;
pub use status::{ResourceSummary, VMStatus};
pub use types::*;

static KUBECONFIG_PATH: OnceLock<String> = OnceLock::new();

/// Set the global kubeconfig path (called once at startup)
pub fn set_kubeconfig_path(path: String) {
    let _ = KUBECONFIG_PATH.set(path);
}

/// Kubernetes client for managing KubeVirt VMs
#[derive(Clone)]
pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    /// Create a new KubeClient using the global kubeconfig if set, or default discovery
    pub async fn new() -> Result<Self> {
        if let Some(path) = KUBECONFIG_PATH.get() {
            Self::with_kubeconfig(path).await
        } else {
            let config = Config::infer().await?;
            let client = Client::try_from(config)?;
            Ok(Self { client })
        }
    }

    /// Create a new KubeClient with custom kubeconfig path
    pub async fn with_kubeconfig(path: &str) -> Result<Self> {
        let kubeconfig = kube::config::Kubeconfig::read_from(path)?;
        let config =
            Config::from_custom_kubeconfig(kubeconfig, &kube::config::KubeConfigOptions::default())
                .await?;
        let client = Client::try_from(config)?;
        Ok(Self { client })
    }

    /// Get a clone of the underlying kube::Client
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Get API handle for VirtualMachines in a namespace
    fn vm_api(&self, namespace: &str) -> Api<VirtualMachine> {
        Api::namespaced(self.client.clone(), namespace)
    }

    /// Get API handle for PVCs in a namespace
    fn pvc_api(&self, namespace: &str) -> Api<PersistentVolumeClaim> {
        Api::namespaced(self.client.clone(), namespace)
    }

    /// List all VMs in a namespace
    pub async fn list_vms(&self, namespace: &str) -> Result<Vec<VirtualMachine>> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);
        let lp = ListParams::default();
        let vm_list = vms.list(&lp).await?;
        Ok(vm_list.items)
    }

    /// List all VMs across all namespaces
    pub async fn list_all_vms(&self) -> Result<Vec<VirtualMachine>> {
        let vms: Api<VirtualMachine> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let vm_list = vms.list(&lp).await?;
        Ok(vm_list.items)
    }

    /// Get a specific VM
    pub async fn get_vm(&self, namespace: &str, name: &str) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);
        let vm = vms.get(name).await?;
        Ok(vm)
    }

    /// Create a VM from VMConfig
    pub async fn create_vm(&self, config: &VMConfig) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(&config.namespace);

        // Check if VM already exists
        match vms.get(&config.name).await {
            Ok(_) => return Err(VMRogueError::VmExists(config.name.clone()).into()),
            Err(kube::Error::Api(ae)) if ae.code == 404 => {} // VM doesn't exist, proceed
            Err(e) => return Err(e.into()), // Propagate other errors (network, auth, etc.)
        }

        // Convert VMConfig to KubeVirt VirtualMachine
        let vm = vm_config_to_kubevirt(config)?;

        // Create the VM
        let pp = PostParams::default();
        let created = vms.create(&pp, &vm).await?;

        Ok(created)
    }

    /// Delete a VM
    pub async fn delete_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);

        // Check if VM exists
        match vms.get(name).await {
            Ok(_) => {}
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                return Err(VMRogueError::VmNotFound(name.to_string()).into());
            }
            Err(e) => return Err(e.into()),
        }

        let dp = DeleteParams::default();
        vms.delete(name, &dp).await?;

        Ok(())
    }

    /// Start a VM (set running: true)
    pub async fn start_vm(&self, namespace: &str, name: &str) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);
        let patch = json!({
            "spec": {
                "running": true
            }
        });
        let pp = PatchParams::default();
        match vms.patch(name, &pp, &Patch::Merge(&patch)).await {
            Ok(patched) => Ok(patched),
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                Err(VMRogueError::VmNotFound(name.to_string()).into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Stop a VM (set running: false)
    pub async fn stop_vm(&self, namespace: &str, name: &str) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);
        let patch = json!({
            "spec": {
                "running": false
            }
        });
        let pp = PatchParams::default();
        match vms.patch(name, &pp, &Patch::Merge(&patch)).await {
            Ok(patched) => Ok(patched),
            Err(kube::Error::Api(ae)) if ae.code == 404 => {
                Err(VMRogueError::VmNotFound(name.to_string()).into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Restart a VM (stop then start, polling for shutdown)
    pub async fn restart_vm(&self, namespace: &str, name: &str) -> Result<VirtualMachine> {
        self.stop_vm(namespace, name).await?;
        // Poll for VM to stop (max 30 seconds)
        let mut stopped = false;
        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            match self.is_running(namespace, name).await {
                Ok(false) => {
                    stopped = true;
                    break;
                }
                Ok(true) => continue,
                Err(e) => {
                    log::warn!("Error checking VM status during restart: {}", e);
                    return Err(e);
                }
            }
        }
        if !stopped {
            anyhow::bail!("VM '{}' did not stop within 30 seconds", name);
        }
        self.start_vm(namespace, name).await
    }

    /// Check if a VM is running
    pub async fn is_running(&self, namespace: &str, name: &str) -> Result<bool> {
        let vm = self.get_vm(namespace, name).await?;
        Ok(vm.status.as_ref().and_then(|s| s.ready).unwrap_or(false))
    }

    /// Get VM status string
    pub async fn get_status(&self, namespace: &str, name: &str) -> Result<String> {
        let vm = self.get_vm(namespace, name).await?;

        if let Some(status) = &vm.status {
            if let Some(printable) = &status.printable_status {
                return Ok(printable.clone());
            }
            if status.ready.unwrap_or(false) {
                return Ok("Running".to_string());
            }
            if status.created.unwrap_or(false) {
                return Ok("Created".to_string());
            }
        }

        if vm.spec.running.unwrap_or(false) {
            Ok("Starting".to_string())
        } else {
            Ok("Stopped".to_string())
        }
    }

    /// Get the VirtualMachineInstance (running instance) for a VM
    pub async fn get_vmi(&self, namespace: &str, name: &str) -> Result<VirtualMachineInstance> {
        let vmis: Api<VirtualMachineInstance> = Api::namespaced(self.client.clone(), namespace);
        let vmi = vmis.get(name).await?;
        Ok(vmi)
    }

    /// Get the IP address of a running VM via its VMI
    pub async fn get_vm_ip(&self, namespace: &str, name: &str) -> Result<Option<String>> {
        match self.get_vmi(namespace, name).await {
            Ok(vmi) => {
                if let Some(status) = &vmi.status {
                    // Return the first non-empty IP address found
                    for iface in &status.interfaces {
                        if let Some(ip) = &iface.ip_address {
                            if !ip.is_empty() {
                                return Ok(Some(ip.clone()));
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => {
                // VMI not found means VM is not running — no IP available
                if let Some(kube::Error::Api(ae)) = e.downcast_ref::<kube::Error>() {
                    if ae.code == 404 {
                        return Ok(None);
                    }
                }
                // Propagate real errors (auth, network, etc.)
                Err(e)
            }
        }
    }

    /// Get the node a VM is running on via its VMI
    pub async fn get_vm_node(&self, namespace: &str, name: &str) -> Result<Option<String>> {
        match self.get_vmi(namespace, name).await {
            Ok(vmi) => Ok(vmi.status.and_then(|s| s.node_name)),
            Err(e) => {
                // VMI not found means VM is not running — no node available
                if let Some(kube::Error::Api(ae)) = e.downcast_ref::<kube::Error>() {
                    if ae.code == 404 {
                        return Ok(None);
                    }
                }
                // Propagate real errors
                Err(e)
            }
        }
    }

    /// Create a PVC for a VM disk
    pub async fn create_pvc(
        &self,
        namespace: &str,
        name: &str,
        size: &str,
        storage_class: Option<&str>,
    ) -> Result<PersistentVolumeClaim> {
        use k8s_openapi::api::core::v1::PersistentVolumeClaimSpec;
        use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
        use std::collections::BTreeMap;

        let pvcs: Api<PersistentVolumeClaim> = self.pvc_api(namespace);

        let mut requests = BTreeMap::new();
        requests.insert("storage".to_string(), Quantity(size.to_string()));

        let pvc = PersistentVolumeClaim {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                resources: Some(k8s_openapi::api::core::v1::VolumeResourceRequirements {
                    requests: Some(requests),
                    ..Default::default()
                }),
                storage_class_name: storage_class.map(|s| s.to_string()),
                ..Default::default()
            }),
            status: None,
        };

        let pp = PostParams::default();
        let created = pvcs.create(&pp, &pvc).await?;

        Ok(created)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_conversion() {
        use crate::config::VMConfigBuilder;

        let config = VMConfigBuilder::new("test")
            .namespace("default")
            .cpu(2, 1, 1)
            .memory("4Gi")
            .add_blank_disk("rootdisk", "20Gi", 1)
            .add_pod_network("default")
            .build();

        let vm = vm_config_to_kubevirt(&config).unwrap();
        assert_eq!(vm.metadata.name, Some("test".to_string()));
    }
}
