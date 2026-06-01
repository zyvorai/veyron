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
        r.summary =
            "No running VMs in scope — start workloads to collect performance signals.".into();
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
    let mut prom_hot = Vec::new();
    for vm in running.iter().take(12) {
        let name = vm.metadata.name.as_deref().unwrap_or("?");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        if prom {
            if let Some((cpu_p95, mem_p95)) = prom_percentiles(ns, name).await {
                if cpu_p95 >= 80.0 || mem_p95 >= 85.0 {
                    prom_hot.push(format!(
                        "{ns}/{name} — CPU p95 {cpu_p95:.0}%, memory p95 {mem_p95:.0}%"
                    ));
                    continue;
                }
                r.evidence.push(format!(
                    "{ns}/{name}: CPU p95 {cpu_p95:.0}%, memory p95 {mem_p95:.0}%"
                ));
                continue;
            }
        }
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

    for row in prom_hot.iter().chain(hot.iter()).take(8) {
        r.evidence.push(format!("Hot: {row}"));
    }

    let total_hot = prom_hot.len() + hot.len();
    r.summary = if total_hot == 0 {
        format!(
            "{} running VM(s) — no VM above 80% CPU / 85% memory{}.",
            running.len(),
            if prom { " (24h Prometheus p95)" } else { "" }
        )
    } else {
        format!("{total_hot} VM(s) show high CPU or memory — review sizing or noisy neighbors.")
    };

    if total_hot > 0 {
        r.recommendations
            .push("Right-size CPU/memory or migrate noisy VMs before they throttle.".into());
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

#[cfg(feature = "web")]
async fn prom_percentiles(ns: &str, vm: &str) -> Option<(f64, f64)> {
    let base = std::env::var("VMROGUE_PROMETHEUS_URL").ok()?;
    let end = chrono::Utc::now().timestamp();
    let start = end - 86400;
    let cpu_q =
        format!("rate(kubevirt_vmi_vcpu_seconds{{namespace=\"{ns}\", name=\"{vm}\"}}[5m]) * 100");
    let mem_q = format!("kubevirt_vmi_memory_resident_bytes{{namespace=\"{ns}\", name=\"{vm}\"}}");
    let cpu_p95 = prom_p95(&base, &cpu_q, start, end).await?;
    let mem_p95 = prom_p95(&base, &mem_q, start, end).await?;
    Some((cpu_p95, mem_p95))
}

#[cfg(feature = "web")]
async fn prom_p95(base: &str, query: &str, start: i64, end: i64) -> Option<f64> {
    let series = crate::api::prometheus::range_query_series(base, query, start, end, 300)
        .await
        .ok()?;
    let mut values: Vec<f64> = series
        .into_iter()
        .flat_map(|(_, pts)| pts.into_iter().map(|(_, v)| v))
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(crate::api::prometheus::percentile_sorted(&values, 0.95))
}

#[cfg(not(feature = "web"))]
async fn prom_percentiles(_ns: &str, _vm: &str) -> Option<(f64, f64)> {
    None
}
