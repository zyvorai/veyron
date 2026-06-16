// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use k8s_openapi::api::core::v1::ResourceQuota;
use kube::api::{Api, ListParams};

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;

fn parse_ratio(used: &str, limit: &str) -> Option<f64> {
    let u: f64 = used.trim().parse().ok()?;
    let l: f64 = limit.trim().parse().ok()?;
    if l <= 0.0 {
        return None;
    }
    Some(u / l * 100.0)
}

/// Fleet ResourceQuota pressure across scoped namespaces.
pub async fn quota_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Quota Advisor",
        "quota_advisor",
        "ResourceQuota pressure in workspace",
    );

    let k8s = client.client();
    let api: Api<ResourceQuota> = if scope == "all" {
        Api::all(k8s.clone())
    } else {
        Api::namespaced(k8s.clone(), scope)
    };

    let Ok(list) = api.list(&ListParams::default()).await else {
        r.summary = "Could not list ResourceQuota objects — check RBAC.".into();
        return r;
    };

    if list.items.is_empty() {
        r.summary = "No ResourceQuotas in scope — namespaces have no hard limits.".into();
        r.recommendations
            .push("Define ResourceQuotas for production VM namespaces.".into());
        return r;
    }

    let mut pressured = Vec::new();
    for q in &list.items {
        let name = q.metadata.name.as_deref().unwrap_or("?");
        let ns = q.metadata.namespace.as_deref().unwrap_or("default");
        let hard = q.status.as_ref().and_then(|s| s.hard.as_ref());
        let used = q.status.as_ref().and_then(|s| s.used.as_ref());

        let cpu_pct = hard
            .and_then(|h| h.get("limits.cpu").or_else(|| h.get("cpu")))
            .and_then(|l| {
                used.and_then(|u| u.get("limits.cpu").or_else(|| u.get("cpu")))
                    .and_then(|u| parse_ratio(&u.0, &l.0))
            });
        let mem_pct = hard
            .and_then(|h| h.get("limits.memory").or_else(|| h.get("memory")))
            .and_then(|l| {
                used.and_then(|u| u.get("limits.memory").or_else(|| u.get("memory")))
                    .and_then(|u| parse_ratio(&u.0, &l.0))
            });

        let vm_limit = hard
            .and_then(|h| {
                h.get("count/virtualmachines.kubevirt.io")
                    .or_else(|| h.get("count/vms.kubevirt.io"))
            })
            .and_then(|v| v.0.parse::<u32>().ok());
        let vm_used = used
            .and_then(|u| {
                u.get("count/virtualmachines.kubevirt.io")
                    .or_else(|| u.get("count/vms.kubevirt.io"))
            })
            .and_then(|v| v.0.parse::<u32>().ok());

        let mut hot = false;
        let mut parts = vec![format!("{ns}/{name}")];
        if let Some(p) = cpu_pct {
            parts.push(format!("CPU {p:.0}%"));
            if p >= 85.0 {
                hot = true;
            }
        }
        if let Some(p) = mem_pct {
            parts.push(format!("memory {p:.0}%"));
            if p >= 85.0 {
                hot = true;
            }
        }
        if let (Some(limit), Some(used_n)) = (vm_limit, vm_used) {
            let pct = used_n as f64 / limit as f64 * 100.0;
            parts.push(format!("VMs {used_n}/{limit} ({pct:.0}%)"));
            if pct >= 85.0 {
                hot = true;
            }
        }
        let line = parts.join(" — ");
        if hot {
            pressured.push(line);
        } else {
            r.evidence.push(line);
        }
    }

    for p in pressured.iter().take(8) {
        r.evidence.push(format!("Pressure: {p}"));
    }

    r.summary = if pressured.is_empty() {
        format!(
            "All {} ResourceQuota(s) below 85% utilization.",
            list.items.len()
        )
    } else {
        format!(
            "{} namespace quota(s) above 85% — scheduling may fail soon.",
            pressured.len()
        )
    };

    if !pressured.is_empty() {
        r.recommendations.push(
            "Increase quotas, migrate VMs, or right-size workloads in pressured namespaces.".into(),
        );
    }

    r.actions = vec![CopilotAction {
        label: "Open Quotas".into(),
        action: "navigate".into(),
        page: Some("quotas".into()),
    }];
    r
}
