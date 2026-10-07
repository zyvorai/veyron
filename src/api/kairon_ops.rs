// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Kairon implementations of the VM day-2 routes that have no KubeVirt-shaped
//! equivalent in [`crate::api::vm_backend`]: snapshots, guest access and expose.
//!
//! Snapshots come in two kinds, picked per Machine:
//! - **disk**: Machines with PVC volumes get a `MachineSnapshot` (CSI). Kubernetes
//!   cannot roll a bound PVC back, so restore creates new claims and a new Machine
//!   on them, leaving the original untouched.
//! - **vm-state**: image-booted Machines have no PVC to snapshot, so Veyron takes a
//!   FluxVM checkpoint (RAM, CPU and disk state) through the kairon-node relay and
//!   restores it in place. FluxVM cannot list or delete checkpoints, so Veyron records
//!   each one in a ConfigMap labelled `veyron.io/type=kairon-vm-checkpoint`.

use crate::kairon::relay::{ExecRequest, RelayTarget};
use crate::kairon::{
    LABEL_MANAGED_BY, Machine, MachineBackup, MachineBackupRestore, MachineBackupRestoreSpec,
    MachineBackupSpec, MachineSnapshot, MachineSnapshotRestore, MachineSnapshotRestoreSpec,
    MachineSnapshotSpec, POWER_RUNNING,
};
use crate::kube::KubeClient;
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::Api;
use kube::api::{DeleteParams, ListParams, PostParams};
use serde::Serialize;
use std::collections::BTreeMap;

pub const CHECKPOINT_TYPE: &str = "kairon-vm-checkpoint";
const LABEL_TYPE: &str = "veyron.io/type";
const LABEL_VM: &str = "veyron.io/vm";
const CHECKPOINT_PREFIX: &str = "veyron-ckpt-";

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRecord {
    pub name: String,
    pub vm_name: String,
    pub namespace: String,
    pub status: String,
    pub ready: bool,
    pub age: String,
    /// `disk` (MachineSnapshot) or `vm-state` (FluxVM checkpoint).
    pub kind: &'static str,
}

fn age_since(ts: Option<DateTime<Utc>>) -> String {
    let Some(created) = ts else {
        return "Unknown".to_string();
    };
    let d = Utc::now().signed_duration_since(created);
    if d.num_days() > 0 {
        format!("{}d", d.num_days())
    } else if d.num_hours() > 0 {
        format!("{}h", d.num_hours())
    } else {
        format!("{}m", d.num_minutes().max(0))
    }
}

fn snapshots(c: &KubeClient, ns: Option<&str>) -> Api<MachineSnapshot> {
    match ns {
        Some(ns) => Api::namespaced(c.client(), ns),
        None => Api::all(c.client()),
    }
}

fn configmaps(c: &KubeClient, ns: Option<&str>) -> Api<ConfigMap> {
    match ns {
        Some(ns) => Api::namespaced(c.client(), ns),
        None => Api::all(c.client()),
    }
}

fn disk_record(s: &MachineSnapshot) -> SnapshotRecord {
    let st = s.status.as_ref();
    let ready = st.is_some_and(|s| s.ready_to_use);
    SnapshotRecord {
        name: s.metadata.name.clone().unwrap_or_default(),
        vm_name: s.spec.machine_name.clone(),
        namespace: s.metadata.namespace.clone().unwrap_or_default(),
        status: st
            .and_then(|s| s.phase.clone())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| if ready { "Ready" } else { "Pending" }.to_string()),
        ready,
        age: age_since(s.metadata.creation_timestamp.as_ref().map(|t| t.0)),
        kind: "disk",
    }
}

fn checkpoint_record(cm: &ConfigMap) -> Option<SnapshotRecord> {
    let data = cm.data.as_ref()?;
    Some(SnapshotRecord {
        name: data.get("tag")?.clone(),
        vm_name: data.get("machine")?.clone(),
        namespace: cm.metadata.namespace.clone().unwrap_or_default(),
        status: "Ready".to_string(),
        ready: true,
        age: age_since(cm.metadata.creation_timestamp.as_ref().map(|t| t.0)),
        kind: "vm-state",
    })
}

