// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use crate::kube::KubeClient;

use super::event::SecurityEvent;

/// Collect security events from Kubernetes audit trail (events) and API-derived signals.
#[cfg(feature = "web")]
pub async fn collect_from_cluster(client: &KubeClient, scope: &str) -> Vec<SecurityEvent> {
    let mut events = Vec::new();
    let k8s_events = client.list_events_for_scope(scope).await;

    for ev in &k8s_events {
        let involved = &ev.involved_object;
        let resource_type = involved.kind.clone().unwrap_or_default();
        let resource_name = involved.name.clone().unwrap_or_default();
        let ns = involved.namespace.clone().unwrap_or_default();
        let reason = ev.reason.clone().unwrap_or_default();
        let message = ev.message.clone().unwrap_or_default();
        let severity = match ev.type_.as_deref() {
            Some("Warning") => "medium",
            Some("Normal") => "low",
            _ => "info",
        };
        let mut se = SecurityEvent::new("audit", severity, "kubernetes", &message)
            .with_namespace(&ns)
            .with_action(&reason)
            .with_target(format!("{resource_type}/{resource_name}"))
            .with_outcome(ev.type_.as_deref().unwrap_or("Unknown"));
        if resource_type == "VirtualMachine" || resource_type == "VirtualMachineInstance" {
            se = se.with_vm(&resource_name);
        }
        se = se.label("k8s.reason", &reason);
        events.push(se);
    }

    events.extend(collect_finding_events(client, scope).await);
    events
}

#[cfg(feature = "web")]
async fn collect_finding_events(client: &KubeClient, scope: &str) -> Vec<SecurityEvent> {
    let vms = client.list_vms_for_scope(scope).await;
    let mut out = Vec::new();
    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("unknown");
        let ns = vm.metadata.namespace.as_deref().unwrap_or(scope);
        let vmi_spec = &vm.spec.template.spec;
        if vmi_spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().any(|n| n.name == "host"))
            .unwrap_or(false)
        {
            out.push(
                SecurityEvent::new(
                    "finding",
                    "critical",
                    "vmrogue",
                    format!("Host network on {ns}/{name}"),
                )
                .with_namespace(ns)
                .with_vm(name)
                .with_action("privileged-vm-spec")
                .label("finding.category", "Network"),
            );
        }
    }
    out
}

/// Record an API mutation for SOC audit (called from middleware).
#[cfg(feature = "web")]
pub fn api_audit_event(
    actor: &str,
    method: &str,
    path: &str,
    outcome: &str,
    namespace: Option<&str>,
) -> SecurityEvent {
    let severity = if outcome == "success" { "low" } else { "medium" };
    let mut ev = SecurityEvent::new(
        "audit",
        severity,
        "vmrogue",
        format!("{method} {path} — {outcome}"),
    )
    .with_actor(actor)
    .with_action(method)
    .with_target(path)
    .with_outcome(outcome);
    if let Some(ns) = namespace {
        ev = ev.with_namespace(ns);
    }
    if path.contains("/vms/") {
        if let Some((_, rest)) = path.split_once("/vms/") {
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() >= 2 {
                ev = ev.with_namespace(parts[0]).with_vm(parts[1]);
            }
        }
    }
    ev
}
