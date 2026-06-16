// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::fixit::vm_health_score;
use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

pub async fn vm_doctor(client: &KubeClient, namespace: &str, name: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Doctor",
        "vm_doctor",
        format!("Health report: {namespace}/{name}"),
    );

    let vm = match client.get_vm(namespace, name).await {
        Ok(v) => v,
        Err(e) => {
            r.summary = format!("Could not load VirtualMachine: {e}");
            r.recommendations
                .push("Verify workspace and VM name.".into());
            return r;
        }
    };

    let status = vm
        .status
        .as_ref()
        .and_then(|s| s.printable_status.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let (score, label) = vm_health_score(&status, false);
    r.health_score = Some(score);
    r.summary = format!("{name} is {status}. Fleet health: {label} ({score}/100).");

    if let Ok(vmi) = client.get_vmi(namespace, name).await {
        if let Some(phase) = vmi.status.as_ref().and_then(|s| s.phase.clone()) {
            r.evidence.push(format!("VMI phase: {phase}"));
        }
        super::guest_inspector::enrich_doctor_with_guest(&mut r, &vmi);
        if let Some(node) = client.get_vm_node(namespace, name).await.ok().flatten() {
            r.evidence.push(format!("Scheduled node: {node}"));
        }
        if let Some(ip) = client.get_vm_ip(namespace, name).await.ok().flatten() {
            r.evidence.push(format!("Guest IP: {ip}"));
        }
    } else if status == "Running" || status == "Starting" {
        r.evidence
            .push("No VMI yet — VM may still be scheduling or starting.".into());
    }

    let events = client.list_events(namespace).await.unwrap_or_default();
    let relevant: Vec<_> = events
        .iter()
        .filter(|e| {
            e.involved_object
                .name
                .as_deref()
                .is_some_and(|n| n.contains(name))
        })
        .take(5)
        .collect();
    for ev in relevant {
        let reason = ev.reason.as_deref().unwrap_or("Event");
        let msg = ev.message.as_deref().unwrap_or("");
        r.evidence.push(format!("Event {reason}: {msg}"));
        let msg_l = msg.to_lowercase();
        if msg_l.contains("insufficient memory") || msg_l.contains("insufficient cpu") {
            r.recommendations
                .push("Reduce CPU/memory or add capacity to the cluster.".into());
        }
        if msg_l.contains("persistentvolumeclaim") || msg_l.contains("unbound") {
            r.recommendations
                .push("Check PVC binding and StorageClass.".into());
        }
    }

    match status.as_str() {
        "Running" if score >= 85 => {
            r.recommendations
                .push("No critical issues detected for this VM.".into());
        }
        "Stopped" => {
            r.recommendations
                .push("VM is stopped — start when ready or reclaim capacity.".into());
        }
        "Failed" | "Error" => {
            r.recommendations
                .push("Inspect virt-launcher pod logs and recent warning events.".into());
        }
        "Pending" | "Scheduling" => {
            r.recommendations
                .push("Run scheduling explainer: ask why this VM is pending.".into());
        }
        _ => {}
    }

    r.actions = vec![
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
        CopilotAction {
            label: "View events".into(),
            action: "navigate".into(),
            page: Some("events".into()),
        },
        CopilotAction {
            label: "Open Monitoring".into(),
            action: "navigate".into(),
            page: Some("monitoring".into()),
        },
    ];

    r
}
