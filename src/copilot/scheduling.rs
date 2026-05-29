// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

pub async fn scheduling_explainer(client: &KubeClient, namespace: &str, name: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Scheduling Explainer",
        "scheduling",
        format!("Why is {namespace}/{name} scheduling?"),
    );

    let status = client
        .get_vm(namespace, name)
        .await
        .ok()
        .and_then(|vm| vm.status)
        .and_then(|s| s.printable_status)
        .unwrap_or_else(|| "Unknown".to_string());

    r.evidence.push(format!("VM status: {status}"));

    let events = client.list_events(namespace).await.unwrap_or_default();
    let mut scheduling_msgs = Vec::new();
    for ev in &events {
        let msg = ev.message.as_deref().unwrap_or("");
        let involved = ev.involved_object.name.as_deref().unwrap_or("");
        if !involved.contains(name) && !involved.contains("virt-launcher") {
            continue;
        }
        let ml = msg.to_lowercase();
        if ml.contains("insufficient")
            || ml.contains("nodes are available")
            || ml.contains("taint")
            || ml.contains("affinity")
            || ml.contains("persistentvolumeclaim")
            || ml.contains("unbound")
            || ml.contains("failed scheduling")
        {
            scheduling_msgs.push(format!(
                "{}: {}",
                ev.reason.as_deref().unwrap_or("Scheduled"),
                msg
            ));
        }
    }

    if scheduling_msgs.is_empty() {
        r.summary =
            "No recent scheduling failure events found for this VM in the workspace.".into();
        r.evidence.push(
            "If the VM is Pending, check VMI conditions and virt-launcher pod events with kubectl."
                .into(),
        );
    } else {
        r.summary = "Scheduling is blocked or delayed for the following reasons:".into();
        r.evidence.extend(scheduling_msgs.iter().cloned());

        for msg in &scheduling_msgs {
            let ml = msg.to_lowercase();
            if ml.contains("insufficient memory") {
                r.recommendations.push(
                    "Best fix: reduce guest memory or free memory on nodes (stop idle VMs)."
                        .into(),
                );
            }
            if ml.contains("insufficient cpu") {
                r.recommendations
                    .push("Best fix: reduce vCPU request or add worker capacity.".into());
            }
            if ml.contains("taint") {
                r.recommendations.push(
                    "Best fix: add a toleration, remove the taint, or schedule on another node."
                        .into(),
                );
            }
            if ml.contains("affinity") || ml.contains("selector") {
                r.recommendations.push(
                    "Best fix: relax nodeSelector/affinity or label a capable node.".into(),
                );
            }
            if ml.contains("persistentvolumeclaim") || ml.contains("unbound") {
                r.recommendations.push(
                    "Best fix: ensure PVC is bound — check StorageClass and provisioner.".into(),
                );
            }
        }
    }

    let nodes = client.list_nodes().await.unwrap_or_default();
    if !nodes.is_empty() {
        let ready = nodes
            .iter()
            .filter(|n| {
                n.status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .and_then(|c| c.iter().find(|x| x.type_ == "Ready"))
                    .is_some_and(|c| c.status == "True")
            })
            .count();
        r.evidence
            .push(format!("Cluster nodes ready: {ready}/{}", nodes.len()));
    }

    r.actions = vec![
        CopilotAction {
            label: "Show node capacity".into(),
            action: "navigate".into(),
            page: Some("nodes".into()),
        },
        CopilotAction {
            label: "Storage tiers".into(),
            action: "navigate".into(),
            page: Some("storage".into()),
        },
        CopilotAction {
            label: "Generate fix command".into(),
            action: "copy_kubectl".into(),
            page: Some(format!(
                "kubectl describe vm {name} -n {namespace} && kubectl get events -n {namespace} --field-selector involvedObject.name={name}"
            )),
        },
    ];

    r
}
