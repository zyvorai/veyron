// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! RDP NodePort / LoadBalancer exposure for Windows KubeVirt VMs.
//!
//! Selects **virt-launcher** pods via `kubevirt.io/vm=<vm>` (not `kubevirt.io/domain` used for SSH expose).

use anyhow::{Result, anyhow};
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::Client;
use kube::api::{Api, DeleteParams, ListParams, PostParams};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

use super::windows_rdp::{WINDOWS_RDP_PORT, vmrogue_rdp_service_name};

/// Within Kubernetes NodePort range 30000–32767.
pub const RDP_NODEPORT_RANGE_START: i32 = 30_100;
pub const RDP_NODEPORT_RANGE_END: i32 = 30_199;

const LABEL_RDP_EXPOSE: &str = "vmrogue.io/rdp-expose";
const LABEL_VM_NAME: &str = "vmrogue.io/vm-name";
const ANN_VM_NAME: &str = "vmrogue.io/vm-name";
const ANN_VM_NS: &str = "vmrogue.io/vm-namespace";

#[derive(Debug, Clone, Serialize)]
pub struct RdpNodePortInUse {
    pub node_port: i32,
    pub vm_name: String,
    pub namespace: String,
    pub service_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RdpExposePortRow {
    pub name: String,
    pub service_port: i32,
    pub target_port: i32,
    pub node_port: Option<i32>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RdpAccessStatus {
    pub guest_ip: Option<String>,
    pub is_windows_vm: bool,
    pub exposed: bool,
    pub node_port: Option<i32>,
    pub cluster_ip: Option<String>,
    pub service_name: String,
    pub service_type: Option<String>,
    pub expose_ports: Vec<RdpExposePortRow>,
    pub rdp_via_nodeport_example: Option<String>,
    pub vm_spec_has_rdp_port: bool,
    pub used_node_ports: Vec<RdpNodePortInUse>,
    pub suggested_node_port: Option<i32>,
    pub nodes: Vec<String>,
}

pub fn suggest_rdp_node_port(used: &HashSet<i32>) -> Option<i32> {
    for p in RDP_NODEPORT_RANGE_START..=RDP_NODEPORT_RANGE_END {
        if !used.contains(&p) {
            return Some(p);
        }
    }
    for p in 30_000..=32_767 {
        if !used.contains(&p) {
            return Some(p);
        }
    }
    None
}

/// Reject NodePorts outside the cluster-valid range.
pub fn validate_node_port(np: i32) -> Result<()> {
    if (30_000..=32_767).contains(&np) {
        Ok(())
    } else {
        Err(anyhow!(
            "node_port must be between 30000 and 32767 (got {np})"
        ))
    }
}

pub fn rdp_node_port_from_service(svc: &Service) -> Option<i32> {
    svc.spec
        .as_ref()?
        .ports
        .as_ref()?
        .iter()
        .find_map(|p| p.node_port)
}

pub async fn list_rdp_nodeport_entries(client: Client) -> Result<Vec<RdpNodePortInUse>> {
    let svc_api: Api<Service> = Api::all(client);
    let list = svc_api
        .list(&ListParams::default().labels(&format!("{LABEL_RDP_EXPOSE}=true")))
        .await?;
    let mut out = Vec::new();
    for svc in list.items {
        let ns = svc.metadata.namespace.clone().unwrap_or_default();
        let vm_name = svc
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get(ANN_VM_NAME))
            .cloned()
            .or_else(|| {
                svc.metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get(LABEL_VM_NAME))
                    .cloned()
            });
        let Some(vmn) = vm_name else { continue };
        let Some(np) = rdp_node_port_from_service(&svc) else {
            continue;
        };
        out.push(RdpNodePortInUse {
            node_port: np,
            vm_name: vmn,
            namespace: ns,
            service_name: svc.metadata.name.clone(),
        });
    }
    out.sort_by_key(|e| e.node_port);
    Ok(out)
}

