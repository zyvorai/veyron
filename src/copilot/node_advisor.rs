// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Node capacity and schedulability pressure.
pub async fn node_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Node Advisor",
        "node_advisor",
        "Node capacity in cluster",
    );

    let nodes = client.list_nodes().await.unwrap_or_default();
    let vms = client.list_vms_for_scope(scope).await;
    let running = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                == Some("Running")
        })
        .count();

    if nodes.is_empty() {
        r.summary = "No nodes returned from the API — check RBAC and cluster connectivity.".into();
        return r;
    }

    let mut ready = 0usize;
    let mut not_ready = Vec::new();
    let mut pressure = Vec::new();

    for node in &nodes {
        let name = node.metadata.name.as_deref().unwrap_or("?");
        let conditions = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref());
        let is_ready = conditions
            .and_then(|c| c.iter().find(|x| x.type_ == "Ready"))
            .is_some_and(|c| c.status == "True");
        if is_ready {
            ready += 1;
        } else {
            not_ready.push(name.to_string());
        }

        if let Some(cond) = conditions {
            for c in cond {
                if c.type_.ends_with("Pressure") && c.status == "True" {
                    pressure.push(format!("{name}: {}", c.type_));
                }
            }
        }
    }

    r.evidence.push(format!("Nodes: {} ({} ready)", nodes.len(), ready));
    r.evidence
        .push(format!("Running VMs in scope: {running}"));
    if nodes.len() > 0 {
        r.evidence.push(format!(
            "VM density: {:.1} running VMs / node",
            running as f64 / nodes.len() as f64
        ));
    }

    for n in not_ready.iter().take(6) {
        r.evidence.push(format!("NotReady: {n}"));
    }
    for p in pressure.iter().take(6) {
        r.evidence.push(format!("Pressure: {p}"));
    }

    r.summary = if !not_ready.is_empty() {
        format!(
            "{} node(s) NotReady — scheduling and migrations may fail.",
            not_ready.len()
        )
    } else if !pressure.is_empty() {
        format!(
            "{} node(s) report resource pressure — review VM sizing and eviction.",
            pressure.len()
        )
    } else {
        format!(
            "All {} node(s) Ready — cluster can accept new VM workloads.",
            ready
        )
    };

    if !pressure.is_empty() {
        r.recommendations
            .push("Drain or migrate VMs from pressured nodes before adding capacity.".into());
    }
    if running as f64 / nodes.len().max(1) as f64 > 10.0 {
        r.recommendations
            .push("High VM-per-node density — plan additional workers or live migration.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Nodes".into(),
            action: "navigate".into(),
            page: Some("nodes".into()),
        },
        CopilotAction {
            label: "Open Scheduling".into(),
            action: "navigate".into(),
            page: Some("scheduling".into()),
        },
    ];
    r
}
