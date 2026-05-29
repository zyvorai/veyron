// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! VMRogue Copilot — rule-based VM lifecycle assistant (v1).
//!
//! Composes cluster facts, KubeVirt state, and existing translators into product-shaped
//! responses. Optional external LLM backends can be wired later; v1 is deterministic.

mod backup_advisor;
mod doctor;
pub mod fixit;
mod intent;
mod network_lens;
mod scheduling;
mod yaml_build;

pub use backup_advisor::backup_advisor;
pub use doctor::vm_doctor;
pub use intent::CopilotIntent;
pub use network_lens::network_lens;
pub use scheduling::scheduling_explainer;
pub use yaml_build::{YamlBuildRequest, yaml_preview};

use fixit::explain_error_message;
use crate::kube::KubeClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotAction {
    pub label: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationItem {
    pub check: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotResponse {
    pub product: String,
    pub module: String,
    pub intent: String,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub recommendations: Vec<String>,
    pub actions: Vec<CopilotAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<Vec<ValidationItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaml_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_score: Option<u8>,
}

impl CopilotResponse {
    fn new(module: &str, intent: &str, title: impl Into<String>) -> Self {
        Self {
            product: "VMRogue Copilot".to_string(),
            module: module.to_string(),
            intent: intent.to_string(),
            title: title.into(),
            summary: String::new(),
            evidence: Vec::new(),
            recommendations: Vec::new(),
            actions: Vec::new(),
            validation: None,
            yaml_preview: None,
            health_score: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopilotAskRequest {
    pub query: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub vm_name: Option<String>,
}

/// Route a natural-language question to the best v1 copilot module.
pub async fn copilot_ask(
    client: &KubeClient,
    scope: &str,
    req: &CopilotAskRequest,
) -> CopilotResponse {
    let query = req.query.trim();
    if query.is_empty() {
        return general_help();
    }

    let intent = intent::detect_intent(query);
    match intent {
        CopilotIntent::VmDoctor => {
            if let (Some(ns), Some(name)) = (req.namespace.as_deref(), req.vm_name.as_deref()) {
                vm_doctor(client, ns, name).await
            } else if let Some(name) = extract_vm_name(query) {
                let ns = req.namespace.as_deref().unwrap_or(scope);
                vm_doctor(client, ns, &name).await
            } else {
                let mut r = CopilotResponse::new(
                    "VMRogue Doctor",
                    "vm_doctor",
                    "Which VM should I inspect?",
                );
                r.summary = "Specify a VM name in your question or pass namespace + vm_name.".into();
                r.recommendations.push("Example: Why is vm-db-01 not starting?".into());
                r.actions.push(CopilotAction {
                    label: "Open VMs".into(),
                    action: "navigate".into(),
                    page: Some("vms".into()),
                });
                r
            }
        }
        CopilotIntent::SchedulingExplainer => {
            if let (Some(ns), Some(name)) = (req.namespace.as_deref(), req.vm_name.as_deref()) {
                scheduling_explainer(client, ns, name).await
            } else if let Some(name) = extract_vm_name(query) {
                let ns = req.namespace.as_deref().unwrap_or(scope);
                scheduling_explainer(client, ns, &name).await
            } else {
                let mut r = CopilotResponse::new(
                    "VMRogue Scheduling Explainer",
                    "scheduling",
                    "Which VM is stuck scheduling?",
                );
                r.summary =
                    "Include the VM name — e.g. “Why is payroll-vm pending?”".into();
                r
            }
        }
        CopilotIntent::UnhealthyFleet => list_unhealthy_vms(client, scope).await,
        CopilotIntent::YamlBuilder => {
            let spec = yaml_build::parse_spec_from_query(query);
            yaml_preview(client, scope, spec).await
        }
        CopilotIntent::MigrationAdvisor => migration_advice_hint(),
        CopilotIntent::BackupAdvisor => backup_advisor(client, scope).await,
        CopilotIntent::NetworkLens => {
            if let (Some(ns), Some(name)) = (req.namespace.as_deref(), req.vm_name.as_deref()) {
                network_lens(client, ns, name).await
            } else if let Some(name) = extract_vm_name(query) {
                let ns = req.namespace.as_deref().unwrap_or(scope);
                network_lens(client, ns, &name).await
            } else {
                let mut r = CopilotResponse::new(
                    "VMRogue Network Lens",
                    "network_lens",
                    "Which VM should I inspect?",
                );
                r.summary =
                    "Include a VM name — e.g. “Show network for vm-app-01”.".into();
                r
            }
        }
        CopilotIntent::ErrorExplainer => explain_error_message(query, Some("copilot")),
        CopilotIntent::GeneralHelp => general_help(),
    }
}

fn general_help() -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Copilot",
        "help",
        "AI-native VM operator for KubeVirt",
    );
    r.summary = "Ask about VM health, scheduling, YAML, migration, or paste a Kubernetes error.".into();
    r.evidence = vec![
        "VMRogue Doctor — inspect a VM and get a health score".into(),
        "VMRogue Scheduling Explainer — why a VM is Pending".into(),
        "VMRogue YAML Builder — generate and validate VirtualMachine YAML".into(),
        "VMRogue Migration Advisor — readiness for VMware/OVA/VMDK".into(),
        "VMRogue Backup Advisor — snapshot and Velero coverage gaps".into(),
        "VMRogue Network Lens — interfaces, expose, and policies".into(),
        "VMRogue Error Explainer — translate ugly K8s errors".into(),
    ];
    r.recommendations = vec![
        "Why is my VM not starting?".into(),
        "Show all unhealthy VMs".into(),
        "Create a Windows Server 2022 VM with 8 CPU and 32GB RAM".into(),
        "Can I migrate this VMware VM safely?".into(),
        "Which VMs have no backups?".into(),
        "Show network for vm-app-01".into(),
    ];
    r
}

fn migration_advice_hint() -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Migration Advisor",
        "migration",
        "Migration readiness lives in CloudOS",
    );
    r.summary =
        "Use the Migration Assistant wizard for scan, inline plan, and CDI import tasks.".into();
    r.recommendations = vec![
        "Discover workloads → scan readiness → preview plan → execute CDI import".into(),
        "Ensure VirtIO drivers and UEFI firmware match your target template".into(),
    ];
    r.actions = vec![
        CopilotAction {
            label: "Open Migration Assistant".into(),
            action: "navigate".into(),
            page: Some("migration".into()),
        },
        CopilotAction {
            label: "Template Store".into(),
            action: "navigate".into(),
            page: Some("app-store".into()),
        },
    ];
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
            let (score, label) = crate::copilot::fixit::vm_health_score(status, false);
            if score < 65 {
                unhealthy.push(format!("{ns}/{name} — {label} ({score}%)"));
            }
        }
    }
    if unhealthy.is_empty() {
        r.summary = "No clearly unhealthy VMs in the active workspace scope.".into();
        r.evidence.push("Checked printable VM status and fleet health heuristics.".into());
    } else {
        r.summary = format!("Found {} VM(s) needing attention.", unhealthy.len());
        r.evidence = unhealthy;
        r.recommendations.push("Open VMRogue Doctor on the worst offender first.".into());
    }
    r.actions.push(CopilotAction {
        label: "Open Virtual Machines".into(),
        action: "navigate".into(),
        page: Some("vms".into()),
    });
    r
}