fn service_to_port_rows(svc: &Service) -> Vec<RdpExposePortRow> {
    let Some(ports) = svc.spec.as_ref().and_then(|s| s.ports.as_ref()) else {
        return Vec::new();
    };
    ports
        .iter()
        .map(|p| {
            let target = match &p.target_port {
                Some(IntOrString::Int(i)) => *i,
                Some(IntOrString::String(s)) => s.parse().unwrap_or(WINDOWS_RDP_PORT),
                None => p.port,
            };
            RdpExposePortRow {
                name: p.name.clone().unwrap_or_else(|| "rdp".to_string()),
                service_port: p.port,
                target_port: target,
                node_port: p.node_port,
                protocol: p.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
            }
        })
        .collect()
}

fn rdp_svc_owns_vm(svc: &Service, vm_name: &str) -> bool {
    svc.metadata
        .labels
        .as_ref()
        .and_then(|l| l.get(LABEL_VM_NAME))
        .map(|v| v == vm_name)
        .unwrap_or(false)
}

pub async fn get_rdp_expose_service(
    client: Client,
    namespace: &str,
    vm_name: &str,
) -> Result<Option<Service>> {
    let svc_name = vmrogue_rdp_service_name(vm_name);
    let svc_api: Api<Service> = Api::namespaced(client, namespace);
    match svc_api.get(&svc_name).await {
        Ok(s) if rdp_svc_owns_vm(&s, vm_name) => Ok(Some(s)),
        Ok(_) => Ok(None),
        Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(None),
        Err(e) => Err(e.into()),
    }
}

async fn validate_node_ports_available(
    client: Client,
    namespace: &str,
    vm_name: &str,
    node_port: i32,
    existing: Option<&Service>,
) -> Result<()> {
    validate_node_port(node_port)?;
    let mut own = HashSet::new();
    if let Some(svc) = existing {
        if let Some(np) = rdp_node_port_from_service(svc) {
            own.insert(np);
        }
    }
    if own.contains(&node_port) {
        return Ok(());
    }
    let entries = list_rdp_nodeport_entries(client).await?;
    if let Some(conflict) = entries
        .iter()
        .find(|e| e.node_port == node_port && !(e.namespace == namespace && e.vm_name == vm_name))
    {
        return Err(anyhow!(
            "NodePort {} is already used by VM {}/{} (Service {}). Choose a different port.",
            node_port,
            conflict.namespace,
            conflict.vm_name,
            conflict.service_name.as_deref().unwrap_or("?")
        ));
    }
    Ok(())
}

