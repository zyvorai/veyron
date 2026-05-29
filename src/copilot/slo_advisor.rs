// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// VM availability SLO narrative derived from printable status.
pub async fn slo_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue SLO Advisor",
        "slo_advisor",
        "VM availability SLO",
    );

    let vms = client.list_vms_for_scope(scope).await;
    if vms.is_empty() {
        r.summary = "No VMs in scope — SLO targets need running workloads.".into();
        return r;
    }

    let total = vms.len() as f64;
    let running = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                == Some("Running")
        })
        .count() as f64;
    let failed = vms
        .iter()
        .filter(|vm| {
            matches!(
                vm.status
                    .as_ref()
                    .and_then(|s| s.printable_status.as_deref()),
                Some("Failed") | Some("Error")
            )
        })
        .count();
    let pending = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                == Some("Pending")
        })
        .count();

    let availability = (running / total * 100.0).round();
    let target = 99.0;
    let budget_remaining = (availability - target).max(0.0);

    r.evidence.push(format!("VMs in scope: {}", vms.len()));
    r.evidence.push(format!("Running: {:.0}% ({running:.0}/{total:.0})", availability));
    r.evidence.push(format!("SLO target (heuristic): {target:.0}%"));
    r.evidence
        .push(format!("Error budget headroom: {budget_remaining:.1} pts"));
    if failed > 0 {
        r.evidence.push(format!("Failed/Error VMs: {failed}"));
    }
    if pending > 0 {
        r.evidence.push(format!("Pending VMs: {pending}"));
    }

    for vm in vms.iter().filter(|vm| {
        matches!(
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref()),
            Some("Failed") | Some("Error") | Some("Pending")
        )
    }).take(8) {
        let name = vm.metadata.name.as_deref().unwrap_or("?");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        let st = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            .unwrap_or("Unknown");
        r.evidence.push(format!("At risk: {ns}/{name} — {st}"));
    }

    r.summary = if availability >= target {
        format!(
            "Fleet availability {availability:.0}% meets the {target:.0}% heuristic SLO."
        )
    } else {
        format!(
            "Fleet availability {availability:.0}% is below {target:.0}% — {failed} failed, {pending} pending."
        )
    };

    if availability < target {
        r.recommendations
            .push("Run VMRogue Doctor on failed VMs and Scheduling Fleet on pending workloads.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open SLO".into(),
            action: "navigate".into(),
            page: Some("slo".into()),
        },
        CopilotAction {
            label: "Open VMs".into(),
            action: "navigate".into(),
            page: Some("vms".into()),
        },
    ];
    r
}
