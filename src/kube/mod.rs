pub mod converter;
pub mod status;
pub mod types;

use crate::config::{CloudInitDelivery, VMConfig};
use crate::utils::VMRogueError;
use anyhow::Result;
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use k8s_openapi::api::core::v1::{Secret, Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{
    Client, Config,
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
};
use serde_json::json;
use std::collections::BTreeMap;
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

    /// List VMs in one namespace, or cluster-wide when `scope` is `"all"`.
    /// Falls back to per-namespace listing if cluster-scoped `list` is not permitted.
    pub async fn list_vms_for_scope(&self, scope: &str) -> Vec<VirtualMachine> {
        if scope == "all" {
            match self.list_all_vms().await {
                Ok(v) => v,
                Err(_) => {
                    let mut out = Vec::new();
                    if let Ok(namespaces) = self.list_namespaces().await {
                        for ns in namespaces.into_iter().filter_map(|n| n.metadata.name) {
                            if let Ok(mut vms) = self.list_vms(&ns).await {
                                out.append(&mut vms);
                            }
                        }
                    }
                    out
                }
            }
        } else {
            self.list_vms(scope).await.unwrap_or_default()
        }
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

        if let Some(ci) = &config.cloud_init {
            if ci.delivery == CloudInitDelivery::ConfigDrive {
                self.ensure_configdrive_userdata_secret(
                    &config.namespace,
                    &cloudinit_configdrive_secret_name(&config.name),
                    &ci.user_data,
                )
                .await?;
            }
        }

        // Convert VMConfig to KubeVirt VirtualMachine
        let vm = vm_config_to_kubevirt(config)?;

        // Create the VM
        let pp = PostParams::default();
        let created = match vms.create(&pp, &vm).await {
            Ok(c) => c,
            Err(e) => {
                let _ = self
                    .delete_secret_ignore(
                        &config.namespace,
                        &cloudinit_configdrive_secret_name(&config.name),
                    )
                    .await;
                return Err(e.into());
            }
        };

        if let Some(exp) = &config.expose {
            if exp.enabled {
                if let Err(err) = self.ensure_vm_expose_service(config, exp).await {
                    log::warn!(
                        "VM '{}' created but expose Service failed: {}",
                        config.name,
                        err
                    );
                }
            }
        }

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

        self.delete_secret_ignore(namespace, &cloudinit_configdrive_secret_name(name))
            .await;
        self.delete_service_ignore(namespace, &vm_expose_service_name(name))
            .await;

        Ok(())
    }

    async fn ensure_configdrive_userdata_secret(
        &self,
        namespace: &str,
        name: &str,
        userdata: &str,
    ) -> Result<()> {
        let secrets: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        let mut string_data = BTreeMap::new();
        string_data.insert("userdata".to_string(), userdata.to_string());
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(BTreeMap::from([
                    ("vmrogue.io/managed-by".to_string(), "vmrogue".to_string()),
                    (
                        "vmrogue.io/configdrive-userdata".to_string(),
                        "true".to_string(),
                    ),
                ])),
                ..Default::default()
            },
            string_data: Some(string_data),
            type_: Some("Opaque".to_string()),
            ..Default::default()
        };

        match secrets.create(&PostParams::default(), &secret).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                secrets
                    .replace(name, &PostParams::default(), &secret)
                    .await?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Creates or updates a `Service` whose selector targets the **virt-launcher pod**
    /// (`kubevirt.io/domain=<vm>`). That forwards to the VM’s network namespace when the guest is
    /// running; it does **not** guarantee the guest OS is listening on the exposed ports.
    async fn ensure_vm_expose_service(
        &self,
        config: &VMConfig,
        expose: &crate::config::VmExposeConfig,
    ) -> Result<()> {
        let svc_api: Api<Service> = Api::namespaced(self.client.clone(), &config.namespace);
        let svc_name = vm_expose_service_name(&config.name);

        let mut selector = BTreeMap::new();
        selector.insert("kubevirt.io/domain".to_string(), config.name.clone());

        let type_str = match expose.service_type.to_ascii_lowercase().as_str() {
            "nodeport" => "NodePort",
            "loadbalancer" => "LoadBalancer",
            _ => "ClusterIP",
        };

        let ports_src = if expose.ports.is_empty() {
            vec![crate::config::VmExposePort {
                name: Some("ssh".to_string()),
                port: 22,
                target_port: 22,
                protocol: "TCP".to_string(),
            }]
        } else {
            expose.ports.clone()
        };

        let ports: Vec<ServicePort> = ports_src
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let mut port = ServicePort::default();
                port.name = Some(p.name.clone().unwrap_or_else(|| format!("port{}", i)));
                port.port = p.port;
                port.protocol = Some(p.protocol.clone());
                port.target_port = Some(IntOrString::Int(p.target_port));
                port
            })
            .collect();

        let svc = Service {
            metadata: ObjectMeta {
                name: Some(svc_name.clone()),
                namespace: Some(config.namespace.clone()),
                labels: Some(BTreeMap::from([
                    ("vmrogue.io/managed-by".to_string(), "vmrogue".to_string()),
                    ("vmrogue.io/expose-for-vm".to_string(), config.name.clone()),
                ])),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(selector),
                ports: Some(ports),
                type_: Some(type_str.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        match svc_api.create(&PostParams::default(), &svc).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                svc_api
                    .replace(&svc_name, &PostParams::default(), &svc)
                    .await?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn delete_secret_ignore(&self, namespace: &str, name: &str) {
        let secrets: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        match secrets.get(name).await {
            Ok(s) => {
                let managed = s
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("vmrogue.io/managed-by"))
                    .map(|v| v.as_str())
                    == Some("vmrogue");
                if !managed {
                    log::debug!(
                        "skip delete Secret {}: not labeled vmrogue.io/managed-by=vmrogue",
                        name
                    );
                    return;
                }
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => return,
            Err(e) => {
                log::debug!("Secret get {} before delete: {}", name, e);
                return;
            }
        }
        match secrets.delete(name, &DeleteParams::default()).await {
            Ok(_) => {}
            Err(kube::Error::Api(ae)) if ae.code == 404 => {}
            Err(e) => log::debug!("Secret delete {}: {}", name, e),
        }
    }

    async fn delete_service_ignore(&self, namespace: &str, name: &str) {
        let svcs: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        match svcs.get(name).await {
            Ok(s) => {
                let managed = s
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("vmrogue.io/managed-by"))
                    .map(|v| v.as_str())
                    == Some("vmrogue");
                if !managed {
                    log::debug!(
                        "skip delete Service {}: not labeled vmrogue.io/managed-by=vmrogue",
                        name
                    );
                    return;
                }
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => return,
            Err(e) => {
                log::debug!("Service get {} before delete: {}", name, e);
                return;
            }
        }
        match svcs.delete(name, &DeleteParams::default()).await {
            Ok(_) => {}
            Err(kube::Error::Api(ae)) if ae.code == 404 => {}
            Err(e) => log::debug!("Service delete {}: {}", name, e),
        }
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

    /// List pods in one namespace or cluster-wide (`scope == "all"`), with RBAC-safe fan-out.
    pub async fn list_pods_for_scope(&self, scope: &str) -> Vec<k8s_openapi::api::core::v1::Pod> {
        if scope == "all" {
            match self.list_all_pods().await {
                Ok(pods) => pods,
                Err(_) => {
                    let mut out = Vec::new();
                    if let Ok(namespaces) = self.list_namespaces().await {
                        for ns in namespaces.into_iter().filter_map(|n| n.metadata.name) {
                            if let Ok(mut pods) = self.list_pods(&ns).await {
                                out.append(&mut pods);
                            }
                        }
                    }
                    out
                }
            }
        } else {
            self.list_pods(scope).await.unwrap_or_default()
        }
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

    /// Events in one namespace or cluster-wide (`scope == "all"`), with fan-out fallback.
    pub async fn list_events_for_scope(
        &self,
        scope: &str,
    ) -> Vec<k8s_openapi::api::core::v1::Event> {
        if scope == "all" {
            match self.list_all_events().await {
                Ok(events) => events,
                Err(_) => {
                    let mut out = Vec::new();
                    if let Ok(namespaces) = self.list_namespaces().await {
                        for ns in namespaces.into_iter().filter_map(|n| n.metadata.name) {
                            if let Ok(mut ev) = self.list_events(&ns).await {
                                out.append(&mut ev);
                            }
                        }
                    }
                    out
                }
            }
        } else {
            self.list_events(scope).await.unwrap_or_default()
        }
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

    /// VMIs in one namespace or cluster-wide (`scope == "all"`), with fan-out fallback.
    pub async fn list_vmis_for_scope(&self, scope: &str) -> Vec<VirtualMachineInstance> {
        if scope == "all" {
            match self.list_all_vmis().await {
                Ok(vmis) => vmis,
                Err(_) => {
                    let mut out = Vec::new();
                    if let Ok(namespaces) = self.list_namespaces().await {
                        for ns in namespaces.into_iter().filter_map(|n| n.metadata.name) {
                            if let Ok(mut vmis) = self.list_vmis(&ns).await {
                                out.append(&mut vmis);
                            }
                        }
                    }
                    out
                }
            }
        } else {
            self.list_vmis(scope).await.unwrap_or_default()
        }
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

    /// List PVCs cluster-wide (`Api::all`), with per-namespace fan-out if cluster-scoped list is forbidden.
    pub async fn list_all_pvcs(&self) -> Result<Vec<PersistentVolumeClaim>> {
        let lp = ListParams::default();
        let pvcs: Api<PersistentVolumeClaim> = Api::all(self.client.clone());
        match pvcs.list(&lp).await {
            Ok(list) => Ok(list.items),
            Err(_) => {
                use k8s_openapi::api::core::v1::Namespace;
                let ns_api: Api<Namespace> = Api::all(self.client.clone());
                let mut out = Vec::new();
                if let Ok(ns_list) = ns_api.list(&lp).await {
                    for ns in ns_list.items.into_iter().filter_map(|n| n.metadata.name) {
                        let napi: Api<PersistentVolumeClaim> =
                            Api::namespaced(self.client.clone(), &ns);
                        if let Ok(pl) = napi.list(&lp).await {
                            out.extend(pl.items);
                        }
                    }
                }
                Ok(out)
            }
        }
    }

    /// PVCs in one namespace or cluster-wide (`scope == "all"`).
    pub async fn list_pvcs_for_scope(&self, scope: &str) -> Vec<PersistentVolumeClaim> {
        if scope == "all" {
            self.list_all_pvcs().await.unwrap_or_default()
        } else {
            self.list_pvcs(scope).await.unwrap_or_default()
        }
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

/// Kubernetes name for the Secret referenced by `cloudInitConfigDrive.userDataSecretRef`.
/// Keep consistent with `operator/internal/controller` helpers.
pub fn cloudinit_configdrive_secret_name(vm_name: &str) -> String {
    truncate_k8s_dns_subdomain(format!("{vm_name}-vmrogue-cfgdrv"), 253)
}

/// Kubernetes Service name for VMRogue-created exposed Services.
pub fn vm_expose_service_name(vm_name: &str) -> String {
    truncate_k8s_dns_subdomain(format!("{vm_name}-vmrogue-xp"), 63)
}

fn truncate_k8s_dns_subdomain(mut s: String, max: usize) -> String {
    while s.len() > max {
        s.pop();
    }
    s
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