/// Snapshots in `scope_ns` (`"all"` for every namespace), optionally for one VM.
pub async fn list_snapshots(
    c: &KubeClient,
    scope_ns: &str,
    vm: Option<&str>,
) -> Result<Vec<SnapshotRecord>> {
    let ns = (scope_ns != "all").then_some(scope_ns);
    let mut out: Vec<SnapshotRecord> = snapshots(c, ns)
        .list(&ListParams::default())
        .await?
        .items
        .iter()
        .map(disk_record)
        .collect();
    let lp = ListParams::default().labels(&format!("{LABEL_TYPE}={CHECKPOINT_TYPE}"));
    out.extend(
        configmaps(c, ns)
            .list(&lp)
            .await?
            .items
            .iter()
            .filter_map(checkpoint_record),
    );
    if let Some(vm) = vm {
        out.retain(|r| r.vm_name == vm);
    }
    Ok(out)
}

pub fn new_snapshot(m: &Machine, name: &str) -> MachineSnapshot {
    let mut s = MachineSnapshot::new(
        name,
        MachineSnapshotSpec {
            machine_name: m.name().to_string(),
            ..Default::default()
        },
    );
    s.metadata.namespace = Some(m.namespace().to_string());
    s.metadata
        .labels
        .get_or_insert_with(Default::default)
        .insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
    s
}

pub fn checkpoint_configmap(m: &Machine, tag: &str) -> ConfigMap {
    let mut cm = ConfigMap::default();
    cm.metadata.name = Some(format!("{CHECKPOINT_PREFIX}{tag}"));
    cm.metadata.namespace = Some(m.namespace().to_string());
    cm.metadata.labels = Some(BTreeMap::from([
        (LABEL_TYPE.to_string(), CHECKPOINT_TYPE.to_string()),
        (LABEL_VM.to_string(), m.name().to_string()),
        (LABEL_MANAGED_BY.to_string(), "veyron".to_string()),
    ]));
    cm.data = Some(BTreeMap::from([
        ("machine".to_string(), m.name().to_string()),
        ("tag".to_string(), tag.to_string()),
        ("created".to_string(), Utc::now().to_rfc3339()),
    ]));
    cm
}

pub async fn create_snapshot(
    c: &KubeClient,
    ns: &str,
    vm: &str,
    name: &str,
) -> Result<SnapshotRecord> {
    let m = Api::<Machine>::namespaced(c.client(), ns).get(vm).await?;
    if !m.spec.volumes.is_empty() {
        let created = snapshots(c, Some(ns))
            .create(&PostParams::default(), &new_snapshot(&m, name))
            .await
            .with_context(|| format!("creating MachineSnapshot {ns}/{name}"))?;
        return Ok(disk_record(&created));
    }
    if !matches!(m.phase(), "Running" | "Paused") {
        bail!(
            "Machine '{vm}' must be Running or Paused for a VM-state snapshot (phase {}) (conflict)",
            m.phase()
        );
    }
    RelayTarget::for_machine(&c.client(), &m)
        .await?
        .vm_snapshot(name)
        .await?;
    let cm = configmaps(c, Some(ns))
        .create(&PostParams::default(), &checkpoint_configmap(&m, name))
        .await
        .context("recording the VM-state snapshot")?;
    checkpoint_record(&cm).ok_or_else(|| anyhow!("checkpoint record is malformed"))
}

/// Returns a note for the caller when part of the snapshot outlives the delete.
pub async fn delete_snapshot(c: &KubeClient, ns: &str, name: &str) -> Result<Option<String>> {
    let cm_name = format!("{CHECKPOINT_PREFIX}{name}");
    if configmaps(c, Some(ns)).get_opt(&cm_name).await?.is_some() {
        configmaps(c, Some(ns))
            .delete(&cm_name, &DeleteParams::default())
            .await?;
        return Ok(Some(
            "FluxVM cannot delete VM-state checkpoints; the record is gone but its data stays on the node until the Machine is deleted".to_string(),
        ));
    }
    snapshots(c, Some(ns))
        .delete(name, &DeleteParams::default())
        .await?;
    Ok(None)
}

