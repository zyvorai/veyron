// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use super::{
    alert_advisor, backup_advisor, cilium_advisor, compliance_advisor, cost_advisor,
    cost_advisor_vm, drift_advisor, forecast_advisor, gitops_advisor, guest_filesystem_report,
    guest_inspector, integrations_advisor, migration_advisor, network_lens, node_advisor,
    observability_advisor, performance_advisor, scheduling_explainer, scheduling_fleet_advisor,
    security_sentinel, security_sentinel_fleet, slo_advisor, storage_doctor, vm_doctor,
    yaml_build, CopilotAction, CopilotAskRequest, CopilotIntent, CopilotResponse,
};
use super::fixit::explain_error_message;
use crate::kube::KubeClient;

pub fn resolve_vm_target<'a>(
    req: &'a CopilotAskRequest,
    scope: &'a str,
    query: &str,
) -> Option<(&'a str, String)> {
    if let (Some(ns), Some(name)) = (req.namespace.as_deref(), req.vm_name.as_deref()) {
        return Some((ns, name.to_string()));
    }
    extract_vm_name(query).map(|name| {
        let ns = req.namespace.as_deref().unwrap_or(scope);
        (ns, name)
    })
}

pub async fn dispatch_intent(
    client: &KubeClient,
    scope: &str,
    intent: CopilotIntent,
    req: &CopilotAskRequest,
) -> CopilotResponse {
    let query = req.query.trim();
    match intent {
        CopilotIntent::VmDoctor => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => vm_doctor(client, ns, &name).await,
            None => vm_name_required("VMRogue Doctor", "vm_doctor", query),
        },
        CopilotIntent::SchedulingExplainer => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => scheduling_explainer(client, ns, &name).await,
            None => scheduling_fleet_advisor(client, scope).await,
        },
        CopilotIntent::UnhealthyFleet => list_unhealthy_vms(client, scope).await,
        CopilotIntent::YamlBuilder => {
            let spec = yaml_build::parse_spec_from_query(query);
            yaml_build::yaml_preview(client, scope, spec).await
        }
        CopilotIntent::BackupAdvisor => backup_advisor(client, scope).await,
        CopilotIntent::CostAdvisor => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => cost_advisor_vm(client, ns, &name).await,
            None => cost_advisor(client, scope).await,
        },
        CopilotIntent::NetworkLens => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => network_lens(client, ns, &name).await,
            None => vm_name_required(
                "VMRogue Network Lens",
                "network_lens",
                "Include a VM name — e.g. “Show network for vm-app-01”.",
            ),
        },
        CopilotIntent::GuestInspector => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => guest_inspector(client, ns, &name).await,
            None => vm_name_required(
                "VMRogue Guest Inspector",
                "guest_inspector",
                "Include a VM name — e.g. “Show guest agent status for vm-db-01”.",
            ),
        },
        CopilotIntent::GuestFilesystem => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => guest_filesystem_report(client, ns, &name).await,
            None => vm_name_required(
                "VMRogue Guest Filesystem",
                "guest_filesystem",
                "Include a VM name — e.g. “Show guest filesystem for vm-db-01”.",
            ),
        },
        CopilotIntent::StorageDoctor => storage_doctor(client, scope).await,
        CopilotIntent::SecuritySentinel => match resolve_vm_target(req, scope, query) {
            Some((ns, name)) => security_sentinel(client, ns, &name).await,
            None => security_sentinel_fleet(client, scope).await,
        },
        CopilotIntent::PerformanceAdvisor => performance_advisor(client, scope).await,
        CopilotIntent::GitopsAdvisor => gitops_advisor(client, scope).await,
        CopilotIntent::ForecastAdvisor => forecast_advisor(client, scope).await,
        CopilotIntent::IntegrationsAdvisor => integrations_advisor().await,
        CopilotIntent::ComplianceAdvisor => compliance_advisor(client, scope).await,
        CopilotIntent::ObservabilityAdvisor => observability_advisor(client, scope).await,
        CopilotIntent::CiliumAdvisor => cilium_advisor(client, scope).await,
        CopilotIntent::NodeAdvisor => node_advisor(client, scope).await,
        CopilotIntent::DriftAdvisor => drift_advisor(client, scope).await,
        CopilotIntent::AlertAdvisor => alert_advisor(client, scope).await,
        CopilotIntent::SloAdvisor => slo_advisor(client, scope).await,
        CopilotIntent::MigrationAdvisor => migration_advisor(client, scope).await,
        CopilotIntent::QuotaAdvisor => super::quota_advisor::quota_advisor(client, scope).await,
        CopilotIntent::CatalogAdvisor => super::catalog_advisor::catalog_advisor(client).await,
        CopilotIntent::VeleroDrAdvisor => {
            super::velero_dr_advisor::velero_dr_advisor(client, scope).await
        }
        CopilotIntent::ErrorExplainer => explain_error_message(query, Some("copilot")),
        CopilotIntent::GeneralHelp => super::general_help(),
    }
}

fn vm_name_required(module: &str, intent: &str, summary: impl Into<String>) -> CopilotResponse {
    let mut r = CopilotResponse::new(module, intent, "Which VM should I inspect?");
    r.summary = summary.into();
    r.actions.push(CopilotAction {
        label: "Open VMs".into(),
        action: "navigate".into(),
        page: Some("vms".into()),
    });
    r
}

async fn list_unhealthy_vms(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Doctor",
        "unhealthy_fleet",
        "Unhealthy VMs in scope",
    );
    let vms = client.list_vms_for_scope(scope).await;
    let mut unhealthy = Vec::new();
    for vm in &vms {
        let name = vm.metadata.name.as_deref().unwrap_or("");
        let ns = vm.metadata.namespace.as_deref().unwrap_or("default");
        let status = vm
            .status
            .as_ref()
            .and_then(|s| s.printable_status.as_deref())
            .unwrap_or("Unknown");
        if matches!(status, "Failed" | "Error" | "Unknown") || status == "Pending" {
            unhealthy.push(format!("{ns}/{name} — {status}"));
        } else if status == "Stopped" {
            continue;
        } else if status == "Running" {
            let (score, label) = super::fixit::vm_health_score(status, false);
            if score < 65 {
                unhealthy.push(format!("{ns}/{name} — {label} ({score}%)"));
            }
        }
    }
    if unhealthy.is_empty() {
        r.summary = "No clearly unhealthy VMs in the active workspace scope.".into();
        r.evidence
            .push("Checked printable VM status and fleet health heuristics.".into());
    } else {
        r.summary = format!("Found {} VM(s) needing attention.", unhealthy.len());
        r.evidence = unhealthy;
        r.recommendations
            .push("Open VMRogue Doctor on the worst offender first.".into());
    }
    r.actions.push(CopilotAction {
        label: "Open Virtual Machines".into(),
        action: "navigate".into(),
        page: Some("vms".into()),
    });
    r
}

pub fn extract_vm_name(query: &str) -> Option<String> {
    let lower = query.to_lowercase();
    for token in query.split_whitespace() {
        let t = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
        if t.len() >= 2
            && ![
                "why", "is", "my", "vm", "the", "not", "starting", "pending", "stuck", "show",
                "all", "can", "migrate", "create", "windows", "linux", "server", "with",
            ]
            .contains(&t.to_lowercase().as_str())
            && lower.contains(&t.to_lowercase())
        {
            return Some(t.to_string());
        }
    }
    None
}
