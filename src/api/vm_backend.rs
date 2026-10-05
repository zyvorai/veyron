// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Which platform runs VMs behind `/api/v1/vms`.
//!
//! `VEYRON_VM_BACKEND=kubevirt` (default) keeps the native KubeVirt path, including
//! the VMI/drift enrichment in the list and get handlers. `VEYRON_VM_BACKEND=kairon`
//! (requires the `kairon` cargo feature) serves VM CRUD and power operations from
//! `kairon.zyvor.dev` Machines instead.

use crate::config::VMConfig;
use crate::kube::KubeClient;
use crate::tui::state::VmInfo;
use anyhow::Result;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    KubeVirt,
    #[cfg(feature = "kairon")]
    Kairon,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::KubeVirt => "kubevirt",
            #[cfg(feature = "kairon")]
            BackendKind::Kairon => "kairon",
        }
    }
}

pub fn parse_backend(value: Option<&str>) -> BackendKind {
    match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        None | Some("") | Some("kubevirt") => BackendKind::KubeVirt,
        #[cfg(feature = "kairon")]
        Some("kairon") => BackendKind::Kairon,
        Some(other) => {
            log::warn!("VEYRON_VM_BACKEND={other} is not available in this build; using kubevirt");
            BackendKind::KubeVirt
        }
    }
}

static SELECTED: Lazy<BackendKind> =
    Lazy::new(|| parse_backend(std::env::var("VEYRON_VM_BACKEND").ok().as_deref()));

pub fn selected() -> BackendKind {
    *SELECTED
}

/// VM lifecycle operations shared by every backend.
pub enum VmBackend {
    KubeVirt(KubeClient),
    #[cfg(feature = "kairon")]
    Kairon,
}

impl VmBackend {
    pub fn for_client(client: &KubeClient) -> Self {
        match selected() {
            BackendKind::KubeVirt => VmBackend::KubeVirt(client.clone()),
            #[cfg(feature = "kairon")]
            BackendKind::Kairon => VmBackend::Kairon,
        }
    }

    pub fn kind(&self) -> BackendKind {
        match self {
            VmBackend::KubeVirt(_) => BackendKind::KubeVirt,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => BackendKind::Kairon,
        }
    }

    pub async fn list(&self, scope_ns: &str) -> Result<Vec<VmInfo>> {
        match self {
            VmBackend::KubeVirt(c) => Ok(c
                .list_vms_for_scope(scope_ns)
                .await
                .iter()
                .map(VmInfo::from_vm)
                .collect()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::list(scope_ns).await,
        }
    }

    /// Full object as JSON (`VirtualMachine` or `Machine`).
    pub async fn get(&self, ns: &str, name: &str) -> Result<serde_json::Value> {
        match self {
            VmBackend::KubeVirt(c) => Ok(serde_json::to_value(c.get_vm(ns, name).await?)?),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::get(ns, name).await,
        }
    }

    pub async fn create(&self, config: &VMConfig) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.create_vm(config).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::create(config).await,
        }
    }

    pub async fn delete(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.delete_vm(ns, name).await,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::delete(ns, name).await,
        }
    }

    pub async fn start(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.start_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::set_power(ns, name, crate::kairon::POWER_RUNNING).await,
        }
    }

    pub async fn stop(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.stop_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::set_power(ns, name, crate::kairon::POWER_STOPPED).await,
        }
    }

    pub async fn restart(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.restart_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon => kairon::restart(ns, name).await,
        }
    }
}

#[cfg(feature = "kairon")]
mod kairon {
    use super::*;
    use crate::kairon::{
        LABEL_MANAGED_BY, Machine, POWER_RUNNING, POWER_STOPPED, machine_from_config,
    };
    use anyhow::{Context, anyhow};
    use chrono::Utc;
    use kube::Api;
    use kube::api::{DeleteParams, ListParams, Patch, PatchParams, PostParams};
    use std::time::Duration;

    async fn api(ns: Option<&str>) -> Result<Api<Machine>> {
        let client = crate::kube::get_client().await?;
        Ok(match ns {
            Some(ns) => Api::namespaced(client, ns),
            None => Api::all(client),
        })
    }

