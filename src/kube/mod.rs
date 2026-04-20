pub mod converter;
pub mod status;
pub mod types;

use crate::config::VMConfig;
use crate::utils::VMRogueError;
use anyhow::Result;
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use kube::{
    Client, Config,
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
};
use serde_json::json;
use std::sync::OnceLock;

pub use converter::vm_config_to_kubevirt;
pub use status::{ResourceSummary, VMStatus};
pub use types::*;

pub(crate) static KUBECONFIG_PATH: OnceLock<String> = OnceLock::new();
static CACHED_CONFIG: tokio::sync::OnceCell<Config> = tokio::sync::OnceCell::const_new();

/// Set the global kubeconfig path (called once at startup)
pub fn set_kubeconfig_path(path: String) {
    let _ = KUBECONFIG_PATH.set(path);
}

/// Resolve the kube `Config`, caching it for subsequent calls.
///
/// The config is pure data (no runtime handles), so it is safe to cache
/// across await points and even across tokio runtimes in tests.
async fn resolve_config() -> Result<Config> {
    let config = CACHED_CONFIG
        .get_or_try_init(|| async {
            let cfg = if let Some(path) = KUBECONFIG_PATH.get() {
                let kubeconfig =
                    kube::config::Kubeconfig::read_from(path).map_err(|e| anyhow::anyhow!(e))?;
                Config::from_custom_kubeconfig(
                    kubeconfig,
                    &kube::config::KubeConfigOptions::default(),
                )
                .await
                .map_err(|e| anyhow::anyhow!(e))?
            } else {
                Config::infer().await.map_err(|e| anyhow::anyhow!(e))?
            };
            Ok::<Config, anyhow::Error>(cfg)
        })
        .await?;
    Ok(config.clone())
}

/// Get a raw `kube::Client` that respects the global kubeconfig path.
///
/// The kubeconfig is parsed once and cached. A fresh `Client` is created
/// per call so it binds to the current tokio runtime (important for tests).
/// `Client::try_from` is cheap — it only allocates a connection pool.
pub async fn get_client() -> Result<Client> {
    let config = resolve_config().await.map_err(|e| {
        let hint = if KUBECONFIG_PATH.get().is_some() {
            "Check that the --kubeconfig path is correct and the file is readable."
        } else {
            "Set KUBECONFIG, pass --kubeconfig, or run inside a Kubernetes pod.\n  \
             Verify with: kubectl cluster-info"
        };
        anyhow::anyhow!("Failed to connect to Kubernetes: {}\n  Hint: {}", e, hint)
    })?;
    Ok(Client::try_from(config)?)
}

