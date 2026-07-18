// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::vm_rdp;
use crate::kube::windows_rdp::veyron_rdp_service_name;

/// Per-VM network posture: interfaces, expose, RDP, and namespace policies.
pub async fn network_lens(client: &KubeClient, namespace: &str, name: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Network Lens",
        "network_lens",
        format!("Network view: {namespace}/{name}"),
    );

    let vm = match client.get_vm(namespace, name).await {
        Ok(v) => v,
        Err(e) => {
            r.summary = format!("Could not load VirtualMachine: {e}");
            return r;
        }
    };

    let spec = &vm.spec.template.spec;
    let ifaces = spec
        .domain
        .devices
        .as_ref()
        .and_then(|d| d.interfaces.as_ref());
    let networks = spec.networks.as_ref();

    if let Some(ifaces) = ifaces {
        for iface in ifaces {
            let mut kind = "unknown";
            if iface.masquerade.is_some() {
                kind = "masquerade (pod network)";
            } else if iface.bridge.is_some() {
                kind = "bridge";
            } else if iface.sriov.is_some() {
                kind = "sriov";
            }
            let ports = iface
                .ports
                .as_ref()
                .map(|ps| {
                    ps.iter()
                        .map(|p| p.port.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let port_note = if ports.is_empty() {
                String::new()
            } else {
                format!(" ports [{ports}]")
            };
            r.evidence
                .push(format!("Interface {} — {kind}{port_note}", iface.name));
        }
    } else {
        r.evidence
            .push("No interfaces declared on domain.devices".into());
    }

    if let Some(nets) = networks {
        for net in nets {
            if let Some(multus) = &net.multus {
                r.evidence.push(format!(
                    "Network {} — Multus {}",
                    net.name, multus.network_name
                ));
            } else if net.pod.is_some() {
                r.evidence
                    .push(format!("Network {} — pod (default CNI)", net.name));
            }
        }
    }

    let k8s = client.client();
    let np_api: Api<NetworkPolicy> = Api::namespaced(k8s.clone(), namespace);
    let policies = np_api
        .list(&ListParams::default())
        .await
        .map(|l| l.items.len())
        .unwrap_or(0);
    r.evidence.push(format!(
        "Namespace NetworkPolicies: {policies} (Cilium policies may also apply)"
    ));

    match client.get_vm_expose_service(namespace, name).await {
        Ok(Some(svc)) => {
            let st = svc
                .spec
                .as_ref()
                .and_then(|s| s.type_.as_deref())
                .unwrap_or("ClusterIP");
            r.evidence.push(format!(
                "SSH expose Service `{}` — {st}",
                svc.metadata.name.as_deref().unwrap_or("ssh")
            ));
            if st == "NodePort" {
                r.recommendations.push(
                    "SSH NodePort exposes the guest — restrict with firewall or use VPN.".into(),
                );
            }
        }
        Ok(None) => r
            .evidence
            .push("No SSH expose Service (vm.kubevirt.io/name)".into()),
        Err(e) => r.evidence.push(format!("SSH expose lookup: {e}")),
    }

    match vm_rdp::get_rdp_expose_service(k8s.clone(), namespace, name).await {
        Ok(Some(svc)) => {
            let np = vm_rdp::rdp_node_port_from_service(&svc);
            r.evidence.push(format!(
                "RDP expose Service `{}`{}",
                veyron_rdp_service_name(name),
                np.map(|p| format!(" — NodePort {p}")).unwrap_or_default()
            ));
            r.recommendations
                .push("Windows RDP via NodePort should be gateway-only in production.".into());
        }
        Ok(None) => r.evidence.push("No RDP NodePort Service".into()),
        Err(e) => r.evidence.push(format!("RDP expose lookup: {e}")),
    }

    if let Ok(Some(ip)) = client.get_vm_ip(namespace, name).await {
        r.evidence.push(format!("Guest IP (VMI): {ip}"));
    }

    let allow_inet = vm
        .metadata
        .labels
        .as_ref()
        .and_then(|l| l.get("veyron.io/allow-internet"))
        .map(|v| v == "true")
        .unwrap_or(true);
    r.evidence.push(format!(
        "Internet egress policy: {}",
        if allow_inet {
            "allowed (Veyron default)"
        } else {
            "restricted"
        }
    ));

    r.summary = if r.evidence.is_empty() {
        "No network details found.".into()
    } else {
        format!("Mapped {} network signal(s) for {name}.", r.evidence.len())
    };

    if policies == 0 {
        r.recommendations.push(
            "Namespace has no Kubernetes NetworkPolicies — consider default-deny with explicit egress.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
        CopilotAction {
            label: "Network policies".into(),
            action: "navigate".into(),
            page: Some("network-policies".into()),
        },
        CopilotAction {
            label: "Open Ingress".into(),
            action: "navigate".into(),
            page: Some("ingress".into()),
        },
    ];
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_lens_response_shape() {
        let r = CopilotResponse::new("Veyron Network Lens", "network_lens", "test");
        assert_eq!(r.module, "Veyron Network Lens");
    }
}
