// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};

pub fn vm_health_score(status: &str, drift_detected: bool) -> (u8, String) {
    let base: u8 = match status {
        "Running" => 92,
        "Stopped" => 68,
        "Failed" | "Error" => 35,
        _ => 55,
    };
    let score = if drift_detected {
        base.saturating_sub(18)
    } else {
        base
    };
    let label = if score >= 85 {
        "Healthy"
    } else if score >= 65 {
        "Fair"
    } else if score >= 45 {
        "Needs attention"
    } else {
        "Critical"
    };
    (score, label.to_string())
}

/// Rule-based error explainer (Veyron Error Explainer v1).
pub fn explain_error_message(message: &str, context: Option<&str>) -> CopilotResponse {
    let lower = message.to_lowercase();
    let ctx = context.unwrap_or("general");

    type Fixes = Vec<(&'static str, &'static str, Option<&'static str>)>;
    let (title, explanation, fixes): (&str, &str, Fixes) = if lower
        .contains("persistentvolumeclaim")
        || lower.contains("unbound immediate persistentvolumeclaims")
        || lower.contains("waiting for a volume")
    {
        (
            "Disk storage is not ready",
            "This VM cannot start because its disk volume is still being provisioned or no storage is available for the requested StorageClass.",
            vec![
                ("Check storage tiers", "navigate", Some("storage")),
                ("Retry start", "retry", None),
            ],
        )
    } else if lower.contains("network attachment definition")
        || lower.contains("networkattachmentdefinition")
        || lower.contains("multus")
    {
        (
            "VM network is missing",
            "The VM references a network that does not exist in this workspace. Create the network or choose an existing one.",
            vec![
                ("Open networks", "navigate", Some("cilium")),
                ("Edit VM network", "edit_network", Some("vms")),
            ],
        )
    } else if lower.contains("insufficient cpu")
        || lower.contains("insufficient memory")
        || (lower.contains("0/") && lower.contains("nodes are available"))
    {
        (
            "Not enough cluster capacity",
            "No node can schedule this VM with its current CPU, memory, or placement rules.",
            vec![
                ("View nodes", "navigate", Some("nodes")),
                ("Resize VM", "resize", Some("vms")),
            ],
        )
    } else if lower.contains("forbidden") || lower.contains("403") {
        (
            "Permission denied",
            "Your API key or SSO role cannot perform this action on this resource.",
            vec![("Review access", "navigate", Some("rbac"))],
        )
    } else if ctx == "migrate" && (lower.contains("migration") || lower.contains("live migrate")) {
        (
            "Live migration blocked",
            "KubeVirt could not migrate this running VM. Common causes: shared storage missing, VM not running, or network/storage policy.",
            vec![
                ("Check VM status", "navigate", Some("vms")),
                ("View nodes", "navigate", Some("nodes")),
            ],
        )
    } else if lower.contains("not found") || lower.contains("404") {
        (
            "Resource not found",
            "The VM or related object may have been deleted or is in another workspace.",
            vec![("Refresh VM list", "refresh", Some("vms"))],
        )
    } else {
        (
            "Operation failed",
            "The platform returned an error. Check events and VM details for the underlying Kubernetes or KubeVirt cause.",
            vec![
                ("View events", "navigate", Some("events")),
                ("Open VM", "navigate", Some("vms")),
            ],
        )
    };

    let mut r = CopilotResponse::new("Veyron Error Explainer", "explain", title);
    r.summary = explanation.to_string();
    r.evidence.push(message.to_string());
    r.recommendations = fixes.iter().map(|(l, _, _)| (*l).to_string()).collect();
    r.actions = fixes
        .into_iter()
        .map(|(label, action, page)| CopilotAction {
            label: label.to_string(),
            action: action.to_string(),
            page: page.map(str::to_string),
        })
        .collect();
    r
}
