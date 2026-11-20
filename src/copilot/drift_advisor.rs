// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::operator_crds::VeyronVM;

/// Fleet operator drift from VeyronVM CRs.
pub async fn drift_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Drift Advisor",
        "drift_advisor",
        "Template drift in workspace",
    );

    let k8s = client.client();
    let api: Api<VeyronVM> = if scope == "all" {
        Api::all(k8s)
    } else {
        Api::namespaced(k8s, scope)
    };

    let Ok(list) = api.list(&ListParams::default()).await else {
        r.summary = "Could not list VeyronVM CRs — operator may not be installed in scope.".into();
        return r;
    };

    let total = list.items.len();
    let mut drifted = Vec::new();

    for vrvm in &list.items {
        if vrvm.status.as_ref().is_some_and(|s| s.drift_detected) {
            let name = vrvm.metadata.name.as_deref().unwrap_or("?");
            let ns = vrvm.metadata.namespace.as_deref().unwrap_or("default");
            let msg = vrvm
                .status
                .as_ref()
                .and_then(|s| s.drift_message.clone())
                .unwrap_or_else(|| "spec drift from template/profile".into());
            drifted.push(format!("{ns}/{name}: {msg}"));
        }
    }

    r.evidence.push(format!("VeyronVM CRs in scope: {total}"));
    r.evidence
        .push(format!("Drift detected: {}", drifted.len()));

    for row in drifted.iter().take(10) {
        r.evidence.push(format!("Drift: {row}"));
    }
    if drifted.len() > 10 {
        r.evidence
            .push(format!("… and {} more", drifted.len() - 10));
    }

    r.summary = if total == 0 {
        "No VeyronVM CRs — deploy via catalog or veyron deploy for drift tracking.".into()
    } else if drifted.is_empty() {
        format!("All {total} VeyronVM(s) match their template/profile baseline.")
    } else {
        format!(
            "{} of {total} VeyronVM(s) report drift — reconcile or update blueprints.",
            drifted.len()
        )
    };

    if !drifted.is_empty() {
        r.recommendations.push(
            "Review drift on the VM detail panel or export GitOps to re-sync desired state.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open VMs".into(),
            action: "navigate".into(),
            page: Some("vms".into()),
        },
        CopilotAction {
            label: "Open Insights".into(),
            action: "navigate".into(),
            page: Some("insights".into()),
        },
    ];
    r
}
