// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

pub mod guest_runtime;
pub mod guestkit_client;
pub mod converter;
pub mod guest_filesystem;
pub mod guest_os;
#[cfg(feature = "web")]
pub mod guest_prometheus;
#[cfg(not(feature = "web"))]
pub mod guest_prometheus {
    use super::guest_filesystem::GuestFilesystemMount;
    use super::types::VirtualMachine;
    use kube::Client;

    pub async fn prometheus_storage_mounts_for_vm(
        _client: &Client,
        _prom_base: &str,
        _namespace: &str,
        _vm_name: &str,
        _vm: &VirtualMachine,
    ) -> (Vec<GuestFilesystemMount>, Vec<String>) {
        (Vec::new(), Vec::new())
    }
}
pub mod kubevirt_subresources;
pub mod status;
pub mod types;
pub mod vm_data_disk;
pub mod vm_internet;
pub mod vm_multus;
pub mod vm_rdp;
pub mod vm_ssh;
pub mod windows_rdp;

use crate::config::{CloudInitDelivery, VMConfig, VmExposeConfig};
use crate::utils::VMRogueError;
use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use k8s_openapi::api::core::v1::{Secret, Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{
    Client, Config,
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub use converter::vm_config_to_kubevirt;
pub use status::{ResourceSummary, VMStatus};
pub use types::*;

/// Dashboard: KubeVirt + CDI readiness from the cluster API.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformReadiness {
    pub kubevirt_api_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubevirt_api_message: Option<String>,
    pub kubevirt_control_plane_ok: bool,
    pub kubevirt_detail: String,
    pub cdi_operator_ok: bool,
    pub cdi_detail: String,
    pub forge_vm_ready: bool,
    pub virtctl_note: &'static str,
}

pub(crate) static KUBECONFIG_PATH: OnceLock<String> = OnceLock::new();
static CACHED_CONFIG: tokio::sync::OnceCell<Config> = tokio::sync::OnceCell::const_new();
static ACTIVE_KUBECONFIG_CONTEXT: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Override kube client context for the API process (multi-cluster kubeconfig).
pub fn set_active_kube_context(ctx: Option<String>) {
    if let Ok(mut guard) = ACTIVE_KUBECONFIG_CONTEXT.write() {
        *guard = ctx;
    }
}

/// Active kubeconfig context override, if any.
pub fn active_kube_context() -> Option<String> {
    ACTIVE_KUBECONFIG_CONTEXT
        .read()
        .ok()
        .and_then(|g| g.clone())
}

fn kubeconfig_path_string() -> Option<String> {
    if let Some(path) = KUBECONFIG_PATH.get() {
        return Some(path.clone());
    }
    std::env::var("KUBECONFIG")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| dirs::home_dir().map(|h| h.join(".kube/config").to_string_lossy().into_owned()))
}

/// Set the global kubeconfig path (called once at startup)
pub fn set_kubeconfig_path(path: String) {
    let _ = KUBECONFIG_PATH.set(path);
}

