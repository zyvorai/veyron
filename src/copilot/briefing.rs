// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use serde::{Deserialize, Serialize};

use super::fixit;
use crate::kube::KubeClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotBriefingItem {
    pub id: String,
    /// `info` | `warn` | `error`
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// Lightweight fleet scan for Datacenter home — max 3 actionable Copilot prompts.
pub async fn fleet_briefing(client: &KubeClient, scope: &str) -> Vec<CopilotBriefingItem> {
    let mut items = Vec::new();
    let vms = client.list_vms_for_scope(scope).await;

    let mut running = 0usize;
    let mut pending = 0usize;
    let mut failed = 0usize;
    let mut degraded = 0usize;

    for vm in &vms {
        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            .unwrap_or("Unknown");
        match status {
            "Running" => {
                running += 1;
                let (score, _) = fixit::vm_health_score(status, false);
                if score < 65 {
                    degraded += 1;
                }
            }
            "Pending" => pending += 1,
            "Failed" | "Error" | "Unknown" => failed += 1,
            _ => {}
        }
    }

    if failed > 0 {
        items.push(CopilotBriefingItem {
            id: "failed-vms".into(),
            severity: "error".into(),
            title: format!("{failed} VM(s) failed or unknown"),
            detail: "Run Veyron Doctor on affected workloads.".into(),
            action: "copilot:unhealthy".into(),
            query: None,
        });
    }
    if pending > 0 {
        items.push(CopilotBriefingItem {
            id: "pending-vms".into(),
            severity: "warn".into(),
            title: format!("{pending} VM(s) stuck Pending"),
            detail: "Scheduling Explainer can surface unbound PVCs or capacity gaps.".into(),
            action: "copilot:scheduling".into(),
            query: None,
        });
    }
    if degraded > 0 {
        items.push(CopilotBriefingItem {
            id: "degraded-vms".into(),
            severity: "warn".into(),
            title: format!("{degraded} running VM(s) look degraded"),
            detail: "Health score below 65 — review events and guest agent.".into(),
            action: "copilot:unhealthy".into(),
            query: None,
        });
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
        if ready < nodes.len() {
            items.push(CopilotBriefingItem {
                id: "nodes-not-ready".into(),
                severity: "warn".into(),
                title: format!("{} node(s) not Ready", nodes.len() - ready),
                detail: "Node Advisor summarizes capacity and pressure.".into(),
                action: "open_copilot".into(),
                query: Some("Node capacity and pressure".into()),
            });
        }
    }

    if items.is_empty() {
        let summary = if vms.is_empty() {
            "No VMs in scope — forge one from Template Foundry.".to_string()
        } else {
            format!(
                "{running} of {} VM(s) running — no urgent Ask Zeus actions.",
                vms.len()
            )
        };
        items.push(CopilotBriefingItem {
            id: "fleet-ok".into(),
            severity: "info".into(),
            title: "Fleet looks healthy".into(),
            detail: summary,
            action: "open_copilot".into(),
            query: None,
        });
    }

    items.truncate(3);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefing_item_serializes_action() {
        let item = CopilotBriefingItem {
            id: "test".into(),
            severity: "warn".into(),
            title: "Title".into(),
            detail: "Detail".into(),
            action: "copilot:scheduling".into(),
            query: None,
        };
        let j = serde_json::to_value(&item).unwrap();
        assert_eq!(j["action"], "copilot:scheduling");
    }
}