/// New Machine booting from restored claims. `restored` maps each snapshotted
/// volume name to its new claim; volumes and data disks the snapshot does not cover
/// are dropped so the copy never shares a live PVC with the original.
pub fn restored_machine(
    src: &Machine,
    new_name: &str,
    restored: &BTreeMap<String, String>,
) -> Result<Machine> {
    let boot = src
        .spec
        .volumes
        .first()
        .ok_or_else(|| anyhow!("Machine '{}' has no volumes", src.name()))?;
    if !restored.contains_key(&boot.name) {
        bail!(
            "snapshot does not cover boot volume '{}' of Machine '{}'",
            boot.name,
            src.name()
        );
    }
    let mut spec = src.spec.clone();
    spec.node_name = None;
    spec.power_state = Some(POWER_RUNNING.to_string());
    spec.disks.clear();
    spec.volumes.retain(|v| restored.contains_key(&v.name));
    for v in &mut spec.volumes {
        v.claim_name = restored[&v.name].clone();
        v.atlas = None;
    }
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
    labels.insert(
        "veyron.io/restored-from".to_string(),
        src.name().to_string(),
    );
    Ok(m)
}

pub async fn restore_snapshot(c: &KubeClient, ns: &str, name: &str) -> Result<serde_json::Value> {
    let machines = Api::<Machine>::namespaced(c.client(), ns);
    if let Some(cm) = configmaps(c, Some(ns))
        .get_opt(&format!("{CHECKPOINT_PREFIX}{name}"))
        .await?
    {
        let rec =
            checkpoint_record(&cm).ok_or_else(|| anyhow!("checkpoint record is malformed"))?;
        let m = machines.get(&rec.vm_name).await?;
        RelayTarget::for_machine(&c.client(), &m)
            .await?
            .vm_restore_snapshot(name)
            .await?;
        return Ok(serde_json::json!({
            "name": name,
            "vm_name": rec.vm_name,
            "kind": "vm-state",
            "message": format!("VM '{}' restarted from snapshot '{name}'", rec.vm_name),
        }));
    }

    let snap = snapshots(c, Some(ns)).get(name).await?;
    let status = snap.status.clone().unwrap_or_default();
    if !status.ready_to_use {
        bail!("snapshot '{name}' is not ready to use yet (conflict)");
    }
    let src = machines
        .get(&snap.spec.machine_name)
        .await
        .with_context(|| {
            format!(
                "Machine '{}' is gone; restore its claims with a MachineSnapshotRestore",
                snap.spec.machine_name
            )
        })?;
    let stamp = Utc::now().format("%m%d%H%M%S");
    let new_name = format!("{}-restored-{stamp}", src.name());
    let restores = Api::<MachineSnapshotRestore>::namespaced(c.client(), ns);
    let mut claims = BTreeMap::new();
    for vs in &status.volume_snapshots {
        let claim = format!("{new_name}-{}", vs.volume_name);
        let mut r = MachineSnapshotRestore::new(
            &format!("{name}-{}-{stamp}", vs.volume_name),
            MachineSnapshotRestoreSpec {
                snapshot_name: name.to_string(),
                volume_name: Some(vs.volume_name.clone()),
                target_claim_name: claim.clone(),
                storage_class_name: None,
                storage_size: None,
            },
        );
        r.metadata.namespace = Some(ns.to_string());
        restores
            .create(&PostParams::default(), &r)
            .await
            .with_context(|| format!("restoring volume {}", vs.volume_name))?;
        claims.insert(vs.volume_name.clone(), claim);
    }
    let m = restored_machine(&src, &new_name, &claims)?;
    machines
        .create(&PostParams::default(), &m)
        .await
        .with_context(|| format!("creating Machine {ns}/{new_name}"))?;
    Ok(serde_json::json!({
        "name": name,
        "vm_name": src.name(),
        "kind": "disk",
        "restored_vm": new_name,
        "claims": claims,
        "message": format!(
            "Snapshot '{name}' restored into new VM '{new_name}'; '{}' is unchanged",
            src.name()
        ),
    }))
}