    fn age(m: &Machine) -> String {
        m.metadata
            .creation_timestamp
            .as_ref()
            .map(|created| {
                let d = Utc::now().signed_duration_since(created.0);
                let (days, hours, minutes) =
                    (d.num_days(), d.num_hours() % 24, d.num_minutes() % 60);
                if days > 0 {
                    format!("{days}d{hours}h")
                } else if hours > 0 {
                    format!("{hours}h{minutes}m")
                } else {
                    format!("{minutes}m")
                }
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(super) fn machine_info(m: &Machine) -> VmInfo {
        VmInfo {
            name: m.name().to_string(),
            namespace: m.namespace().to_string(),
            status: m.printable_status(),
            cpu: m.vcpus().to_string(),
            memory: if m.memory().is_empty() {
                "N/A".to_string()
            } else {
                m.memory().to_string()
            },
            age: age(m),
            ready: m.is_running(),
            disk: m.image_ref().unwrap_or_else(|| "N/A".to_string()),
            ip: m.guest_ip().unwrap_or_default(),
            node: m.node().unwrap_or_default().to_string(),
            veyron_managed: None,
            drift_detected: None,
            drift_message: None,
            guest_agent_connected: None,
            gpu_count: m
                .has_device_claims()
                .then_some(m.spec.device_claims.len() as u32),
        }
    }

    pub(super) async fn list(scope_ns: &str) -> Result<Vec<VmInfo>> {
        let ns = (scope_ns != "all").then_some(scope_ns);
        let machines = api(ns).await?.list(&ListParams::default()).await?;
        Ok(machines.items.iter().map(machine_info).collect())
    }

    pub(super) async fn get(ns: &str, name: &str) -> Result<serde_json::Value> {
        let m = api(Some(ns)).await?.get(name).await?;
        let mut v = serde_json::to_value(&m)?;
        v["info"] = serde_json::to_value(machine_info(&m))?;
        Ok(v)
    }

    pub(super) async fn create(config: &VMConfig) -> Result<()> {
        let mut machine = machine_from_config(config)?;
        machine.metadata.namespace = Some(config.namespace.clone());
        machine
            .metadata
            .labels
            .get_or_insert_with(Default::default)
            .insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
        api(Some(&config.namespace))
            .await?
            .create(&PostParams::default(), &machine)
            .await
            .with_context(|| format!("creating Machine {}/{}", config.namespace, config.name))?;
        Ok(())
    }

    pub(super) async fn delete(ns: &str, name: &str) -> Result<()> {
        api(Some(ns))
            .await?
            .delete(name, &DeleteParams::default())
            .await?;
        Ok(())
    }

    pub(super) async fn set_power(ns: &str, name: &str, state: &str) -> Result<()> {
        let patch = serde_json::json!({ "spec": { "powerState": state } });
        api(Some(ns))
            .await?
            .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;
        Ok(())
    }

    /// Kairon has no restart verb: power off, wait for the controller to report the
    /// machine stopped, then power back on.
    pub(super) async fn restart(ns: &str, name: &str) -> Result<()> {
        set_power(ns, name, POWER_STOPPED).await?;
        let machines = api(Some(ns)).await?;
        let wait = async {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if !machines.get(name).await?.is_running() {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(60), wait)
            .await
            .map_err(|_| anyhow!("Machine '{name}' did not stop within 60 seconds"))??;
        set_power(ns, name, POWER_RUNNING).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_kubevirt() {
        assert_eq!(parse_backend(None), BackendKind::KubeVirt);
        assert_eq!(parse_backend(Some("")), BackendKind::KubeVirt);
        assert_eq!(parse_backend(Some("KubeVirt")), BackendKind::KubeVirt);
        assert_eq!(parse_backend(Some("bogus")), BackendKind::KubeVirt);
    }

    #[cfg(feature = "kairon")]
    #[test]
    fn selects_kairon() {
        assert_eq!(parse_backend(Some(" Kairon ")), BackendKind::Kairon);
        assert_eq!(BackendKind::Kairon.as_str(), "kairon");
    }

    #[cfg(feature = "kairon")]
    #[test]
    fn machine_maps_to_vm_info() {
        use crate::kairon::{Machine, MachineSpec, MachineStatus};
        let mut m = Machine::new("web", MachineSpec::default());
        m.metadata.namespace = Some("prod".into());
        m.spec.resources.cpu = "2".into();
        m.spec.resources.memory = "4Gi".into();
        m.status = Some(MachineStatus {
            phase: Some("Running".into()),
            guest_ip: Some("10.0.0.5".into()),
            ..Default::default()
        });
        let info = kairon::machine_info(&m);
        assert_eq!(info.namespace, "prod");
        assert_eq!(info.status, "Running");
        assert_eq!(info.cpu, "2");
        assert_eq!(info.memory, "4Gi");
        assert_eq!(info.ip, "10.0.0.5");
        assert!(info.ready);
    }
}
