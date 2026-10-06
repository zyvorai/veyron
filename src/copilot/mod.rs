// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Veyron Copilot — rule-based VM lifecycle assistant (v1).
//!
//! Composes cluster facts, KubeVirt state, and existing translators into product-shaped
//! responses. Optional external LLM backends can be wired later; v1 is deterministic.

mod agent;
mod alert_advisor;
mod backup_advisor;
mod blueprint_studio;
mod briefing;
mod catalog_advisor;
mod cilium_advisor;
mod compliance_advisor;
mod cost_advisor;
mod dispatch;
mod doctor;
mod drift_advisor;
pub mod fixit;
mod forecast_advisor;
mod gitops_advisor;
mod gitops_generator;
mod guest_filesystem;
mod guest_inspector;
mod integrations_advisor;
mod intent;
mod llm;
mod migration_advisor;
mod network_lens;
mod node_advisor;
mod observability_advisor;
mod performance_advisor;
mod quota_advisor;
mod scheduling;
mod security_sentinel;
mod slo_advisor;
mod storage_doctor;
mod tools;
mod velero_dr_advisor;
mod yaml_build;

pub use agent::copilot_chat;
pub use alert_advisor::alert_advisor;
pub use backup_advisor::backup_advisor;
pub use blueprint_studio::{BlueprintSaveRequest, BlueprintSaveResponse, save_blueprint};
pub use briefing::{CopilotBriefingItem, fleet_briefing};
pub use catalog_advisor::catalog_advisor;
pub use cilium_advisor::cilium_advisor;
pub use compliance_advisor::compliance_advisor;
pub use cost_advisor::{cost_advisor, cost_advisor_vm};
pub use doctor::vm_doctor;
pub use drift_advisor::drift_advisor;
pub use forecast_advisor::forecast_advisor;
pub use gitops_advisor::gitops_advisor;
pub use gitops_generator::{GitOpsExportRequest, GitOpsExportResponse, export_gitops};
pub use guest_filesystem::guest_filesystem_report;
pub use guest_inspector::guest_inspector;
pub use integrations_advisor::integrations_advisor;
pub use intent::CopilotIntent;
pub use llm::{
    AiMode, LlmConfig, ai_mode, ai_rate_limit_per_min, env_llm_config, extra_headers, llm_config,
    runtime_override, set_runtime_override, status_snapshot, timeout_secs,
};
pub use migration_advisor::migration_advisor;
pub use network_lens::network_lens;
pub use node_advisor::node_advisor;
pub use observability_advisor::observability_advisor;
pub use performance_advisor::performance_advisor;
pub use quota_advisor::quota_advisor;
pub use scheduling::{scheduling_explainer, scheduling_fleet_advisor};
pub use security_sentinel::{security_sentinel, security_sentinel_fleet};
pub use slo_advisor::slo_advisor;
pub use storage_doctor::storage_doctor;
pub use tools::{ToolInvokeArgs, invoke_tool, tool_definitions};
pub use velero_dr_advisor::velero_dr_advisor;
pub use yaml_build::{YamlBuildRequest, yaml_preview};

use crate::kube::KubeClient;
use dispatch::dispatch_intent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotChatRequest {
    #[serde(default)]
    pub messages: Vec<CopilotChatMessage>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub vm_name: Option<String>,
}