// ---------------------------------------------------------------------------
// Guest access (kairon-node relay → QEMU guest agent)
// ---------------------------------------------------------------------------

async fn relay_for(c: &KubeClient, ns: &str, name: &str) -> Result<(Machine, RelayTarget)> {
    let m = Api::<Machine>::namespaced(c.client(), ns).get(name).await?;
    let target = RelayTarget::for_machine(&c.client(), &m).await?;
    Ok((m, target))
}

/// Accepts the GuestKit/QGA body (`path` + `arg`), `args`, a `command` string run
/// through `/bin/sh -c`, or `powershell` for Windows guests.
pub fn exec_request(params: &serde_json::Value) -> Result<ExecRequest> {
    let timeout = params
        .get("timeout_seconds")
        .or_else(|| params.get("timeoutSeconds"))
        .and_then(|v| v.as_u64())
        .unwrap_or(60);
    if let Some(ps) = params.get("powershell").and_then(|v| v.as_str()) {
        return Ok(ExecRequest::powershell(ps, timeout));
    }
    if let Some(cmd) = params.get("command").and_then(|v| v.as_str()) {
        return Ok(ExecRequest::shell(cmd, timeout));
    }
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("guest exec needs `path`, `command` or `powershell`"))?;
    let args = params
        .get("arg")
        .or_else(|| params.get("args"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(ExecRequest {
        path: Some(path.to_string()),
        args,
        powershell: None,
        timeout_seconds: Some(timeout),
    })
}

pub async fn guest_exec(
    c: &KubeClient,
    ns: &str,
    name: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let req = exec_request(params)?;
    let (_, relay) = relay_for(c, ns, name).await?;
    let r = relay.exec(&req).await?;
    Ok(serde_json::json!({
        "exit_code": r.exit_code,
        "exitcode": r.exit_code,
        "stdout": r.stdout,
        "stderr": r.stderr,
    }))
}

/// Turns Windows Remote Desktop on or off inside a running guest through the relay.
pub async fn set_windows_rdp(
    c: &KubeClient,
    ns: &str,
    name: &str,
    enable: bool,
) -> Result<crate::kube::windows_rdp::RdpGuestAgentResponse> {
    use crate::kube::windows_rdp::{WINDOWS_DISABLE_RDP_PS1_INLINE, WINDOWS_ENABLE_RDP_PS1_INLINE};
    let script = if enable {
        WINDOWS_ENABLE_RDP_PS1_INLINE
    } else {
        WINDOWS_DISABLE_RDP_PS1_INLINE
    };
    let out = guest_exec(c, ns, name, &serde_json::json!({ "powershell": script })).await?;
    let exit_code = out.get("exit_code").and_then(|v| v.as_i64());
    let success = exit_code == Some(0);
    let verb = if enable { "enabled" } else { "disabled" };
    Ok(crate::kube::windows_rdp::RdpGuestAgentResponse {
        success,
        guest_agent_connected: true,
        is_windows_vm: true,
        message: if success {
            format!("Remote Desktop {verb} in the guest")
        } else {
            format!("PowerShell exited with {exit_code:?}; Remote Desktop may not be {verb}")
        },
        exit_code,
        stdout: out
            .get("stdout")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        stderr: out
            .get("stderr")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        guest_exec: out,
    })
}

/// Probes the QEMU guest agent with a harmless `guest-fsfreeze-status`.
pub async fn guest_status(c: &KubeClient, ns: &str, name: &str) -> Result<serde_json::Value> {
    let m = Api::<Machine>::namespaced(c.client(), ns).get(name).await?;
    let enabled = m.spec.guest_agent.as_ref().is_some_and(|g| g.enabled);
    let (connected, reason) = if !m.is_running() {
        (false, Some(format!("Machine is {}", m.phase())))
    } else if !enabled {
        (false, Some("spec.guestAgent.enabled is false".to_string()))
    } else {
        match RelayTarget::for_machine(&c.client(), &m).await {
            Ok(relay) => match relay.fsfreeze_status().await {
                Ok(_) => (true, None),
                Err(e) => (false, Some(format!("{e:#}"))),
            },
            Err(e) => (false, Some(format!("{e:#}"))),
        }
    };
    Ok(serde_json::json!({
        "connected": connected,
        "runtime_kind": "qga",
        "guest_agent_connected": connected,
        "version": serde_json::Value::Null,
        "capabilities": serde_json::Value::Null,
        "reason": reason,
    }))
}

/// FluxVM/QEMU runtime logs for the Machine.
pub async fn runtime_logs(c: &KubeClient, ns: &str, name: &str, tail: u32) -> Result<String> {
    let (_, relay) = relay_for(c, ns, name).await?;
    relay.logs(&format!("tail={tail}")).await
}

// ---------------------------------------------------------------------------
// Expose (SSH / RDP)
// ---------------------------------------------------------------------------

/// How a Machine's guest port is reachable from outside.
#[derive(Debug, Clone, PartialEq)]
pub enum ExposeTarget {
    /// A creation-time `spec.network.forwards` entry: `<node IP>:<host_port>`.
    Forward { node: String, host_port: u16 },
    /// A routable guest address a selector-less Service can point at.
    GuestIp(String),
}

/// `user`-mode guests sit behind FluxVM's NAT on a link-local address nothing else
/// can reach, so only a creation-time forward exposes them.
/// Whether a Service endpoint on the guest IP can reach the VM: only bridged (tap/macvtap)
/// guests with a non-link-local address. User-mode networking (the default) hands out
/// QEMU's private `10.0.2.0/24`, which exists only behind FluxVM's NAT.
fn routable_guest_ip(m: &Machine, ip: &str) -> bool {
    let bridged = m
        .spec
        .network
        .as_ref()
        .and_then(|n| n.mode.as_deref())
        .is_some_and(|mode| matches!(mode, "tap" | "macvtap"));
    let parsed = ip.parse::<std::net::IpAddr>();
    let usable = match parsed {
        Ok(std::net::IpAddr::V4(v4)) => {
            !v4.is_link_local() && !v4.is_loopback() && !v4.is_unspecified()
        }
        Ok(std::net::IpAddr::V6(v6)) => !v6.is_loopback() && (v6.segments()[0] & 0xffc0) != 0xfe80,
        Err(_) => false,
    };
    bridged && usable
}

pub fn expose_target(m: &Machine, guest_port: u16) -> Result<ExposeTarget> {
    if let Some(f) = m
        .spec
        .network
        .as_ref()
        .and_then(|n| n.forwards.iter().find(|f| f.guest_port == guest_port))
    {
        return Ok(ExposeTarget::Forward {
            node: m.node().unwrap_or_default().to_string(),
            host_port: f.host_port,
        });
    }
    match m.guest_ip() {
        Some(ip) if routable_guest_ip(m, &ip) => Ok(ExposeTarget::GuestIp(ip)),
        Some(ip) => bail!(
            "Machine '{}' uses user-mode networking (guest {ip} is only reachable through FluxVM's NAT) and has no forward for port {guest_port}; Kairon applies port forwards only at creation, so recreate it with the forward or use tap/macvtap networking (conflict)",
            m.name()
        ),
        None => bail!(
            "Machine '{}' has no guest IP yet (phase {}) (conflict)",
            m.name(),
            m.phase()
        ),
    }
}

pub async fn machine_expose_target(
    c: &KubeClient,
    ns: &str,
    name: &str,
    guest_port: u16,
) -> Result<ExposeTarget> {
    let m = Api::<Machine>::namespaced(c.client(), ns).get(name).await?;
    expose_target(&m, guest_port)
}

/// Drop the Service's pod selector and back it with an EndpointSlice on the guest IP.
/// The slice is owned by the Service, so deleting the Service removes it.
pub async fn bind_service_to_guest(
    c: &KubeClient,
    ns: &str,
    service: &str,
    guest_ip: &str,
) -> Result<()> {
    use k8s_openapi::api::core::v1::Service;
    use k8s_openapi::api::discovery::v1::{Endpoint, EndpointPort, EndpointSlice};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
    use kube::api::{Patch, PatchParams};

    let services = Api::<Service>::namespaced(c.client(), ns);
    let svc = services
        .patch(
            service,
            &PatchParams::default(),
            &Patch::Merge(serde_json::json!({ "spec": { "selector": null } })),
        )
        .await
        .with_context(|| format!("clearing selector on Service {ns}/{service}"))?;
    let ports = svc
        .spec
        .as_ref()
        .and_then(|s| s.ports.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|p| EndpointPort {
            name: p.name,
            port: match p.target_port {
                Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(n)) => Some(n),
                _ => Some(p.port),
            },
            protocol: p.protocol,
            app_protocol: None,
        })
        .collect();
    let slice = EndpointSlice {
        metadata: kube::api::ObjectMeta {
            name: Some(format!("{service}-guest")),
            namespace: Some(ns.to_string()),
            labels: Some(BTreeMap::from([
                (
                    "kubernetes.io/service-name".to_string(),
                    service.to_string(),
                ),
                (
                    "endpointslice.kubernetes.io/managed-by".to_string(),
                    "veyron.io".to_string(),
                ),
            ])),
            owner_references: Some(vec![OwnerReference {
                api_version: "v1".to_string(),
                kind: "Service".to_string(),
                name: service.to_string(),
                uid: svc.metadata.uid.clone().unwrap_or_default(),
                ..Default::default()
            }]),
            ..Default::default()
        },
        address_type: if guest_ip.contains(':') {
            "IPv6"
        } else {
            "IPv4"
        }
        .to_string(),
        endpoints: vec![Endpoint {
            addresses: vec![guest_ip.to_string()],
            ..Default::default()
        }],
        ports: Some(ports),
    };
    let slices = Api::<EndpointSlice>::namespaced(c.client(), ns);
    let pp = PatchParams::apply("veyron").force();
    slices
        .patch(&format!("{service}-guest"), &pp, &Patch::Apply(&slice))
        .await
        .with_context(|| format!("writing EndpointSlice for Service {ns}/{service}"))?;
    Ok(())
}

