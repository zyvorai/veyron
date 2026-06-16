// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Capacity / growth forecast narrative (heuristic + optional Prometheus).
pub async fn forecast_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Forecast Advisor",
        "forecast_advisor",
        "Capacity forecast in workspace",
    );

    let vms = client.list_vms_for_scope(scope).await;
    let nodes = client.list_nodes().await.unwrap_or_default();

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
    let node_count = nodes.len().max(1) as f64;
    let density = running / node_count;
    let running_ratio = if total > 0.0 { running / total } else { 0.0 };

    let growth = (0.05 + running_ratio * 0.12).min(0.35);
    let projected_vms = (total * (1.0 + growth)).ceil() as u32;

    r.evidence.push(format!("VMs in scope: {}", vms.len()));
    r.evidence.push(format!("Nodes: {}", nodes.len()));
    r.evidence
        .push(format!("Running density: {density:.1} VMs/node"));
    r.evidence.push(format!(
        "30-day VM growth estimate: +{:.0}% → ~{projected_vms} VMs",
        growth * 100.0
    ));

    if density > 8.0 {
        r.evidence
            .push("High VM density per node — plan capacity or spread workloads.".into());
        r.recommendations
            .push("Add worker nodes or enable live migration before density limits.".into());
    }
    if running_ratio < 0.5 && total > 3.0 {
        r.recommendations
            .push("Many stopped VMs — review idle capacity and snapshot before delete.".into());
    }

    #[cfg(feature = "web")]
    if std::env::var("VEYRON_PROMETHEUS_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some()
    {
        r.evidence.push(
            "Prometheus configured — forecasting page can join kubevirt_vmi_* trends.".into(),
        );
    } else {
        r.recommendations.push(
            "Set VEYRON_PROMETHEUS_URL for trend-based forecasts instead of heuristics only."
                .into(),
        );
    }

    r.summary = format!(
        "Fleet may grow ~{:.0}% in 30 days ({:.0} → ~{projected_vms} VMs) at current utilization.",
        growth * 100.0,
        total
    );

    r.actions = vec![CopilotAction {
        label: "Open Forecasting".into(),
        action: "navigate".into(),
        page: Some("forecasting".into()),
    }];
    r
}
