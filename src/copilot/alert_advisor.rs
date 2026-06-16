// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Active warning events as an alert narrative.
pub async fn alert_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Alert Advisor",
        "alert_advisor",
        "Active warnings in workspace",
    );

    let events = client.list_events_for_scope(scope).await;
    let warnings: Vec<_> = events
        .iter()
        .filter(|e| e.type_.as_deref() == Some("Warning"))
        .collect();

    r.evidence
        .push(format!("Warning events in scope: {}", warnings.len()));

    let mut seen = std::collections::HashSet::new();
    for ev in warnings.iter().rev().take(80) {
        let reason = ev.reason.as_deref().unwrap_or("Warning");
        let msg = ev
            .message
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(100)
            .collect::<String>();
        let involved = ev.involved_object.name.as_deref().unwrap_or("?");
        let key = format!("{reason}:{involved}:{msg}");
        if seen.insert(key) {
            r.evidence.push(format!("{involved} — {reason}: {msg}"));
        }
        if r.evidence.len() > 14 {
            break;
        }
    }

    let prom = std::env::var("VMROGUE_ALERTMANAGER_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some();
    if prom {
        r.evidence.push(
            "VMROGUE_ALERTMANAGER_URL set — dashboard Alerts page can join firing alerts".into(),
        );
    }

    r.summary = if warnings.is_empty() {
        "No Warning events in the active namespace scope.".into()
    } else {
        format!(
            "{} Warning event(s) — review recurring reasons before they become outages.",
            warnings.len()
        )
    };

    if warnings.len() > 5 {
        r.recommendations
            .push("Group events by reason and open Veyron Doctor on affected VMs.".into());
    }
    if !prom {
        r.recommendations.push(
            "Set VMROGUE_ALERTMANAGER_URL to merge Prometheus Alertmanager timelines.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Alerts".into(),
            action: "navigate".into(),
            page: Some("alerts".into()),
        },
        CopilotAction {
            label: "Open Events".into(),
            action: "navigate".into(),
            page: Some("events".into()),
        },
    ];
    r
}
