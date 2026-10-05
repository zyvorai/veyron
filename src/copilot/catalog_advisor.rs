// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use super::{CopilotAction, CopilotResponse};
use crate::catalog;
use crate::kube::KubeClient;

/// VMTemplate / VMProfile catalog sync health.
pub async fn catalog_advisor(client: &KubeClient) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Catalog Advisor",
        "catalog_advisor",
        "Template catalog health",
    );

    let status = catalog::catalog_status(&client.client()).await;

    r.evidence.push(format!(
        "Embedded: {} templates, {} profiles",
        status.embedded_templates, status.embedded_profiles
    ));
    r.evidence.push(format!(
        "Cluster CRDs: {} templates, {} profiles",
        status.cluster_templates, status.cluster_profiles
    ));

    if let Some(msg) = &status.message {
        r.evidence.push(msg.clone());
    }

    for t in status.missing_templates.iter().take(6) {
        r.evidence.push(format!("Missing template: {t}"));
    }
    if status.missing_templates.len() > 6 {
        r.evidence.push(format!(
            "… and {} more missing templates",
            status.missing_templates.len() - 6
        ));
    }
    for p in status.missing_profiles.iter().take(4) {
        r.evidence.push(format!("Missing profile: {p}"));
    }

    r.summary = if !status.crds_available {
        "Catalog CRDs not installed — install operator CRDs before sync.".into()
    } else if status.in_sync {
        "Cluster catalog matches all embedded templates and profiles.".into()
    } else {
        format!(
            "Catalog drift — {} template(s) and {} profile(s) missing from cluster.",
            status.missing_templates.len(),
            status.missing_profiles.len()
        )
    };

    if !status.in_sync && status.crds_available {
        r.recommendations.push(
            "Run catalog sync from the Template Catalog page or `veyron catalog sync`.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Catalog".into(),
            action: "navigate".into(),
            page: Some("catalog".into()),
        },
        CopilotAction {
            label: "Open Template Foundry".into(),
            action: "navigate".into(),
            page: Some("app-store".into()),
        },
    ];
    r
}
