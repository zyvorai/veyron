// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::networking::v1::NetworkPolicy;
use kube::api::{Api, ApiResource, DynamicObject, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Cilium / network policy posture for the workspace.
pub async fn cilium_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Cilium Advisor",
        "cilium_advisor",
        "Network policy posture",
    );

    let k8s = client.client();
    let mut agent_ns = String::new();
    let mut agent_ready = 0u32;
    let mut agent_desired = 0u32;

    for ns in ["cilium-system", "kube-system"] {
        let ds_api: Api<DaemonSet> = Api::namespaced(k8s.clone(), ns);
        if let Ok(list) = ds_api.list(&ListParams::default()).await {
            for ds in &list.items {
                let name = ds.metadata.name.as_deref().unwrap_or("").to_lowercase();
                if name.contains("cilium") && !name.contains("operator") {
                    agent_ns = ns.to_string();
                    agent_desired = ds
                        .status
                        .as_ref()
                        .map(|s| s.desired_number_scheduled)
                        .unwrap_or(0) as u32;
                    agent_ready = ds.status.as_ref().map(|s| s.number_ready).unwrap_or(0) as u32;
                }
            }
        }
    }

    let cilium_installed = !agent_ns.is_empty();
    if cilium_installed {
        r.evidence.push(format!(
            "Cilium agents: {agent_ready}/{agent_desired} ready in {agent_ns}"
        ));
    } else {
        r.evidence
            .push("Cilium DaemonSet not detected — using Kubernetes NetworkPolicy view".into());
    }

    let mut cnp_count = 0usize;
    let ar = ApiResource {
        group: "cilium.io".into(),
        version: "v2".into(),
        api_version: "cilium.io/v2".into(),
        kind: "CiliumNetworkPolicy".into(),
        plural: "ciliumnetworkpolicies".into(),
    };
    let cnp_api: Api<DynamicObject> = if scope == "all" {
        kube::Api::all_with(k8s.clone(), &ar)
    } else {
        kube::Api::namespaced_with(k8s.clone(), scope, &ar)
    };
    if let Ok(list) = cnp_api.list(&ListParams::default().limit(50)).await {
        cnp_count = list.items.len();
        r.evidence
            .push(format!("CiliumNetworkPolicies in scope: {cnp_count}"));
    }

    let vms = client.list_vms_for_scope(scope).await;
    let namespaces: Vec<String> = if scope == "all" {
        let mut ns: Vec<String> = vms
            .iter()
            .filter_map(|v| v.metadata.namespace.clone())
            .collect();
        ns.sort();
        ns.dedup();
        ns
    } else {
        vec![scope.to_string()]
    };

    let mut np_count = 0usize;
    for ns in namespaces.iter().take(12) {
        let np_api: Api<NetworkPolicy> = Api::namespaced(k8s.clone(), ns);
        if let Ok(list) = np_api.list(&ListParams::default()).await {
            np_count += list.items.len();
        }
    }
    r.evidence
        .push(format!("Kubernetes NetworkPolicies in scope: {np_count}"));

    let vm_count = vms.len();
    if vm_count > 0 && np_count == 0 && cnp_count == 0 {
        r.evidence.push(format!(
            "{vm_count} VM(s) with no namespace NetworkPolicy in scope"
        ));
    }

    r.summary = if cilium_installed && agent_ready < agent_desired {
        format!("Cilium degraded ({agent_ready}/{agent_desired} agents) — check {agent_ns} pods.")
    } else if cilium_installed {
        format!("Cilium healthy — {cnp_count} CNP + {np_count} NetworkPolicy in scope.")
    } else if np_count > 0 {
        format!("Standard NetworkPolicy mode — {np_count} policy(ies) in scope.")
    } else {
        "No Cilium or NetworkPolicies detected — VMs may have flat namespace networking.".into()
    };

    if np_count == 0 && cnp_count == 0 {
        r.recommendations.push(
            "Define default-deny egress/ingress for VM namespaces or use Cilium cluster policies."
                .into(),
        );
    }
    if cilium_installed && agent_ready < agent_desired {
        r.recommendations
            .push("Restart unhealthy cilium-agent pods and verify kernel/BPF requirements.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Cilium".into(),
            action: "navigate".into(),
            page: Some("cilium".into()),
        },
        CopilotAction {
            label: "Open Network Policies".into(),
            action: "navigate".into(),
            page: Some("network-policies".into()),
        },
    ];
    r
}
