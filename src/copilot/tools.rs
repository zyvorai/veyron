// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::dispatch::dispatch_intent;
use super::{CopilotAskRequest, CopilotIntent, CopilotResponse};
use crate::kube::KubeClient;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolInvokeArgs {
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub vm_name: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        tool(
            "vm_doctor",
            "Inspect a single VM health, events, and score",
            vm_params(),
        ),
        tool(
            "scheduling_explainer",
            "Explain why a VM is Pending or unschedulable",
            vm_params(),
        ),
        tool(
            "list_unhealthy_vms",
            "List unhealthy or degraded VMs in the workspace",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "backup_advisor",
            "Fleet snapshot schedule and Velero coverage gaps",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "cost_advisor",
            "Fleet or per-VM cost analysis",
            json!({
                "type": "object",
                "properties": {
                    "namespace": {"type": "string"},
                    "vm_name": {"type": "string", "description": "Optional — fleet when omitted"}
                }
            }),
        ),
        tool(
            "network_lens",
            "Per-VM network interfaces, expose, policies",
            vm_params(),
        ),
        tool(
            "guest_inspector",
            "QEMU guest-agent and in-guest OS signals",
            vm_params(),
        ),
        tool(
            "guest_filesystem",
            "In-guest disk usage via guest-exec",
            vm_params(),
        ),
        tool(
            "storage_doctor",
            "PVC pressure and snapshot sprawl",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "security_sentinel",
            "Security exposure, RDP, drift, CVE (fleet when vm_name omitted)",
            json!({
                "type": "object",
                "properties": {
                    "namespace": {"type": "string"},
                    "vm_name": {"type": "string"}
                }
            }),
        ),
        tool(
            "performance_advisor",
            "CPU/memory hotspots (Prometheus p95 when configured)",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "explain_error",
            "Translate a Kubernetes or KubeVirt error message",
            json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Raw error text"}
                },
                "required": ["message"]
            }),
        ),
        tool(
            "quota_advisor",
            "Namespace ResourceQuota pressure",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "catalog_advisor",
            "VMTemplate/VMProfile catalog sync health",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "velero_dr_advisor",
            "Velero backup/restore and DR readiness",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "gitops_advisor",
            "GitOps drift and Argo CD / Flux sync posture",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "forecast_advisor",
            "Capacity and growth forecast from fleet metrics",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "integrations_advisor",
            "Prometheus, Grafana, OpenCost, Trivy integration health",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "compliance_advisor",
            "Compliance framework scores and failing checks",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "observability_advisor",
            "Monitoring stack install and scrape health",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "cilium_advisor",
            "Cilium and Kubernetes network policy posture",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "node_advisor",
            "Node capacity, pressure, and schedulability",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "drift_advisor",
            "VeyronVM operator drift vs desired state",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "alert_advisor",
            "Active warning events and alert noise in scope",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "slo_advisor",
            "Fleet availability SLO and error budget",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "migration_advisor",
            "Live migration status and blockers",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "yaml_preview",
            "Generate KubeVirt VM YAML from a natural-language spec",
            json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "VM spec description"}
                }
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, parameters: Value) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
    }
}

fn vm_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "namespace": {"type": "string"},
            "vm_name": {"type": "string"}
        },
        "required": ["vm_name"]
    })
}

pub fn tool_name_to_intent(name: &str) -> Option<CopilotIntent> {
    Some(match name {
        "vm_doctor" => CopilotIntent::VmDoctor,
        "scheduling_explainer" => CopilotIntent::SchedulingExplainer,
        "list_unhealthy_vms" => CopilotIntent::UnhealthyFleet,
        "backup_advisor" => CopilotIntent::BackupAdvisor,
        "cost_advisor" => CopilotIntent::CostAdvisor,
        "network_lens" => CopilotIntent::NetworkLens,
        "guest_inspector" => CopilotIntent::GuestInspector,
        "guest_filesystem" => CopilotIntent::GuestFilesystem,
        "storage_doctor" => CopilotIntent::StorageDoctor,
        "security_sentinel" => CopilotIntent::SecuritySentinel,
        "performance_advisor" => CopilotIntent::PerformanceAdvisor,
        "explain_error" => CopilotIntent::ErrorExplainer,
        "quota_advisor" => CopilotIntent::QuotaAdvisor,
        "catalog_advisor" => CopilotIntent::CatalogAdvisor,
        "velero_dr_advisor" => CopilotIntent::VeleroDrAdvisor,
        "gitops_advisor" => CopilotIntent::GitopsAdvisor,
        "forecast_advisor" => CopilotIntent::ForecastAdvisor,
        "integrations_advisor" => CopilotIntent::IntegrationsAdvisor,
        "compliance_advisor" => CopilotIntent::ComplianceAdvisor,
        "observability_advisor" => CopilotIntent::ObservabilityAdvisor,
        "cilium_advisor" => CopilotIntent::CiliumAdvisor,
        "node_advisor" => CopilotIntent::NodeAdvisor,
        "drift_advisor" => CopilotIntent::DriftAdvisor,
        "alert_advisor" => CopilotIntent::AlertAdvisor,
        "slo_advisor" => CopilotIntent::SloAdvisor,
        "migration_advisor" => CopilotIntent::MigrationAdvisor,
        "yaml_preview" => CopilotIntent::YamlBuilder,
        _ => return None,
    })
}

pub async fn invoke_tool(
    tool_name: &str,
    args: &ToolInvokeArgs,
    client: &KubeClient,
    scope: &str,
    fallback_query: &str,
) -> Option<CopilotResponse> {
    let intent = tool_name_to_intent(tool_name)?;
    let query = if intent == CopilotIntent::ErrorExplainer || intent == CopilotIntent::YamlBuilder {
        args.message
            .as_deref()
            .unwrap_or(fallback_query)
            .to_string()
    } else {
        fallback_query.to_string()
    };
    let req = CopilotAskRequest {
        query,
        namespace: args.namespace.clone(),
        vm_name: args.vm_name.clone(),
    };
    Some(dispatch_intent(client, scope, intent, &req).await)
}

pub fn parse_tool_args(raw: &str) -> ToolInvokeArgs {
    serde_json::from_str(raw).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_maps_to_intent() {
        for def in tool_definitions() {
            assert!(
                tool_name_to_intent(&def.name).is_some(),
                "missing intent mapping for {}",
                def.name
            );
        }
    }

    #[test]
    fn tool_registry_covers_all_advisor_intents() {
        let names: Vec<String> = tool_definitions().into_iter().map(|t| t.name).collect();
        assert!(names.iter().any(|n| n == "migration_advisor"));
        assert!(names.iter().any(|n| n == "yaml_preview"));
        assert_eq!(names.len(), 27);
    }

    #[test]
    fn parse_tool_args_empty_json() {
        let args = parse_tool_args("{}");
        assert!(args.namespace.is_none());
        assert!(args.vm_name.is_none());
    }

    #[test]
    fn parse_tool_args_message_for_yaml() {
        let args = parse_tool_args(r#"{"message":"Create Ubuntu 22.04 2 CPU 4Gi"}"#);
        assert_eq!(
            args.message.as_deref(),
            Some("Create Ubuntu 22.04 2 CPU 4Gi")
        );
    }
}