/// Kubernetes client for managing KubeVirt VMs
#[derive(Clone)]
pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    /// Create a new KubeClient using the global kubeconfig if set, or default discovery
    pub async fn new() -> Result<Self> {
        let client = get_client().await?;
        Ok(Self { client })
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

    /// Restart a VM (stop then start, polling for shutdown with timeout)
    pub async fn restart_vm(&self, namespace: &str, name: &str) -> Result<VirtualMachine> {
        self.stop_vm(namespace, name).await?;
        // Poll for VM to stop with a 30-second timeout
        let poll = async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                match self.is_running(namespace, name).await {
                    Ok(false) => return Ok(()),
                    Ok(true) => continue,
                    Err(e) => {
                        log::warn!("Error checking VM status during restart: {}", e);
                        return Err(e);
                    }
                }
            }
        };
        tokio::time::timeout(tokio::time::Duration::from_secs(30), poll)
            .await
            .map_err(|_| anyhow::anyhow!("VM '{}' did not stop within 30 seconds", name))??;
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

    /// Get both IP and node from a single VMI fetch (avoids double API call)
    pub async fn get_vm_ip_and_node(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<(Option<String>, Option<String>)> {
        match self.get_vmi(namespace, name).await {
            Ok(vmi) => {
                let ip = vmi.status.as_ref().and_then(|s| {
                    s.interfaces.iter().find_map(|iface| {
                        iface
                            .ip_address
                            .as_ref()
                            .filter(|ip| !ip.is_empty())
                            .cloned()
                    })
                });
                let node = vmi.status.and_then(|s| s.node_name);
                Ok((ip, node))
            }
            Err(e) => {
                if let Some(kube::Error::Api(ae)) = e.downcast_ref::<kube::Error>() {
                    if ae.code == 404 {
                        return Ok((None, None));
                    }
                }
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

    /// List all nodes in the cluster
    pub async fn list_nodes(&self) -> Result<Vec<k8s_openapi::api::core::v1::Node>> {
        let nodes: Api<k8s_openapi::api::core::v1::Node> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let node_list = nodes.list(&lp).await?;
        Ok(node_list.items)
    }

    /// List pods in a namespace
    pub async fn list_pods(&self, namespace: &str) -> Result<Vec<k8s_openapi::api::core::v1::Pod>> {
        let pods: Api<k8s_openapi::api::core::v1::Pod> =
            Api::namespaced(self.client.clone(), namespace);
        let lp = ListParams::default();
        let pod_list = pods.list(&lp).await?;
        Ok(pod_list.items)
    }

    /// List pods in all namespaces (for `namespace=all` dashboards).
    pub async fn list_all_pods(&self) -> Result<Vec<k8s_openapi::api::core::v1::Pod>> {
        let pods: Api<k8s_openapi::api::core::v1::Pod> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let pod_list = pods.list(&lp).await?;
        Ok(pod_list.items)
    }

    /// List events in a namespace
    pub async fn list_events(
        &self,
        namespace: &str,
    ) -> Result<Vec<k8s_openapi::api::core::v1::Event>> {
        let events: Api<k8s_openapi::api::core::v1::Event> =
            Api::namespaced(self.client.clone(), namespace);
        let lp = ListParams::default();
        let event_list = events.list(&lp).await?;
        Ok(event_list.items)
    }

    /// List events across all namespaces
    pub async fn list_all_events(&self) -> Result<Vec<k8s_openapi::api::core::v1::Event>> {
        let events: Api<k8s_openapi::api::core::v1::Event> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let event_list = events.list(&lp).await?;
        Ok(event_list.items)
    }

    /// List all namespaces
    pub async fn list_namespaces(&self) -> Result<Vec<k8s_openapi::api::core::v1::Namespace>> {
        let ns: Api<k8s_openapi::api::core::v1::Namespace> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let ns_list = ns.list(&lp).await?;
        Ok(ns_list.items)
    }

    /// List all VMIs in a namespace
    pub async fn list_vmis(&self, namespace: &str) -> Result<Vec<VirtualMachineInstance>> {
        let vmis: Api<VirtualMachineInstance> = Api::namespaced(self.client.clone(), namespace);
        let lp = ListParams::default();
        let vmi_list = vmis.list(&lp).await?;
        Ok(vmi_list.items)
    }

    /// List all VMIs cluster-wide (same VM name + namespace keys as namespaced list)
    pub async fn list_all_vmis(&self) -> Result<Vec<VirtualMachineInstance>> {
        let vmis: Api<VirtualMachineInstance> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let vmi_list = vmis.list(&lp).await?;
        Ok(vmi_list.items)
    }

    /// Clone a VM by copying its spec and creating a new VM with a different name
    pub async fn clone_vm(
        &self,
        namespace: &str,
        name: &str,
        new_name: &str,
    ) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);

        // Get existing VM
        let source = vms.get(name).await?;

        // Build new VM from source spec
        let mut new_vm = VirtualMachine {
            metadata: kube::api::ObjectMeta {
                name: Some(new_name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: source.metadata.labels.clone(),
                annotations: source.metadata.annotations.clone(),
                ..Default::default()
            },
            spec: source.spec.clone(),
            status: None,
        };
        // Don't auto-start the clone — use RunStrategy instead of deprecated Running field
        new_vm.spec.running = None;
        new_vm.spec.run_strategy = Some("Halted".to_string());

        let pp = PostParams::default();
        let created = vms.create(&pp, &new_vm).await?;
        Ok(created)
    }

    /// Pause a running VM instance via virtctl
    pub async fn pause_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args(["pause", "vmi", name, "-n", namespace])
            .output()
            .await?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to pause VM: {}", err));
        }
        Ok(())
    }

    /// Unpause a paused VM instance via virtctl
    pub async fn unpause_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args(["unpause", "vmi", name, "-n", namespace])
            .output()
            .await?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to unpause VM: {}", err));
        }
        Ok(())
    }

    /// Trigger live migration of a VM to another node
    pub async fn migrate_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let migrations: Api<VirtualMachineInstanceMigration> =
            Api::namespaced(self.client.clone(), namespace);

        let migration_name = format!(
            "{}-migration-{}",
            name,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        );
        let migration = VirtualMachineInstanceMigration {
            metadata: kube::api::ObjectMeta {
                name: Some(migration_name),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: VirtualMachineInstanceMigrationSpec {
                vmi_name: Some(name.to_string()),
            },
            status: None,
        };

        let pp = PostParams::default();
        migrations.create(&pp, &migration).await?;
        Ok(())
    }

    /// Update VM CPU and memory (requires VM to be stopped for changes to take effect)
    pub async fn update_vm_resources(
        &self,
        namespace: &str,
        name: &str,
        cpus: Option<u32>,
        memory: Option<&str>,
    ) -> Result<VirtualMachine> {
        let vms: Api<VirtualMachine> = self.vm_api(namespace);

        let mut patch = json!({});
        if let Some(cores) = cpus {
            patch["spec"]["template"]["spec"]["domain"]["cpu"]["cores"] = json!(cores);
        }
        if let Some(mem) = memory {
            patch["spec"]["template"]["spec"]["domain"]["resources"]["requests"]["memory"] =
                json!(mem);
        }

        let pp = PatchParams::default();
        let patched = vms.patch(name, &pp, &Patch::Merge(patch)).await?;
        Ok(patched)
    }

    /// List PVCs in a namespace
    pub async fn list_pvcs(&self, namespace: &str) -> Result<Vec<PersistentVolumeClaim>> {
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), namespace);
        let lp = ListParams::default();
        let list = pvcs.list(&lp).await?;
        Ok(list.items)
    }

    /// List storage classes
    pub async fn list_storage_classes(
        &self,
    ) -> Result<Vec<k8s_openapi::api::storage::v1::StorageClass>> {
        let scs: Api<k8s_openapi::api::storage::v1::StorageClass> = Api::all(self.client.clone());
        let lp = ListParams::default();
        let list = scs.list(&lp).await?;
        Ok(list.items)
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