/// Resolve the kube `Config`, caching it for subsequent calls.
///
/// The config is pure data (no runtime handles), so it is safe to cache
/// across await points and even across tokio runtimes in tests.
async fn resolve_config_for_context(ctx: &str) -> Result<Config> {
    let path = kubeconfig_path_string()
        .filter(|p| std::path::Path::new(p).exists())
        .context("kubeconfig file required for context switch (set KUBECONFIG or --kubeconfig)")?;
    let kubeconfig = kube::config::Kubeconfig::read_from(&path).map_err(|e| anyhow::anyhow!(e))?;
    let opts = kube::config::KubeConfigOptions {
        context: Some(ctx.to_string()),
        ..Default::default()
    };
    Config::from_custom_kubeconfig(kubeconfig, &opts)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

async fn resolve_config() -> Result<Config> {
    if let Some(ctx) = active_kube_context() {
        return resolve_config_for_context(&ctx).await;
    }
    let config = CACHED_CONFIG
        .get_or_try_init(|| async {
            let cfg = if let Some(path) =
                kubeconfig_path_string().filter(|p| std::path::Path::new(p).exists())
            {
                let kubeconfig =
                    kube::config::Kubeconfig::read_from(&path).map_err(|e| anyhow::anyhow!(e))?;
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

/// Options for [`KubeClient::migrate_vm`].
#[derive(Debug, Clone, Default)]
pub struct MigrateVmOptions {
    pub added_node_selector: Option<BTreeMap<String, String>>,
    pub priority: Option<String>,
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

    /// Create a client using an explicit kubeconfig context (does not mutate global override).
    pub async fn for_context(context: &str) -> Result<Self> {
        let config = resolve_config_for_context(context).await?;
        Ok(Self {
            client: Client::try_from(config)?,
        })
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
        let mut vm = vm_config_to_kubevirt(config)?;
        if let Ok(mut v) = serde_json::to_value(&vm) {
            let template_hint = config.labels.get("vmrogue.io/template").map(|s| s.as_str());
            if windows_rdp::should_apply_windows_rdp(&v, template_hint) {
                windows_rdp::merge_windows_rdp_defaults(&mut v);
                if let Ok(parsed) = serde_json::from_value(v) {
                    vm = parsed;
                }
            }
        }

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
                if let Err(err) = self
                    .upsert_vm_expose_service(&config.namespace, &config.name, exp)
                    .await
                {
                    log::warn!(
                        "VM '{}' created but expose Service failed: {}",
                        config.name,
                        err
                    );
                }
            }
        }

        if let Err(err) = self.apply_vm_internet_egress(config).await {
            log::warn!(
                "VM '{}' created but internet egress policy failed: {}",
                config.name,
                err
            );
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
        self.delete_service_ignore(namespace, &windows_rdp::vmrogue_rdp_service_name(name))
            .await;

        if let Err(e) =
            crate::kube::vm_internet::remove_vm_internet_egress(&self.client(), namespace, name)
                .await
        {
            log::warn!(
                "Failed to remove internet egress policy for {}/{}: {}",
                namespace,
                name,
                e
            );
        }

        Ok(())
    }

    /// Apply internet egress policy when `config.allow_internet` is set.
    pub async fn apply_vm_internet_egress(
        &self,
        config: &VMConfig,
    ) -> Result<Option<vm_internet::VmInternetStatus>> {
        if !config.allow_internet {
            return Ok(None);
        }
        let st =
            vm_internet::ensure_vm_internet_egress(&self.client(), &config.namespace, &config.name)
                .await?;
        Ok(Some(st))
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
    pub async fn upsert_vm_expose_service(
        &self,
        namespace: &str,
        vm_name: &str,
        expose: &VmExposeConfig,
    ) -> Result<()> {
        let svc_api: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        let svc_name = vm_expose_service_name(vm_name);

        let mut selector = BTreeMap::new();
        selector.insert("kubevirt.io/domain".to_string(), vm_name.to_string());

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
                namespace: Some(namespace.to_string()),
                labels: Some(BTreeMap::from([
                    ("vmrogue.io/managed-by".to_string(), "vmrogue".to_string()),
                    ("vmrogue.io/expose-for-vm".to_string(), vm_name.to_string()),
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

    /// Returns the VMRogue-managed expose `Service` for this VM, if it exists.
    pub async fn get_vm_expose_service(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<Option<Service>> {
        let svc_name = vm_expose_service_name(vm_name);
        let svc_api: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        match svc_api.get(&svc_name).await {
            Ok(s) => {
                let managed = s
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("vmrogue.io/managed-by"))
                    .map(|v| v.as_str())
                    == Some("vmrogue");
                if !managed {
                    return Ok(None);
                }
                Ok(Some(s))
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Deletes the VMRogue-managed expose `Service` for this VM (idempotent).
    pub async fn delete_vm_expose_service(&self, namespace: &str, vm_name: &str) -> Result<()> {
        let svc_name = vm_expose_service_name(vm_name);
        let svc_api: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        match svc_api.get(&svc_name).await {
            Ok(s) => {
                let managed = s
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("vmrogue.io/managed-by"))
                    .map(|v| v.as_str())
                    == Some("vmrogue");
                if !managed {
                    return Err(anyhow::anyhow!(
                        "Service {} exists but is not managed by VMRogue",
                        svc_name
                    ));
                }
            }
            Err(kube::Error::Api(ae)) if ae.code == 404 => return Ok(()),
            Err(e) => return Err(e.into()),
        }
        match svc_api.delete(&svc_name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(()),
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

    /// Resolve the `VirtualMachineInstance` resource name for VNC/serial subresources.
    ///
    /// KubeVirt subresource URLs use the VMI name, which usually matches the VM name but not
    /// always. Try `GET` by VM name first; on 404, list VMIs with `kubevirt.io/vm=<vm_name>`.
    pub async fn resolve_vmi_name_for_console(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<String> {
        let vmis: Api<VirtualMachineInstance> = Api::namespaced(self.client.clone(), namespace);
        match vmis.get(vm_name).await {
            Ok(vmi) => Ok(vmi.metadata.name.unwrap_or_else(|| vm_name.to_string())),
            Err(kube::Error::Api(resp)) if resp.code == 404 => {
                let selector = format!("kubevirt.io/vm={vm_name}");
                let lp = ListParams::default().labels(&selector);
                let list = vmis.list(&lp).await?;
                let mut candidates: Vec<VirtualMachineInstance> = list
                    .items
                    .into_iter()
                    .filter(|vmi| vmi.metadata.deletion_timestamp.is_none())
                    .collect();
                candidates.sort_by(|a, b| {
                    let ar = a.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running");
                    let br = b.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running");
                    br.cmp(&ar)
                        .then_with(|| a.metadata.name.cmp(&b.metadata.name))
                });
                let vmi = candidates.into_iter().next().ok_or_else(|| {
                    anyhow::anyhow!("No VMI found for VM {vm_name} in namespace {namespace}")
                })?;
                vmi.metadata
                    .name
                    .ok_or_else(|| anyhow::anyhow!("VMI missing metadata.name"))
            }
            Err(e) => Err(e.into()),
        }
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

    /// Pause a running VM instance via KubeVirt VMI pause subresource.
    pub async fn pause_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let vmi = self.resolve_vmi_name_for_console(namespace, name).await?;
        kubevirt_subresources::vmi_pause(self.client.clone(), namespace, &vmi).await
    }

    /// Unpause a paused VM instance via KubeVirt VMI unpause subresource.
    pub async fn unpause_vm(&self, namespace: &str, name: &str) -> Result<()> {
        let vmi = self.resolve_vmi_name_for_console(namespace, name).await?;
        kubevirt_subresources::vmi_unpause(self.client.clone(), namespace, &vmi).await
    }

    /// Trigger live migration of a VM to another node.
    pub async fn migrate_vm(
        &self,
        namespace: &str,
        name: &str,
        opts: Option<MigrateVmOptions>,
    ) -> Result<()> {
        let migrations: Api<VirtualMachineInstanceMigration> =
            Api::namespaced(self.client.clone(), namespace);

        let migration_name = format!(
            "{}-migration-{}",
            name,
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        );
        let mut spec = VirtualMachineInstanceMigrationSpec {
            vmi_name: Some(name.to_string()),
            ..Default::default()
        };
        if let Some(o) = opts {
            spec.added_node_selector = o.added_node_selector;
            spec.priority = o.priority;
        }
        let migration = VirtualMachineInstanceMigration {
            metadata: kube::api::ObjectMeta {
                name: Some(migration_name),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec,
            status: None,
        };

        let pp = PostParams::default();
        migrations.create(&pp, &migration).await?;
        Ok(())
    }

    /// List `VirtualMachineInstanceMigration` objects for a given VMI name in a namespace.
    pub async fn list_migrations_for_vmi(
        &self,
        namespace: &str,
        vmi_name: &str,
    ) -> Result<Vec<VirtualMachineInstanceMigration>> {
        let migrations: Api<VirtualMachineInstanceMigration> =
            Api::namespaced(self.client.clone(), namespace);
        let list = migrations.list(&ListParams::default()).await?;
        Ok(list
            .items
            .into_iter()
            .filter(|m| {
                m.spec
                    .vmi_name
                    .as_deref()
                    .map(|n| n == vmi_name)
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Delete a migration object by name (best-effort cancel).
    pub async fn delete_migration(&self, namespace: &str, migration_name: &str) -> Result<()> {
        let migrations: Api<VirtualMachineInstanceMigration> =
            Api::namespaced(self.client.clone(), namespace);
        migrations
            .delete(migration_name, &DeleteParams::default())
            .await?;
        Ok(())
    }

    /// Guest filesystem freeze via `virtctl` (QEMU guest agent).
    pub async fn guest_freeze(&self, namespace: &str, vmi_name: &str) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args(["freeze", "vmi", vmi_name, "-n", namespace])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "virtctl freeze failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// Unfreeze guest filesystems via `virtctl`.
    pub async fn guest_unfreeze(&self, namespace: &str, vmi_name: &str) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args(["unfreeze", "vmi", vmi_name, "-n", namespace])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "virtctl unfreeze failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// ACPI soft reboot via guest agent (`virtctl softreboot`).
    pub async fn guest_soft_reboot(&self, namespace: &str, vmi_name: &str) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args(["softreboot", "vmi", vmi_name, "-n", namespace])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "virtctl softreboot failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    async fn guest_agent_rdp_context(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<(String, serde_json::Value, serde_json::Value)> {
        let vmi_name = self
            .resolve_vmi_name_for_console(namespace, vm_name)
            .await
            .context("No backing VMI — start the VM first")?;
        let vmi = self.get_vmi(namespace, &vmi_name).await?;
        let vmi_json = serde_json::to_value(&vmi)?;
        let vm = self.get_vm(namespace, vm_name).await?;
        let vm_json = serde_json::to_value(&vm)?;
        Ok((vmi_name, vmi_json, vm_json))
    }

    /// Enable Windows Remote Desktop in a running guest via QEMU guest-agent (`guest-exec`).
    pub async fn enable_rdp_via_guest_agent(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<windows_rdp::RdpGuestAgentResponse> {
        let (vmi_name, vmi_json, vm_json) =
            self.guest_agent_rdp_context(namespace, vm_name).await?;
        windows_rdp::set_rdp_via_guest_agent(
            self.client.clone(),
            namespace,
            vm_name,
            &vmi_name,
            &vmi_json,
            &vm_json,
            true,
        )
        .await
    }

    /// Disable Windows Remote Desktop in a running guest via QEMU guest-agent (`guest-exec`).
    pub async fn disable_rdp_via_guest_agent(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<windows_rdp::RdpGuestAgentResponse> {
        let (vmi_name, vmi_json, vm_json) =
            self.guest_agent_rdp_context(namespace, vm_name).await?;
        windows_rdp::set_rdp_via_guest_agent(
            self.client.clone(),
            namespace,
            vm_name,
            &vmi_name,
            &vmi_json,
            &vm_json,
            false,
        )
        .await
    }

    /// In-guest filesystem usage via QEMU guest-agent (`df` / PowerShell) with optional Prometheus enrichment.
    pub async fn guest_filesystem_metrics(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<guest_filesystem::GuestFilesystemResponse> {
        let vm = self.get_vm(namespace, vm_name).await?;
        let vmi_name = self
            .resolve_vmi_name_for_console(namespace, vm_name)
            .await
            .unwrap_or_else(|_| vm_name.to_string());
        let vmi_json = self
            .get_vmi(namespace, &vmi_name)
            .await
            .ok()
            .and_then(|v| serde_json::to_value(&v).ok());
        let vm_json = serde_json::to_value(&vm)?;
        let agent_up = vmi_json
            .as_ref()
            .is_some_and(windows_rdp::vmi_guest_agent_connected);

        let mut prom_mounts = Vec::new();
        let mut prom_sources = Vec::new();
        if let Ok(prom_url) = std::env::var("VEYRON_PROMETHEUS_URL") {
            if !prom_url.trim().is_empty() {
                let (m, s) = guest_prometheus::prometheus_storage_mounts_for_vm(
                    &self.client,
                    prom_url.trim(),
                    namespace,
                    vm_name,
                    &vm,
                )
                .await;
                prom_mounts = m;
                prom_sources = s;
            }
        }

        if agent_up {
            let vmi_j = vmi_json.as_ref().unwrap_or(&serde_json::Value::Null);
            let kind = guest_runtime::detect_runtime_kind(
                self.client.clone(),
                namespace,
                &vmi_name,
                Some(&vm_json),
                Some(vmi_j),
            )
            .await;
            let windows = guest_os::detect_guest_os_family(Some(&vm_json), vmi_json.as_ref())
                == guest_os::GuestOsFamily::Windows;
            if kind == guest_runtime::GuestRuntimeKind::GuestKit && !windows {
                if let Ok(mut resp) = guest_filesystem::collect_guest_filesystem_guestkit(
                    self.client.clone(),
                    namespace,
                    vm_name,
                    &vmi_name,
                    &vm_json,
                    vmi_j,
                )
                .await
                {
                    resp.prometheus_mounts = prom_mounts.clone();
                    resp.sources.extend(prom_sources);
                    return Ok(resp);
                }
            }
            let mut resp = guest_filesystem::collect_guest_filesystem(
                self.client.clone(),
                namespace,
                vm_name,
                &vmi_name,
                &vm_json,
                vmi_j,
            )
            .await?;
            resp.guest_runtime =
                Some(guest_runtime::runtime_kind_label(kind).to_string());
            resp.prometheus_mounts = prom_mounts.clone();
            resp.sources.extend(prom_sources);
            return Ok(resp);
        }

        if !prom_mounts.is_empty() {
            let family = guest_os::detect_guest_os_family(Some(&vm_json), vmi_json.as_ref());
            return Ok(guest_filesystem::GuestFilesystemResponse {
                namespace: namespace.to_string(),
                vm_name: vm_name.to_string(),
                vmi_name,
                guest_agent_connected: false,
                guest_runtime: None,
                os_family: family.as_str().to_string(),
                mounts: prom_mounts.clone(),
                prometheus_mounts: prom_mounts,
                sources: prom_sources,
                message: Some(
                    "Guest agent not connected — showing Prometheus PVC / exporter metrics only."
                        .into(),
                ),
                exit_code: None,
                stderr: None,
            });
        }

        anyhow::bail!(
            "Guest runtime is not connected and no Prometheus storage metrics were found (set VEYRON_PROMETHEUS_URL)"
        );
    }

    async fn build_guest_context(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<guest_runtime::GuestContext> {
        let vm = self.get_vm(namespace, vm_name).await?;
        let vmi_name = self
            .resolve_vmi_name_for_console(namespace, vm_name)
            .await
            .unwrap_or_else(|_| vm_name.to_string());
        let vmi = self.get_vmi(namespace, &vmi_name).await?;
        let vm_json = serde_json::to_value(&vm)?;
        let vmi_json = serde_json::to_value(&vmi)?;
        let mut ctx = guest_runtime::GuestContext::with_json(
            self.client.clone(),
            namespace,
            vm_name,
            &vmi_name,
            vm_json,
            vmi_json,
        );
        ctx.resolve_runtime_kind().await;
        Ok(ctx)
    }

    /// Guest runtime status (GuestKit or QGA) for a running VM.
    pub async fn guest_runtime_status(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<guest_runtime::GuestRuntimeStatus> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        let connected = ctx.connected();
        let mut version = None;
        let mut capabilities = None;
        if connected && ctx.runtime_kind == guest_runtime::GuestRuntimeKind::GuestKit {
            if let Ok(v) = guest_runtime::guestkit_version(&ctx).await {
                version = v
                    .get("version")
                    .or_else(|| v.get("agent_version"))
                    .and_then(|x| x.as_str())
                    .map(str::to_string);
            }
            capabilities = guest_runtime::guestkit_capabilities(&ctx).await.ok();
        }
        Ok(guest_runtime::GuestRuntimeStatus {
            connected,
            runtime_kind: guest_runtime::runtime_kind_label(ctx.runtime_kind).to_string(),
            guest_agent_connected: connected,
            version,
            capabilities,
        })
    }

    /// Live GuestKit evidence snapshot (Linux GuestKit VMs).
    pub async fn guest_evidence(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<guestkit::evidence::EvidenceSnapshot> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit evidence requires GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_evidence(&ctx).await
    }

    /// Live GuestKit doctor report.
    pub async fn guest_doctor(
        &self,
        namespace: &str,
        vm_name: &str,
        target: &str,
    ) -> Result<serde_json::Value> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit doctor requires GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_doctor(&ctx, target).await
    }

    /// Apply a GuestKit fix plan in the guest.
    pub async fn guest_fix_plan(
        &self,
        namespace: &str,
        vm_name: &str,
        plan: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit fix-plan requires GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_fix_plan(&ctx, plan).await
    }

    /// Live GuestKit runtime metrics (CPU, memory, disk, network).
    pub async fn guest_metrics(
        &self,
        namespace: &str,
        vm_name: &str,
    ) -> Result<serde_json::Value> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit metrics require GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_metrics(&ctx).await
    }

    /// GuestKit P2V / migration readiness score for a VM.
    pub async fn guest_migrate_score(
        &self,
        namespace: &str,
        vm_name: &str,
        target: &str,
    ) -> Result<serde_json::Value> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit migrate score requires GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_migrate_score(&ctx, target).await
    }

    /// Run a command in the guest via GuestKit exec (replaces K8s guest-exec when unavailable).
    pub async fn guest_exec_via_guestkit(
        &self,
        namespace: &str,
        vm_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let ctx = self.build_guest_context(namespace, vm_name).await?;
        if !ctx.connected() {
            anyhow::bail!("Guest runtime is not connected (AgentConnected condition is not True)");
        }
        if ctx.runtime_kind != guest_runtime::GuestRuntimeKind::GuestKit {
            anyhow::bail!("GuestKit exec requires GuestKit guest runtime (Linux VM)");
        }
        guest_runtime::guestkit_exec(&ctx, params).await
    }

    /// Read bundled GuestKit musl binary for cloud-init / platform endpoint.
    pub fn guestkit_binary_bytes() -> Result<Vec<u8>> {
        std::fs::read(guestkit_client::guestkit_binary_path()).with_context(|| {
            format!(
                "read guestkit binary at {}",
                guestkit_client::guestkit_binary_path().display()
            )
        })
    }

    /// Hotplug a PVC volume onto a VM (`virtctl addvolume`).
    pub async fn add_vm_volume(
        &self,
        namespace: &str,
        vm_name: &str,
        volume_name: &str,
        pvc_name: &str,
    ) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args([
                "addvolume",
                vm_name,
                "--volume-name",
                volume_name,
                "--pvc",
                pvc_name,
                "-n",
                namespace,
            ])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "virtctl addvolume failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// Remove a hotplug volume (`virtctl removevolume`).
    pub async fn remove_vm_volume(
        &self,
        namespace: &str,
        vm_name: &str,
        volume_name: &str,
    ) -> Result<()> {
        let output = tokio::process::Command::new("virtctl")
            .args([
                "removevolume",
                vm_name,
                "--volume-name",
                volume_name,
                "-n",
                namespace,
            ])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "virtctl removevolume failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// Return `status.volumeStatus` from the VMI (hotplug / volume health), if present.
    pub async fn get_vmi_volume_status_json(
        &self,
        namespace: &str,
        vmi_name: &str,
    ) -> Result<serde_json::Value> {
        let vmi = self.get_vmi(namespace, vmi_name).await?;
        let v = serde_json::to_value(&vmi)?;
        Ok(v.pointer("/status/volumeStatus")
            .cloned()
            .unwrap_or(serde_json::json!([])))
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

    /// KubeVirt / CDI signals for Veyron Stack Health.
    pub async fn platform_readiness(&self) -> PlatformReadiness {
        const VIRTCTL: &str =
            "virtctl runs on your workstation — optional for the dashboard; use for CLI console/VNC.";

        let (kubevirt_api_ok, kubevirt_api_message) = match self.list_all_vms().await {
            Ok(_) => (true, None),
            Err(e) => {
                let mut s = e.to_string();
                if s.len() > 400 {
                    s.truncate(400);
                    s.push('…');
                }
                (false, Some(s))
            }
        };

        let kv_pods = self.list_pods("kubevirt").await.unwrap_or_default();
        let kv_running: Vec<_> = kv_pods.iter().filter(|p| pod_is_running(p)).collect();
        let has_virt_api = kv_running
            .iter()
            .any(|p| pod_name_str(p).contains("virt-api"));
        let has_virt_controller = kv_running
            .iter()
            .any(|p| pod_name_str(p).contains("virt-controller"));
        let kubevirt_control_plane_ok = has_virt_api && has_virt_controller;
        let kubevirt_detail = if kv_pods.is_empty() && kv_running.is_empty() {
            "No pods listed in namespace kubevirt (missing namespace or RBAC).".to_string()
        } else {
            format!(
                "kubevirt: {} Running pods — virt-api {}, virt-controller {}",
                kv_running.len(),
                if has_virt_api { "ok" } else { "missing" },
                if has_virt_controller { "ok" } else { "missing" }
            )
        };

        let cdi_pods = self.list_pods("cdi").await.unwrap_or_default();
        let cdi_running: Vec<_> = cdi_pods.iter().filter(|p| pod_is_running(p)).collect();
        let has_cdi_op = cdi_running
            .iter()
            .any(|p| pod_name_str(p).contains("cdi-operator"));
        let cdi_operator_ok = has_cdi_op;
        let cdi_detail = if cdi_pods.is_empty() {
            "Namespace cdi: no pods (CDI not installed or no RBAC).".to_string()
        } else {
            format!(
                "cdi: {} Running pods — cdi-operator {}",
                cdi_running.len(),
                if has_cdi_op { "ok" } else { "missing" }
            )
        };

        let forge_vm_ready = kubevirt_api_ok && kubevirt_control_plane_ok;

        PlatformReadiness {
            kubevirt_api_ok,
            kubevirt_api_message,
            kubevirt_control_plane_ok,
            kubevirt_detail,
            cdi_operator_ok,
            cdi_detail,
            forge_vm_ready,
            virtctl_note: VIRTCTL,
        }
    }
}

fn pod_is_running(p: &k8s_openapi::api::core::v1::Pod) -> bool {
    p.status
        .as_ref()
        .and_then(|s| s.phase.as_ref())
        .map(|ph| ph == "Running")
        .unwrap_or(false)
}

fn pod_name_str(p: &k8s_openapi::api::core::v1::Pod) -> &str {
    p.metadata.name.as_deref().unwrap_or("")
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