pub async fn upsert_rdp_expose_service(
    client: Client,
    namespace: &str,
    vm_name: &str,
    service_type: &str,
    node_port: i32,
) -> Result<()> {
    let svc_name = vmrogue_rdp_service_name(vm_name);
    let svc_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let existing = match svc_api.get(&svc_name).await {
        Ok(s) => {
            if !rdp_svc_owns_vm(&s, vm_name) {
                return Err(anyhow!(
                    "Service {svc_name} exists but is not the RDP exposure for this VM"
                ));
            }
            Some(s)
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => None,
        Err(e) => return Err(e.into()),
    };

    if service_type == "NodePort" || service_type == "LoadBalancer" {
        validate_node_ports_available(
            client.clone(),
            namespace,
            vm_name,
            node_port,
            existing.as_ref(),
        )
        .await?;
    }

    let mut port = ServicePort::default();
    port.name = Some("rdp".to_string());
    port.port = WINDOWS_RDP_PORT;
    port.target_port = Some(IntOrString::Int(WINDOWS_RDP_PORT));
    port.protocol = Some("TCP".to_string());
    if service_type == "NodePort" || service_type == "LoadBalancer" {
        port.node_port = Some(node_port);
    }

    let mut labels = BTreeMap::new();
    labels.insert("vmrogue.io/managed-by".to_string(), "vmrogue".to_string());
    labels.insert(LABEL_RDP_EXPOSE.to_string(), "true".to_string());
    labels.insert(LABEL_VM_NAME.to_string(), vm_name.to_string());

    let mut annotations = BTreeMap::new();
    annotations.insert(ANN_VM_NAME.to_string(), vm_name.to_string());
    annotations.insert(ANN_VM_NS.to_string(), namespace.to_string());

    let mut selector = BTreeMap::new();
    selector.insert("kubevirt.io/vm".to_string(), vm_name.to_string());

    let svc = Service {
        metadata: ObjectMeta {
            name: Some(svc_name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            type_: Some(service_type.to_string()),
            selector: Some(selector),
            ports: Some(vec![port]),
            ..Default::default()
        }),
        ..Default::default()
    };

    match existing {
        Some(prev) => {
            let mut desired = svc;
            desired.metadata.resource_version = prev.metadata.resource_version;
            svc_api
                .replace(&svc_name, &PostParams::default(), &desired)
                .await?;
        }
        None => {
            svc_api.create(&PostParams::default(), &svc).await?;
        }
    }
    Ok(())
}

pub async fn delete_rdp_expose_service(
    client: Client,
    namespace: &str,
    vm_name: &str,
) -> Result<()> {
    let svc_name = vmrogue_rdp_service_name(vm_name);
    let svc_api: Api<Service> = Api::namespaced(client, namespace);
    match svc_api.get(&svc_name).await {
        Ok(s) if rdp_svc_owns_vm(&s, vm_name) => {
            svc_api.delete(&svc_name, &DeleteParams::default()).await?;
        }
        Ok(_) => {}
        Err(kube::Error::Api(ae)) if ae.code == 404 => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

pub async fn assemble_rdp_access(
    client: Client,
    namespace: &str,
    vm_name: &str,
    guest_ip: Option<String>,
    is_windows: bool,
    spec_has_rdp_port: bool,
    node_ips: Vec<String>,
) -> Result<RdpAccessStatus> {
    let service_name = vmrogue_rdp_service_name(vm_name);
    let used_entries = list_rdp_nodeport_entries(client.clone())
        .await
        .unwrap_or_default();
    let used_set: HashSet<i32> = used_entries.iter().map(|e| e.node_port).collect();
    let used_node_ports: Vec<RdpNodePortInUse> = used_entries
        .into_iter()
        .filter(|e| !(e.namespace == namespace && e.vm_name == vm_name))
        .collect();
    let suggested_node_port = suggest_rdp_node_port(&used_set);

    let resolved = get_rdp_expose_service(client, namespace, vm_name).await?;

    let (exposed, node_port, cluster_ip, service_type, expose_ports, rdp_via_nodeport_example) =
        match resolved {
            Some(svc) => {
                let rows = service_to_port_rows(&svc);
                let np = rdp_node_port_from_service(&svc);
                let cip = svc
                    .spec
                    .as_ref()
                    .and_then(|s| s.cluster_ip.as_ref())
                    .filter(|c| !c.is_empty() && *c != "None")
                    .cloned();
                let st = svc.spec.as_ref().and_then(|s| s.type_.clone());
                let example = match (np, node_ips.first()) {
                    (Some(port), Some(ip)) => Some(format!("{ip}:{port}")),
                    _ => None,
                };
                (true, np, cip, st, rows, example)
            }
            None => (false, None, None, None, Vec::new(), None),
        };

    Ok(RdpAccessStatus {
        guest_ip,
        is_windows_vm: is_windows,
        exposed,
        node_port,
        cluster_ip,
        service_name,
        service_type,
        expose_ports,
        rdp_via_nodeport_example,
        vm_spec_has_rdp_port: spec_has_rdp_port,
        used_node_ports,
        suggested_node_port,
        nodes: node_ips,
    })
}
