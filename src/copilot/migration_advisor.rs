// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::types::VirtualMachineInstanceMigration;

/// Live and recent KubeVirt live migrations.
pub async fn migration_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Migration Advisor",
        "migration_advisor",
        "Live migration status",
    );

    let k8s = client.client();
    let api: Api<VirtualMachineInstanceMigration> = if scope == "all" {
        Api::all(k8s)
    } else {
        Api::namespaced(k8s, scope)
    };

    let Ok(list) = api.list(&ListParams::default()).await else {
        r.summary = "Could not list VirtualMachineInstanceMigration objects.".into();
        return r;
    };

    let mut active = 0usize;
    let mut failed = 0usize;
    let mut succeeded = 0usize;

    for mig in &list.items {
        let phase = mig
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .unwrap_or("Unknown");
        match phase {
            "Running" | "Pending" => active += 1,
            "Failed" => failed += 1,
            "Succeeded" => succeeded += 1,
            _ => {}
        }

        if matches!(phase, "Running" | "Pending" | "Failed") {
            let vmi = mig
                .spec
                .vmi_name
                .as_deref()
                .or(mig.metadata.name.as_deref())
                .unwrap_or("?");
            let ns = mig.metadata.namespace.as_deref().unwrap_or("default");
            let src = mig
                .status
                .as_ref()
                .and_then(|s| s.migration_state.as_ref())
                .and_then(|ms| ms.source_node.clone())
                .unwrap_or_else(|| "?".into());
            let dst = mig
                .status
                .as_ref()
                .and_then(|s| s.migration_state.as_ref())
                .and_then(|ms| ms.target_node.clone())
                .unwrap_or_else(|| "?".into());
            r.evidence
                .push(format!("{ns}/{vmi}: {phase} ({src} → {dst})"));
        }
    }

    r.evidence.push(format!(
        "Migrations: {} total ({} active, {} failed, {} succeeded)",
        list.items.len(),
        active,
        failed,
        succeeded
    ));

    r.summary = if list.items.is_empty() {
        "No live migrations in scope — cluster is quiescent.".into()
    } else if active > 0 {
        format!("{active} migration(s) in flight — avoid node drains until complete.")
    } else if failed > 0 {
        format!("{failed} failed migration(s) — inspect VMIM status and network bandwidth.")
    } else {
        format!("{succeeded} completed migration(s); none active.")
    };

    if failed > 0 {
        r.recommendations.push(
            "Check virt-launcher logs and ensure migration network bandwidth is sufficient.".into(),
        );
    }
    if active > 0 {
        r.recommendations
            .push("Defer node maintenance until Running migrations reach Succeeded.".into());
    }
    r.recommendations.push(
        "For P2V readiness on Linux VMs, use GET /api/v1/vms/:ns/:name/guest/migrate-score?target=kvm.".into(),
    );

    r.actions = vec![
        CopilotAction {
            label: "Open Nodes".into(),
            action: "navigate".into(),
            page: Some("nodes".into()),
        },
        CopilotAction {
            label: "Open VMs".into(),
            action: "navigate".into(),
            page: Some("vms".into()),
        },
    ];
    r
}
