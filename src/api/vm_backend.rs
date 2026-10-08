// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Which platform runs VMs behind `/api/v1/vms`.
//!
//! Kairon (`kairon.zyvor.dev` Machines, the `kairon` cargo feature, on by default) is the
//! VM engine: unset, `auto`, `kairon` and unknown values of `VEYRON_VM_BACKEND` all select
//! it, and Veyron never falls back to KubeVirt on its own. `VEYRON_VM_BACKEND=kubevirt` is
//! an explicit legacy opt-in; builds without the `kairon` feature can only run KubeVirt.

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
        Some("kubevirt") => BackendKind::KubeVirt,
        #[cfg(feature = "kairon")]
        None | Some("") | Some("auto") | Some("kairon") => BackendKind::Kairon,
        #[cfg(feature = "kairon")]
        Some(other) => {
            log::warn!("VEYRON_VM_BACKEND={other} is not a backend; using kairon");
            BackendKind::Kairon
        }
        #[cfg(not(feature = "kairon"))]
        _ => BackendKind::KubeVirt,
    }
}

static CONFIGURED: Lazy<Option<String>> = Lazy::new(|| {
    std::env::var("VEYRON_VM_BACKEND")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
});

pub fn selected() -> BackendKind {
    static SELECTED: Lazy<BackendKind> = Lazy::new(|| parse_backend(CONFIGURED.as_deref()));
    *SELECTED
}

/// Log the active backend at startup and warn when its CRDs are missing. Never switches
/// backend: a Kairon build without Kairon installed reports errors instead of quietly
/// serving KubeVirt.
pub async fn resolve(client: &kube::Client) -> BackendKind {
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    let kind = selected();
    let crd = match kind {
        BackendKind::KubeVirt => "virtualmachines.kubevirt.io",
        #[cfg(feature = "kairon")]
        BackendKind::Kairon => "machines.kairon.zyvor.dev",
    };
    let crds = kube::Api::<CustomResourceDefinition>::all(client.clone());
    match crds.get_opt(crd).await {
        Ok(Some(_)) => log::info!("VM backend: {}", kind.as_str()),
        Ok(None) => log::warn!(
            "VM backend {} selected but CRD {crd} is not installed; VM routes will fail until it is",
            kind.as_str()
        ),
        Err(e) => log::warn!(
            "VM backend {}: could not check CRD {crd}: {e}",
            kind.as_str()
        ),
    }
    kind
}

pub fn is_kairon() -> bool {
    selected() != BackendKind::KubeVirt
}

/// VM lifecycle operations shared by every backend.
pub enum VmBackend {
    KubeVirt(KubeClient),
    #[cfg(feature = "kairon")]
    Kairon(KubeClient),
}

impl VmBackend {
    pub fn for_client(client: &KubeClient) -> Self {
        match selected() {
            BackendKind::KubeVirt => VmBackend::KubeVirt(client.clone()),
            #[cfg(feature = "kairon")]
            BackendKind::Kairon => VmBackend::Kairon(client.clone()),
        }
    }

    pub fn kind(&self) -> BackendKind {
        match self {
            VmBackend::KubeVirt(_) => BackendKind::KubeVirt,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(_) => BackendKind::Kairon,
        }
    }

    pub async fn list(&self, scope_ns: &str) -> Result<Vec<VmInfo>> {
        match self {
            VmBackend::KubeVirt(c) => {
                let vms = if scope_ns == "all" {
                    c.list_all_vms().await?
                } else {
                    c.list_vms(scope_ns).await?
                };
                Ok(vms.iter().map(VmInfo::from_vm).collect())
            }
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::list(c, scope_ns).await,
        }
    }

    /// Full object as JSON (`VirtualMachine` or `Machine`).
    pub async fn get(&self, ns: &str, name: &str) -> Result<serde_json::Value> {
        match self {
            VmBackend::KubeVirt(c) => Ok(serde_json::to_value(c.get_vm(ns, name).await?)?),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::get(c, ns, name).await,
        }
    }

    fn kube_client(&self) -> &KubeClient {
        match self {
            VmBackend::KubeVirt(c) => c,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => c,
        }
    }