fn extract_vm_name(query: &str) -> Option<String> {
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

/// Quick template recommendation from workload description (v1 heuristic).
pub fn recommend_template(description: &str) -> CopilotResponse {
    let lower = description.to_lowercase();
    let mut r = CopilotResponse::new(
        "VMRogue Blueprint Studio",
        "recommend",
        "Recommended VM profile",
    );
    if lower.contains("postgres") || lower.contains("database") || lower.contains("db") {
        r.summary = "Linux VM tuned for PostgreSQL on KubeVirt.".into();
        r.evidence = vec![
            "Template: ubuntu-22.04 or ubuntu-24.04".into(),
            "CPU: 8 vCPU".into(),
            "Memory: 32Gi".into(),
            "Disk: fast SSD PVC (500Gi+)".into(),
            "Network: dedicated NAD / isolated namespace".into(),
            "Backup: every 4h + restore test monthly".into(),
        ];
    } else if lower.contains("accounting") || lower.contains("windows") {
        r.summary = "Windows Server VM for line-of-business apps.".into();
        r.evidence = vec![
            "Template: windows-2022".into(),
            "CPU: 4 vCPU".into(),
            "Memory: 16Gi".into(),
            "Disk: 250Gi".into(),
            "RDP: gateway-only (no public NodePort)".into(),
            "Backup: daily Velero + snapshots".into(),
        ];
    } else {
        r.summary = "General-purpose Linux app server.".into();
        r.evidence = vec![
            "Template: ubuntu-22.04".into(),
            "CPU: 2 vCPU".into(),
            "Memory: 4Gi".into(),
            "Disk: 40Gi".into(),
        ];
    }
    r.recommendations.push("Use YAML Builder to preview manifests with cluster validation.".into());
    r.actions.push(CopilotAction {
        label: "Forge from Template Store".into(),
        action: "navigate".into(),
        page: Some("app-store".into()),
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_doctor_intent() {
        assert!(matches!(
            intent::detect_intent("Why is my VM not starting?"),
            CopilotIntent::VmDoctor
        ));
    }

    #[test]
    fn detect_backup_intent() {
        assert!(matches!(
            intent::detect_intent("Which VMs have no backups?"),
            CopilotIntent::BackupAdvisor
        ));
    }

    #[test]
    fn detect_network_intent() {
        assert!(matches!(
            intent::detect_intent("Show network for vm-app-01"),
            CopilotIntent::NetworkLens
        ));
    }
}