impl CopilotChatRequest {
    pub fn query(&self) -> String {
        if let Some(q) = &self.query {
            if !q.trim().is_empty() {
                return q.clone();
            }
        }
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotToolTrace {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotChatResponse {
    pub message: CopilotChatMessage,
    pub copilot: CopilotResponse,
    pub tool_trace: Vec<CopilotToolTrace>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotStatusResponse {
    pub llm_configured: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

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
    pub yaml_build: Option<YamlBuildRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_score: Option<u8>,
}

impl CopilotResponse {
    pub(crate) fn new(module: &str, intent: &str, title: impl Into<String>) -> Self {
        Self {
            product: "Veyron Copilot".to_string(),
            module: module.to_string(),
            intent: intent.to_string(),
            title: title.into(),
            summary: String::new(),
            evidence: Vec::new(),
            recommendations: Vec::new(),
            actions: Vec::new(),
            validation: None,
            yaml_preview: None,
            yaml_build: None,
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

    let intent = llm::resolve_intent(query, &[]).await;
    dispatch_intent(client, scope, intent, req).await
}

pub fn general_help() -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "Veyron Copilot",
        "help",
        "AI-native VM operator for KubeVirt",
    );
    r.summary =
        "Ask about VM health, scheduling, YAML, backups, security, or paste a Kubernetes error."
            .into();
    r.evidence = vec![
        "Veyron Doctor — inspect a VM and get a health score".into(),
        "Veyron Scheduling Explainer — why a VM is Pending".into(),
        "Veyron YAML Builder — generate and validate VirtualMachine YAML".into(),
        "Veyron Backup Advisor — snapshot and Velero coverage gaps".into(),
        "Veyron Cost Advisor — fleet spend (OpenCost or reference rates)".into(),
        "Veyron Network Lens — interfaces, expose, and policies".into(),
        "Veyron Guest Inspector — guest agent, OS info, in-guest signals".into(),
        "Veyron Guest Filesystem — in-guest disk usage via guest-exec".into(),
        "Veyron Storage Doctor — PVC pressure and snapshot sprawl".into(),
        "Veyron Security Sentinel — RDP exposure, drift, policies".into(),
        "Veyron Performance Advisor — CPU/memory hotspots".into(),
        "Veyron GitOps Advisor — repos, drift, Argo/Flux".into(),
        "Veyron Forecast Advisor — 30-day capacity heuristic".into(),
        "Veyron Integrations Advisor — optional backend wiring".into(),
        "Veyron Scheduling Fleet — pending VMs and events".into(),
        "Veyron Compliance Advisor — VM hardening and policy gaps".into(),
        "Veyron Observability Advisor — metrics/logs/traces stack".into(),
        "Veyron Cilium Advisor — agents and network policies".into(),
        "Veyron Node Advisor — capacity and node pressure".into(),
        "Veyron Drift Advisor — VeyronVM template drift".into(),
        "Veyron Alert Advisor — warning events narrative".into(),
        "Veyron SLO Advisor — fleet availability SLO".into(),
        "Veyron Migration Advisor — live migration status".into(),
        "Veyron Quota Advisor — ResourceQuota pressure".into(),
        "Veyron Catalog Advisor — template catalog sync".into(),
        "Veyron Velero DR Advisor — backup/restore readiness".into(),
        "Veyron Error Explainer — translate ugly K8s errors".into(),
    ];
    r.recommendations = vec![
        "Why is my VM not starting?".into(),
        "Show all unhealthy VMs".into(),
        "Create a Windows Server 2022 VM with 8 CPU and 32GB RAM".into(),
        "Which VMs have no backups?".into(),
        "Which VMs are most expensive?".into(),
        "Show network for vm-app-01".into(),
        "Show guest agent status for vm-db-01".into(),
        "Show guest filesystem for vm-db-01".into(),
        "Which PVCs are almost full?".into(),
        "Security review for production VMs".into(),
        "Which VMs have high CPU or memory?".into(),
        "GitOps drift in my namespaces".into(),
        "Forecast capacity growth for 30 days".into(),
        "Which integrations are configured?".into(),
        "Which VMs are pending scheduling?".into(),
        "Compliance score for production VMs".into(),
        "Is observability stack installed?".into(),
        "Cilium and network policy posture".into(),
        "Node capacity and pressure".into(),
        "Which VMs have operator drift?".into(),
        "Active warning events in namespace".into(),
        "Fleet availability SLO status".into(),
        "Any live migrations running?".into(),
        "Which namespaces are near quota limits?".into(),
        "Is the template catalog in sync?".into(),
        "Velero DR readiness".into(),
    ];
    r
}

/// Apply optional LLM paraphrase when `VEYRON_AI_URL` is configured.
pub async fn finalize_copilot(resp: CopilotResponse) -> CopilotResponse {
    llm::maybe_enhance_response(resp).await
}

/// Quick template recommendation from workload description (v1 heuristic).
pub fn recommend_template(description: &str) -> CopilotResponse {
    let lower = description.to_lowercase();
    let mut r = CopilotResponse::new(
        "Veyron Blueprint Studio",
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
    r.recommendations
        .push("Use YAML Builder to preview manifests with cluster validation.".into());
    r.actions.push(CopilotAction {
        label: "Forge from Template Foundry".into(),
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

    #[test]
    fn detect_storage_intent() {
        assert!(matches!(
            intent::detect_intent("Which PVCs are almost full?"),
            CopilotIntent::StorageDoctor
        ));
    }

    #[test]
    fn detect_security_intent() {
        assert!(matches!(
            intent::detect_intent("Security review for production VMs"),
            CopilotIntent::SecuritySentinel
        ));
    }

    #[test]
    fn detect_guest_intent() {
        assert!(matches!(
            intent::detect_intent("Show guest agent status for vm-db-01"),
            CopilotIntent::GuestInspector
        ));
    }

    #[test]
    fn detect_guest_filesystem_intent() {
        assert!(matches!(
            intent::detect_intent("Show guest filesystem for vm-db-01"),
            CopilotIntent::GuestFilesystem
        ));
    }

    #[test]
    fn detect_cost_intent() {
        assert!(matches!(
            intent::detect_intent("Which VMs are most expensive?"),
            CopilotIntent::CostAdvisor
        ));
    }

    #[test]
    fn detect_performance_intent() {
        assert!(matches!(
            intent::detect_intent("Which VMs have high CPU or memory?"),
            CopilotIntent::PerformanceAdvisor
        ));
    }

    #[test]
    fn detect_gitops_intent() {
        assert!(matches!(
            intent::detect_intent("GitOps drift in my namespaces"),
            CopilotIntent::GitopsAdvisor
        ));
    }

    #[test]
    fn detect_forecast_intent() {
        assert!(matches!(
            intent::detect_intent("Forecast capacity growth for 30 days"),
            CopilotIntent::ForecastAdvisor
        ));
    }

    #[test]
    fn detect_integrations_intent() {
        assert!(matches!(
            intent::detect_intent("Which integrations are configured?"),
            CopilotIntent::IntegrationsAdvisor
        ));
    }

    #[test]
    fn detect_compliance_intent() {
        assert!(matches!(
            intent::detect_intent("Compliance score for production VMs"),
            CopilotIntent::ComplianceAdvisor
        ));
    }

    #[test]
    fn detect_drift_intent() {
        assert!(matches!(
            intent::detect_intent("Which VMs have operator drift?"),
            CopilotIntent::DriftAdvisor
        ));
    }

    #[test]
    fn detect_migration_intent() {
        assert!(matches!(
            intent::detect_intent("Any live migrations running?"),
            CopilotIntent::MigrationAdvisor
        ));
    }

    #[test]
    fn detect_quota_intent() {
        assert!(matches!(
            intent::detect_intent("Which namespaces are near quota limits?"),
            CopilotIntent::QuotaAdvisor
        ));
    }

    #[test]
    fn detect_catalog_intent() {
        assert!(matches!(
            intent::detect_intent("Is the template catalog in sync?"),
            CopilotIntent::CatalogAdvisor
        ));
    }

    #[test]
    fn detect_velero_dr_intent() {
        assert!(matches!(
            intent::detect_intent("Velero DR readiness"),
            CopilotIntent::VeleroDrAdvisor
        ));
    }
}