    pub async fn create(&self, config: &VMConfig) -> Result<()> {
        crate::api::tenant_admission::reserve(self.kube_client(), config).await?;
        match self {
            VmBackend::KubeVirt(c) => c.create_vm(config).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::create(c, config).await,
        }
    }

    pub async fn delete(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.delete_vm(ns, name).await,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::delete(c, ns, name).await,
        }
    }

    pub async fn start(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.start_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => {
                kairon::set_power(c, ns, name, crate::kairon::POWER_RUNNING).await
            }
        }
    }

    pub async fn stop(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.stop_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::stop(c, ns, name).await,
        }
    }

    pub async fn restart(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.restart_vm(ns, name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::restart(c, ns, name).await,
        }
    }

    pub async fn pause(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.pause_vm(ns, name).await,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::pause(c, ns, name, true).await,
        }
    }

    pub async fn unpause(&self, ns: &str, name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.unpause_vm(ns, name).await,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::pause(c, ns, name, false).await,
        }
    }

    pub async fn clone_vm(&self, ns: &str, name: &str, new_name: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.clone_vm(ns, name, new_name).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::clone(c, ns, name, new_name).await,
        }
    }

    /// KubeVirt run strategy (`Always|Manual|Halted|RerunOnFailure`); Kairon maps it
    /// onto `spec.powerState`.
    pub async fn set_run_strategy(&self, ns: &str, name: &str, strategy: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.set_run_strategy(ns, name, strategy).await.map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => match crate::kairon::power_state_for_run_strategy(strategy) {
                Some(crate::kairon::POWER_STOPPED) => kairon::stop(c, ns, name).await,
                Some(state) => kairon::set_power(c, ns, name, state).await,
                None => anyhow::bail!("unknown run strategy '{strategy}'"),
            },
        }
    }

    pub async fn hotplug(
        &self,
        ns: &str,
        name: &str,
        cpus: Option<u32>,
        memory: Option<&str>,
    ) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c
                .hotplug_vm_resources(ns, name, cpus, memory)
                .await
                .map(|_| ()),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::hotplug(c, ns, name, cpus, memory).await,
        }
    }

    /// Start a migration and return its object name. KubeVirt callers run their
    /// own eligibility gate first; Kairon refuses live-migrating VFIO devices here.
    pub async fn migrate(&self, ns: &str, name: &str, req: &MigrateRequest) -> Result<String> {
        match self {
            VmBackend::KubeVirt(c) => {
                let opts = req
                    .target_node
                    .as_ref()
                    .map(|host| crate::kube::MigrateVmOptions {
                        added_node_selector: Some(
                            [("kubernetes.io/hostname".to_string(), host.clone())].into(),
                        ),
                        priority: None,
                    });
                c.migrate_vm(ns, name, opts).await?;
                Ok(String::new())
            }
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::migrate(c, ns, name, req).await,
        }
    }

    pub async fn list_migrations(&self, ns: &str, name: &str) -> Result<serde_json::Value> {
        match self {
            VmBackend::KubeVirt(c) => Ok(serde_json::to_value(
                c.list_migrations_for_vmi(ns, name).await?,
            )?),
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::list_migrations(c, ns, name).await,
        }
    }

    /// KubeVirt deletes the migration object; Kairon sets `spec.cancel` so the
    /// controller rolls back cleanly before commit.
    pub async fn cancel_migration(&self, ns: &str, migration: &str) -> Result<()> {
        match self {
            VmBackend::KubeVirt(c) => c.delete_migration(ns, migration).await,
            #[cfg(feature = "kairon")]
            VmBackend::Kairon(c) => kairon::cancel_migration(c, ns, migration).await,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MigrateRequest {
    pub target_node: Option<String>,
    /// Kairon only: `auto` | `live` | `cold`.
    pub strategy: Option<String>,
    pub force: bool,
}

/// Returned when a migration is refused because of the VM's own state.
#[derive(Debug)]
pub struct MigrationBlocked(pub String);

impl std::fmt::Display for MigrationBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MigrationBlocked {}

#[cfg(feature = "kairon")]
mod kairon {
    use super::*;
    use crate::kairon::{
        LABEL_MANAGED_BY, Machine, MachineMigration, MachineMigrationSpec, POWER_HALTED,
        POWER_PAUSED, POWER_RUNNING, POWER_STOPPED, machine_from_config,
    };
    use anyhow::{Context, anyhow};
    use chrono::Utc;
    use kube::Api;
    use kube::api::{DeleteParams, ListParams, Patch, PatchParams, PostParams};
    use std::time::Duration;

    fn api(c: &KubeClient, ns: Option<&str>) -> Api<Machine> {
        let client = c.client();
        match ns {
            Some(ns) => Api::namespaced(client, ns),
            None => Api::all(client),
        }
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
            os: m.image_ref(),
            cdroms: m.spec.cdroms.iter().map(|c| c.name.clone()).collect(),
        }
    }

    pub(super) async fn list(c: &KubeClient, scope_ns: &str) -> Result<Vec<VmInfo>> {
        let ns = (scope_ns != "all").then_some(scope_ns);
        let machines = api(c, ns).list(&ListParams::default()).await?;
        Ok(machines.items.iter().map(machine_info).collect())
    }

    pub(super) async fn get(c: &KubeClient, ns: &str, name: &str) -> Result<serde_json::Value> {
        let m = api(c, Some(ns)).get(name).await?;
        let mut v = serde_json::to_value(&m)?;
        v["info"] = serde_json::to_value(machine_info(&m))?;
        Ok(v)
    }

    pub(super) async fn create(c: &KubeClient, config: &VMConfig) -> Result<()> {
        let mut machine = machine_from_config(config)?;
        machine.metadata.namespace = Some(config.namespace.clone());
        let unpinned_oci = machine
            .spec
            .image
            .source
            .as_ref()
            .and_then(|s| s.oci.clone())
            .filter(|_| machine.spec.image.digest.is_none());
        if let Some(oci) = unpinned_oci {
            let digest = crate::kairon::oci::resolve_digest(&oci)
                .await
                .with_context(|| format!("pinning {oci} to a digest for Kairon"))?;
            machine.spec.image.digest = Some(digest);
        }
        machine
            .metadata
            .labels
            .get_or_insert_with(Default::default)
            .insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
        api(c, Some(&config.namespace))
            .create(&PostParams::default(), &machine)
            .await
            .with_context(|| format!("creating Machine {}/{}", config.namespace, config.name))?;
        Ok(())
    }

    pub(super) async fn delete(c: &KubeClient, ns: &str, name: &str) -> Result<()> {
        api(c, Some(ns))
            .delete(name, &DeleteParams::default())
            .await?;
        Ok(())
    }

    pub(super) async fn set_power(c: &KubeClient, ns: &str, name: &str, state: &str) -> Result<()> {
        let patch = serde_json::json!({ "spec": { "powerState": state } });
        api(c, Some(ns))
            .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;
        Ok(())
    }

    /// `Stopped` tears the FluxVM runtime down and rebuilds it from `spec` on the next
    /// start, which throws away an image-booted Machine's disk. `Halted` powers the VM
    /// off and keeps its disk, matching a KubeVirt stop; it needs an existing runtime.
    pub(super) fn stop_state(m: &Machine) -> &'static str {
        if m.runtime_id().is_some() {
            POWER_HALTED
        } else {
            POWER_STOPPED
        }
    }

    pub(super) async fn stop(c: &KubeClient, ns: &str, name: &str) -> Result<()> {
        let m = api(c, Some(ns)).get(name).await?;
        set_power(c, ns, name, stop_state(&m)).await
    }

    /// Kairon has no restart verb: power off, wait for the controller to report the
    /// machine stopped, then power back on.
    pub(super) async fn restart(c: &KubeClient, ns: &str, name: &str) -> Result<()> {
        stop(c, ns, name).await?;
        let machines = api(c, Some(ns));
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
        set_power(c, ns, name, POWER_RUNNING).await
    }

    pub(super) async fn pause(c: &KubeClient, ns: &str, name: &str, paused: bool) -> Result<()> {
        let m = api(c, Some(ns)).get(name).await?;
        let state = m.power_state();
        if paused && state != POWER_RUNNING {
            anyhow::bail!("Machine '{name}' is not running (powerState {state}) (conflict)");
        }
        if !paused && state != POWER_PAUSED {
            anyhow::bail!("Machine '{name}' is not paused (powerState {state}) (conflict)");
        }
        let target = if paused { POWER_PAUSED } else { POWER_RUNNING };
        set_power(c, ns, name, target).await
    }

    /// A clone boots a fresh copy of the source's image and settings. Machines that
    /// boot from or attach PVCs are refused: sharing a volume between two VMs
    /// corrupts it, and copying it needs a snapshot restore into a new claim.
    pub(super) async fn clone(c: &KubeClient, ns: &str, name: &str, new_name: &str) -> Result<()> {
        let src = api(c, Some(ns)).get(name).await?;
        let m = clone_machine(&src, new_name)?;
        api(c, Some(ns))
            .create(&PostParams::default(), &m)
            .await
            .with_context(|| format!("creating Machine {ns}/{new_name}"))?;
        Ok(())
    }

    pub(super) fn clone_machine(src: &Machine, new_name: &str) -> Result<Machine> {
        if !src.spec.volumes.is_empty() || !src.spec.disks.is_empty() {
            anyhow::bail!(
                "Machine '{}' uses PersistentVolumeClaims; snapshot it and restore into a new claim instead of cloning",
                src.name()
            );
        }
        let mut spec = src.spec.clone();
        spec.node_name = None;
        if let Some(net) = spec.network.as_mut() {
            net.mac = None;
            net.forwards.clear();
        }
        if let Some(ci) = spec.cloud_init.as_mut() {
            ci.hostname = Some(new_name.to_string());
        }
        let mut m = Machine::new(new_name, spec);
        m.metadata.namespace = Some(src.namespace().to_string());
        let labels = m.metadata.labels.get_or_insert_with(Default::default);
        labels.insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
        labels.insert("veyron.io/cloned-from".to_string(), src.name().to_string());
        Ok(m)
    }

    /// Kairon hotplug is grow-only and bounded by `maxCpu`/`maxMemory` set at creation;
    /// kairon-controller rejects anything else, so its message is passed through.
    pub(super) async fn hotplug(
        c: &KubeClient,
        ns: &str,
        name: &str,
        cpus: Option<u32>,
        memory: Option<&str>,
    ) -> Result<()> {
        let mut resources = serde_json::Map::new();
        if let Some(n) = cpus {
            resources.insert("cpu".into(), n.to_string().into());
        }
        if let Some(mem) = memory {
            resources.insert("memory".into(), mem.into());
        }
        let patch = serde_json::json!({ "spec": { "resources": resources } });
        api(c, Some(ns))
            .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;
        Ok(())
    }

    fn migrations(c: &KubeClient, ns: &str) -> Api<MachineMigration> {
        Api::namespaced(c.client(), ns)
    }

    pub(super) fn migration_for(
        m: &Machine,
        req: &MigrateRequest,
    ) -> std::result::Result<MachineMigration, MigrationBlocked> {
        if !m.is_running() {
            return Err(MigrationBlocked(format!(
                "Machine '{}' is not running (phase {})",
                m.name(),
                m.phase()
            )));
        }
        let strategy = req.strategy.as_deref().unwrap_or("auto");
        if m.has_device_claims() && strategy != "cold" && !req.force {
            return Err(MigrationBlocked(format!(
                "Machine '{}' has passthrough devices, which cannot live-migrate; use strategy \"cold\"",
                m.name()
            )));
        }
        let stamp = Utc::now().format("%Y%m%d%H%M%S");
        let mut mig = MachineMigration::new(
            &format!("{}-mig-{stamp}", m.name()),
            MachineMigrationSpec {
                machine_name: m.name().to_string(),
                strategy: Some(strategy.to_string()),
                target_node: req.target_node.clone(),
                ..Default::default()
            },
        );
        mig.metadata.namespace = Some(m.namespace().to_string());
        mig.metadata
            .labels
            .get_or_insert_with(Default::default)
            .insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
        Ok(mig)
    }

    pub(super) async fn migrate(
        c: &KubeClient,
        ns: &str,
        name: &str,
        req: &MigrateRequest,
    ) -> Result<String> {
        let m = api(c, Some(ns)).get(name).await?;
        let mig = migration_for(&m, req)?;
        let created = migrations(c, ns)
            .create(&PostParams::default(), &mig)
            .await
            .with_context(|| format!("creating MachineMigration for {ns}/{name}"))?;
        Ok(created.metadata.name.unwrap_or_default())
    }

    pub(super) async fn list_migrations(
        c: &KubeClient,
        ns: &str,
        name: &str,
    ) -> Result<serde_json::Value> {
        let mut items: Vec<MachineMigration> = migrations(c, ns)
            .list(&ListParams::default())
            .await?
            .items
            .into_iter()
            .filter(|mig| mig.spec.machine_name == name)
            .collect();
        items.sort_by(|a, b| {
            b.metadata
                .creation_timestamp
                .cmp(&a.metadata.creation_timestamp)
        });
        Ok(serde_json::to_value(items)?)
    }

    pub(super) async fn cancel_migration(c: &KubeClient, ns: &str, migration: &str) -> Result<()> {
        let patch = serde_json::json!({ "spec": { "cancel": true } });
        migrations(c, ns)
            .patch(migration, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "kairon")]
    #[test]
    fn defaults_to_kairon_and_never_falls_back() {
        for v in [
            None,
            Some(""),
            Some("auto"),
            Some(" Kairon "),
            Some("bogus"),
        ] {
            assert_eq!(parse_backend(v), BackendKind::Kairon, "{v:?}");
        }
        assert_eq!(BackendKind::Kairon.as_str(), "kairon");
    }

    #[test]
    fn kubevirt_only_when_explicit() {
        assert_eq!(parse_backend(Some("KubeVirt")), BackendKind::KubeVirt);
    }

    #[cfg(feature = "kairon")]
    fn running(name: &str) -> crate::kairon::Machine {
        use crate::kairon::{Machine, MachineSpec, MachineStatus};
        let mut m = Machine::new(name, MachineSpec::default());
        m.metadata.namespace = Some("prod".into());
        m.status = Some(MachineStatus {
            phase: Some("Running".into()),
            runtime_id: Some("rt-1".into()),
            ..Default::default()
        });
        m
    }

    #[cfg(feature = "kairon")]
    #[test]
    fn stop_halts_realized_machines_to_keep_their_disk() {
        let mut m = running("web");
        assert_eq!(kairon::stop_state(&m), crate::kairon::POWER_HALTED);
        m.status = None;
        assert_eq!(kairon::stop_state(&m), crate::kairon::POWER_STOPPED);
    }

    #[cfg(feature = "kairon")]
    #[test]
    fn clone_refuses_pvc_machines_and_resets_identity() {
        let mut m = running("web");
        m.spec.node_name = Some("n1".into());
        let c = kairon::clone_machine(&m, "web-2").unwrap();
        assert_eq!(c.name(), "web-2");
        assert_eq!(c.namespace(), "prod");
        assert!(c.spec.node_name.is_none() && c.status.is_none());
        m.spec.volumes.push(crate::kairon::MachineVolume {
            name: "root".into(),
            claim_name: "web-root".into(),
            ..Default::default()
        });
        assert!(kairon::clone_machine(&m, "web-3").is_err());
    }

    #[cfg(feature = "kairon")]
    #[test]
    fn migration_blocks_stopped_and_passthrough_machines() {
        let m = running("gpu");
        let req = MigrateRequest {
            target_node: Some("n2".into()),
            ..Default::default()
        };
        let mig = kairon::migration_for(&m, &req).unwrap();
        assert_eq!(mig.spec.machine_name, "gpu");
        assert_eq!(mig.spec.target_node.as_deref(), Some("n2"));
        assert_eq!(mig.spec.strategy.as_deref(), Some("auto"));

        let mut gpu = running("gpu");
        gpu.spec
            .device_claims
            .push(crate::kairon::DeviceClaimReference {
                name: "a100".into(),
            });
        assert!(kairon::migration_for(&gpu, &req).is_err());
        let cold = MigrateRequest {
            strategy: Some("cold".into()),
            ..Default::default()
        };
        assert!(kairon::migration_for(&gpu, &cold).is_ok());

        let mut off = running("off");
        off.status.as_mut().unwrap().phase = Some("Halted".into());
        assert!(kairon::migration_for(&off, &MigrateRequest::default()).is_err());
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
