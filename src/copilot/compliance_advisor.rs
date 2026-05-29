// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::networking::v1::NetworkPolicy;
use k8s_openapi::api::rbac::v1::RoleBinding;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Fleet KubeVirt compliance posture (VM spec + namespace policy signals).
pub async fn compliance_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Compliance Advisor",
        "compliance_advisor",
        "Compliance posture in workspace",
    );

    let k8s = client.client();
    let vms = client.list_vms_for_scope(scope).await;
    let total = vms.len();

    if total == 0 {
        r.summary = "No VMs in scope — add workloads to evaluate compliance controls.".into();
        return r;
    }

    let mut limits = 0usize;
    let mut eviction = 0usize;
    let mut rng = 0usize;
    let mut no_host_net = 0usize;

    for vm in &vms {
        let vmi_spec = &vm.spec.template.spec;
        let domain = &vmi_spec.domain;
        if domain
            .resources
            .limits
            .as_ref()
            .map(|m| !m.is_empty())
            .unwrap_or(false)
        {
            limits += 1;
        }
        if vmi_spec.eviction_strategy.is_some() {
            eviction += 1;
        }
        if domain
            .devices
            .as_ref()
            .and_then(|d| d.rng.as_ref())
            .is_some()
        {
            rng += 1;
        }
        let host_net = vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false);
        if !host_net {
            no_host_net += 1;
        }
    }

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

    let mut np_total = 0usize;
    let mut rb_total = 0usize;
    for ns in namespaces.iter().take(12) {
        let np_api: Api<NetworkPolicy> = Api::namespaced(k8s.clone(), ns);
        if let Ok(list) = np_api.list(&ListParams::default()).await {
            np_total += list.items.len();
        }
        let rb_api: Api<RoleBinding> = Api::namespaced(k8s.clone(), ns);
        if let Ok(list) = rb_api.list(&ListParams::default()).await {
            rb_total += list.items.len();
        }
    }

    let pct = |n: usize| (n as f64 / total as f64 * 100.0).round() as u32;

    r.evidence.push(format!("VMs evaluated: {total}"));
    r.evidence.push(format!(
        "Resource limits set: {limits}/{total} ({}%)",
        pct(limits)
    ));
    r.evidence.push(format!(
        "Eviction strategy: {eviction}/{total} ({}%)",
        pct(eviction)
    ));
    r.evidence.push(format!("Virtio RNG: {rng}/{total} ({}%)", pct(rng)));
    r.evidence.push(format!(
        "No host networking: {no_host_net}/{total} ({}%)",
        pct(no_host_net)
    ));
    r.evidence
        .push(format!("NetworkPolicies across namespaces: {np_total}"));
    r.evidence.push(format!("RoleBindings across namespaces: {rb_total}"));

    let mut gaps = Vec::new();
    if limits < total {
        gaps.push(format!("{} VM(s) missing CPU/memory limits", total - limits));
    }
    if eviction < total {
        gaps.push(format!(
            "{} VM(s) missing evictionStrategy",
            total - eviction
        ));
    }
    if np_total == 0 {
        gaps.push("No NetworkPolicies in scoped namespaces".into());
    }
    for g in gaps.iter().take(4) {
        r.evidence.push(format!("Gap: {g}"));
    }

    let score = ((limits + eviction + rng + no_host_net) as f64 / (total * 4) as f64 * 100.0)
        .round() as u8;
    r.health_score = Some(score);

    r.summary = if gaps.is_empty() {
        format!("Compliance score {score}/100 — VM hardening controls look healthy.")
    } else {
        format!(
            "Compliance score {score}/100 — {} control gap(s) in scope.",
            gaps.len()
        )
    };

    if limits < total {
        r.recommendations
            .push("Set domain.resources.limits on production VMs.".into());
    }
    if np_total == 0 {
        r.recommendations.push(
            "Add namespace NetworkPolicies or Cilium policies for east-west isolation.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Compliance".into(),
            action: "navigate".into(),
            page: Some("compliance".into()),
        },
        CopilotAction {
            label: "Open Policies".into(),
            action: "navigate".into(),
            page: Some("policies".into()),
        },
    ];
    r
}
