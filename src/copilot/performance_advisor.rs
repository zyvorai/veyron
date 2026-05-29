// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

/// Fleet performance hotspots from live metrics (Prometheus when configured).
pub async fn performance_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Performance Advisor",
        "performance_advisor",
        "Performance hotspots in workspace",
    );

    let vms = client.list_vms_for_scope(scope).await;
    let running: Vec<_> = vms
        .iter()
        .filter(|vm| {
            vm.status
                .as_ref()
                .and_then(|s| s.printable_status.as_deref())
                == Some("Running")
        })
        .collect();

    if running.is_empty() {
        r.summary = "No running VMs in scope — start workloads to collect performance signals.".into();
        return r;
    }

    let prom = std::env::var("VMROGUE_PROMETHEUS_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .is_some();
    r.evidence.push(format!("Running VMs: {}", running.len()));
    r.evidence.push(if prom {
        "Data source: live metrics + Prometheus kubevirt_vmi_* (when available)".into()
    } else {
        "Data source: live kubelet-style estimates — set VMROGUE_PROMETHEUS_URL for p95 history"
            .into()
    });

    let mut hot = Vec::new();
    for vm in running.iter().take(12) {
        let name = vm.metadata.name.as_deref().unwrap_or("?");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        let collector = crate::monitoring::metrics::MetricsCollector::new(ns.to_string());
        if let Ok(m) = collector.collect(name).await {
            let cpu = m.cpu.usage_percent;
            let mem = m.memory.usage_percent;
            if cpu >= 80.0 || mem >= 85.0 {
                hot.push(format!("{ns}/{name} — CPU {cpu:.0}%, memory {mem:.0}%"));
            } else {
                r.evidence
                    .push(format!("{ns}/{name}: CPU {cpu:.0}%, memory {mem:.0}%"));
            }
        }
    }

    for row in hot.iter().take(8) {
        r.evidence.push(format!("Hot: {row}"));
    }

    r.summary = if hot.is_empty() {
        format!(
            "{} running VM(s) — no VM above 80% CPU / 85% memory in the latest sample.",
            running.len()
        )
    } else {
        format!(
            "{} VM(s) show high CPU or memory — review sizing or noisy neighbors.",
            hot.len()
        )
    };

    if !hot.is_empty() {
        r.recommendations.push(
            "Right-size CPU/memory or migrate noisy VMs before they throttle.".into(),
        );
    }
    if !prom {
        r.recommendations.push(
            "Install kube-prometheus-stack and set VMROGUE_PROMETHEUS_URL for historical p95/p99."
                .into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open Performance".into(),
            action: "navigate".into(),
            page: Some("performance".into()),
        },
        CopilotAction {
            label: "Open Monitoring".into(),
            action: "navigate".into(),
            page: Some("monitoring".into()),
        },
    ];
    r
}