pub fn forward_json(vm: &str, guest_port: u16, node: &str, host_port: u16) -> serde_json::Value {
    serde_json::json!({
        "enabled": true,
        "service_name": serde_json::Value::Null,
        "service_type": "HostPort",
        "node": node,
        "ports": [{
            "name": if guest_port == 3389 { "rdp" } else { "ssh" },
            "port": host_port,
            "target_port": guest_port,
            "node_port": host_port,
            "protocol": "TCP",
        }],
        "note": format!("'{vm}' forwards node port {host_port} to guest port {guest_port} (set when the Machine was created)"),
    })
}

// ---------------------------------------------------------------------------
// Backups (MachineBackup / MachineBackupRestore)
// ---------------------------------------------------------------------------

fn backups(c: &KubeClient, ns: Option<&str>) -> Api<MachineBackup> {
    match ns {
        Some(ns) => Api::namespaced(c.client(), ns),
        None => Api::all(c.client()),
    }
}

/// Same shape as `handlers::backups::BackupResponse`.
pub fn backup_json(b: &MachineBackup) -> serde_json::Value {
    let st = b.status.as_ref();
    let phase = st
        .and_then(|s| s.phase.clone())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "Pending".to_string());
    let completed = st
        .and_then(|s| s.extra.get("completionTime"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    serde_json::json!({
        "id": b.metadata.name,
        "name": b.metadata.name,
        "vm_name": b.spec.machine_name,
        "namespace": b.metadata.namespace,
        "backup_type": if b.spec.atlas.is_some() { "atlas" } else { "full" },
        "status": phase,
        "size_bytes": serde_json::Value::Null,
        "compressed": false,
        "encrypted": false,
        "created_at": b.metadata.creation_timestamp.as_ref().map(|t| t.0.to_rfc3339()).unwrap_or_default(),
        "completed_at": completed,
        "message": st.and_then(|s| s.message.clone()),
    })
}

pub async fn list_backups(c: &KubeClient, scope_ns: &str) -> Result<Vec<serde_json::Value>> {
    let ns = (scope_ns != "all").then_some(scope_ns);
    Ok(backups(c, ns)
        .list(&ListParams::default())
        .await?
        .items
        .iter()
        .map(backup_json)
        .collect())
}

pub async fn create_backup(
    c: &KubeClient,
    ns: &str,
    vm: &str,
    name: &str,
) -> Result<serde_json::Value> {
    Api::<Machine>::namespaced(c.client(), ns).get(vm).await?;
    let mut b = MachineBackup::new(
        name,
        MachineBackupSpec {
            machine_name: vm.to_string(),
            quiesce: None,
            atlas: None,
        },
    );
    b.metadata.namespace = Some(ns.to_string());
    b.metadata
        .labels
        .get_or_insert_with(Default::default)
        .insert(LABEL_MANAGED_BY.to_string(), "veyron".to_string());
    let created = backups(c, Some(ns))
        .create(&PostParams::default(), &b)
        .await
        .with_context(|| format!("creating MachineBackup {ns}/{name}"))?;
    Ok(backup_json(&created))
}

/// Kairon restores a backup in place and needs the Machine `Halted` first; a
/// `Stopped` Machine has no runtime left to restore into.
pub async fn restore_backup(c: &KubeClient, ns: &str, name: &str) -> Result<String> {
    let b = backups(c, Some(ns)).get(name).await?;
    let machine = Api::<Machine>::namespaced(c.client(), ns)
        .get(&b.spec.machine_name)
        .await?;
    if machine.phase() != "Halted" {
        bail!(
            "Machine '{}' must be stopped before restoring a backup into it (phase {}) (conflict)",
            machine.name(),
            machine.phase()
        );
    }
    let stamp = Utc::now().format("%m%d%H%M%S");
    let mut r = MachineBackupRestore::new(
        &format!("{name}-restore-{stamp}"),
        MachineBackupRestoreSpec {
            backup_name: name.to_string(),
            machine_name: None,
            storage_class_name: None,
        },
    );
    r.metadata.namespace = Some(ns.to_string());
    let created = Api::<MachineBackupRestore>::namespaced(c.client(), ns)
        .create(&PostParams::default(), &r)
        .await
        .with_context(|| format!("restoring backup {ns}/{name}"))?;
    Ok(created.metadata.name.unwrap_or_default())
}

pub async fn delete_backup(c: &KubeClient, ns: &str, name: &str) -> Result<()> {
    backups(c, Some(ns))
        .delete(name, &DeleteParams::default())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kairon::{MachineDisk, MachineSpec, MachineVolume, NetworkSpec, PortForward};

    #[test]
    fn expose_prefers_forwards_and_refuses_nat_only_guests() {
        let mut m = pvc_machine();
        m.status = Some(crate::kairon::MachineStatus {
            node_name: Some("n1".into()),
            guest_ip: Some("169.254.0.3".into()),
            ..Default::default()
        });
        assert_eq!(
            expose_target(&m, 22).unwrap(),
            ExposeTarget::Forward {
                node: "n1".into(),
                host_port: 2222
            }
        );
        let err = expose_target(&m, 3389).unwrap_err().to_string();
        assert!(err.contains("user-mode") && err.ends_with("(conflict)"));
        m.status.as_mut().unwrap().guest_ip = Some("10.0.2.15".into());
        assert!(
            expose_target(&m, 3389).is_err(),
            "user-mode QEMU NAT address"
        );
        m.spec.network.as_mut().unwrap().mode = Some("tap".into());
        m.status.as_mut().unwrap().guest_ip = Some("169.254.0.3".into());
        assert!(expose_target(&m, 3389).is_err(), "bridged but link-local");
        m.status.as_mut().unwrap().guest_ip = Some("10.10.0.7".into());
        assert_eq!(
            expose_target(&m, 3389).unwrap(),
            ExposeTarget::GuestIp("10.10.0.7".into())
        );
    }

    #[test]
    fn exec_request_accepts_guestkit_and_shell_bodies() {
        let r = exec_request(&serde_json::json!({"path": "/bin/sh", "arg": ["-c", "id"]})).unwrap();
        assert_eq!(r.path.as_deref(), Some("/bin/sh"));
        assert_eq!(r.args, vec!["-c", "id"]);
        let r =
            exec_request(&serde_json::json!({"command": "uptime", "timeout_seconds": 5})).unwrap();
        assert_eq!(r.args, vec!["-c", "uptime"]);
        assert_eq!(r.timeout_seconds, Some(5));
        let r = exec_request(&serde_json::json!({"powershell": "Get-Date"})).unwrap();
        assert_eq!(r.powershell.as_deref(), Some("Get-Date"));
        assert!(exec_request(&serde_json::json!({})).is_err());
    }

    #[test]
    fn backup_maps_to_backup_response_shape() {
        let mut b = MachineBackup::new(
            "nightly",
            MachineBackupSpec {
                machine_name: "web".into(),
                quiesce: None,
                atlas: None,
            },
        );
        b.metadata.namespace = Some("prod".into());
        let v = backup_json(&b);
        assert_eq!(v["id"], "nightly");
        assert_eq!(v["vm_name"], "web");
        assert_eq!(v["status"], "Pending");
        assert_eq!(v["backup_type"], "full");
    }

    fn pvc_machine() -> Machine {
        let mut m = Machine::new(
            "db",
            MachineSpec {
                node_name: Some("n1".into()),
                volumes: vec![
                    MachineVolume {
                        name: "root".into(),
                        claim_name: "db-root".into(),
                        ..Default::default()
                    },
                    MachineVolume {
                        name: "share".into(),
                        claim_name: "db-share".into(),
                        ..Default::default()
                    },
                ],
                disks: vec![MachineDisk {
                    name: "data".into(),
                    claim_name: "db-data".into(),
                    ..Default::default()
                }],
                network: Some(NetworkSpec {
                    mac: Some("52:54:00:00:00:01".into()),
                    forwards: vec![PortForward {
                        host_port: 2222,
                        guest_port: 22,
                        protocol: None,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        m.metadata.namespace = Some("prod".into());
        m
    }

    #[test]
    fn restored_machine_never_shares_source_claims() {
        let claims = BTreeMap::from([("root".to_string(), "db-restored-root".to_string())]);
        let m = restored_machine(&pvc_machine(), "db-restored", &claims).unwrap();
        assert_eq!(m.spec.volumes.len(), 1);
        assert_eq!(m.spec.volumes[0].claim_name, "db-restored-root");
        assert!(m.spec.disks.is_empty());
        assert!(m.spec.node_name.is_none());
        let net = m.spec.network.as_ref().unwrap();
        assert!(net.mac.is_none() && net.forwards.is_empty());
        assert_eq!(m.namespace(), "prod");
    }

    #[test]
    fn restore_requires_boot_volume() {
        let claims = BTreeMap::from([("share".to_string(), "x".to_string())]);
        assert!(restored_machine(&pvc_machine(), "db-restored", &claims).is_err());
    }

    #[test]
    fn checkpoint_record_round_trips() {
        let cm = checkpoint_configmap(&pvc_machine(), "before-upgrade");
        assert_eq!(
            cm.metadata.name.as_deref(),
            Some("veyron-ckpt-before-upgrade")
        );
        let rec = checkpoint_record(&cm).unwrap();
        assert_eq!(rec.vm_name, "db");
        assert_eq!(rec.name, "before-upgrade");
        assert_eq!(rec.kind, "vm-state");
        let snap = new_snapshot(&pvc_machine(), "s1");
        assert_eq!(disk_record(&snap).vm_name, "db");
    }
}
