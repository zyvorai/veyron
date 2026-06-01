// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};

const INTEGRATIONS: &[(&str, &str)] = &[
    (
        "VMROGUE_PROMETHEUS_URL",
        "Metrics, PVC usage, performance p95",
    ),
    (
        "VMROGUE_OPENCOST_URL",
        "Real allocation costs (with VMROGUE_COST_BACKEND=opencost)",
    ),
    (
        "VMROGUE_TRIVY_URL",
        "CVE findings merged into security posture",
    ),
    ("VMROGUE_LOKI_URL", "Cluster-wide logs (LogQL)"),
    ("VMROGUE_ALERTMANAGER_URL", "Active alert timeline"),
    ("VMROGUE_ARGOCD_URL", "GitOps sync triggers"),
    ("VMROGUE_GRAFANA_URL", "External Grafana console links"),
    ("VMROGUE_JAEGER_QUERY_URL", "Distributed traces"),
];

/// Which optional backends are configured vs missing.
pub async fn integrations_advisor() -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Integrations Advisor",
        "integrations_advisor",
        "Optional backend wiring",
    );

    let mut configured = Vec::new();
    let mut missing = Vec::new();

    for (var, feeds) in INTEGRATIONS {
        let set = std::env::var(var)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_some();
        if set {
            configured.push(format!("{var} — {feeds}"));
        } else {
            missing.push(format!("{var} — {feeds}"));
        }
    }

    r.evidence.push(format!(
        "Configured: {} / {}",
        configured.len(),
        INTEGRATIONS.len()
    ));
    for row in configured.iter().take(8) {
        r.evidence.push(format!("✔ {row}"));
    }
    for row in missing.iter().take(6) {
        r.evidence.push(format!("○ {row}"));
    }

    r.summary = if configured.is_empty() {
        "No optional integrations configured — dashboard uses Kubernetes heuristics only.".into()
    } else if missing.is_empty() {
        "All listed optional backends are configured.".into()
    } else {
        format!(
            "{} integration(s) active; {} still optional for production parity.",
            configured.len(),
            missing.len()
        )
    };

    if missing.iter().any(|m| m.contains("PROMETHEUS")) {
        r.recommendations.push(
            "Prometheus unlocks storage usage %, performance percentiles, and guest-exec PVC join."
                .into(),
        );
    }
    if missing.iter().any(|m| m.contains("OPENCOST")) {
        r.recommendations
            .push("OpenCost replaces reference-rate cost estimates with allocation data.".into());
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Integrations".into(),
            action: "navigate".into(),
            page: Some("integrations".into()),
        },
        CopilotAction {
            label: "Open Monitoring install doc".into(),
            action: "navigate".into(),
            page: Some("monitoring".into()),
        },
    ];
    r
}
