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
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub cpus: Option<u32>,
    #[serde(default)]
    pub memory: Option<String>,
    /// `propose_bulk_vm_action`: search query (`status:failed ns:prod web`) selecting the VMs.
    #[serde(default)]
    pub query: Option<String>,
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
            "search_resources",
            "Search VMs, pods, nodes and PVCs. Query supports words plus filters \
             kind:vm|pod|node|pvc, ns:<namespace>, status:<text>, os:<text>",
            json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Search query, e.g. 'kind:vm status:failed ns:prod'"},
                    "namespace": {"type": "string"}
                },
                "required": ["message"]
            }),
        ),
        tool(
            "propose_vm_action",
            "PROPOSE (never executes) a change to one or more VMs for the user to approve. \
             The user must approve in the console; do not claim it was performed.",
            json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["start", "stop", "restart", "pause", "unpause", "migrate", "delete", "snapshot", "heal"]},
                    "namespace": {"type": "string"},
                    "names": {"type": "array", "items": {"type": "string"}, "description": "VM names (max 25)"}
                },
                "required": ["action", "namespace", "names"]
            }),
        ),
        tool(
            "propose_vm_resize",
            "PROPOSE (never executes) a live resize: grow CPU sockets and/or guest memory of one or \
             more VMs. Grow-only and bounded by the VM's configured maximums; the user must approve.",
            json!({
                "type": "object",
                "properties": {
                    "namespace": {"type": "string"},
                    "names": {"type": "array", "items": {"type": "string"}, "description": "VM names (max 25)"},
                    "cpus": {"type": "integer", "description": "New CPU socket count"},
                    "memory": {"type": "string", "description": "New guest memory, e.g. 4Gi or 512Mi"}
                },
                "required": ["namespace", "names"]
            }),
        ),
        tool(
            "propose_bulk_vm_action",
            "PROPOSE (never executes) one action over every VM matching a search query, across \
             namespaces, e.g. restart all Failed VMs in prod. The query uses the search syntax \
             (status:, ns:, os:, free words) and must not be empty. At most 25 VMs; larger plans \
             make the user type the VM count to confirm. For action 'hotplug' pass cpus and/or memory.",
            json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["start", "stop", "restart", "pause", "unpause", "migrate", "delete", "snapshot", "heal", "hotplug"]},
                    "query": {"type": "string", "description": "Search query selecting the VMs, e.g. 'status:failed ns:prod'"},
                    "cpus": {"type": "integer", "description": "hotplug only: new CPU socket count"},
                    "memory": {"type": "string", "description": "hotplug only: new guest memory, e.g. 4Gi"}
                },
                "required": ["action", "query"]
            }),
        ),
        tool(
            "propose_node_action",
            "PROPOSE (never executes) cordoning/uncordoning nodes; needs admin approval.",
            json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["cordon", "uncordon"]},
                    "names": {"type": "array", "items": {"type": "string"}, "description": "Node names"}
                },
                "required": ["action", "names"]
            }),
        ),
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
    if tool_name == "search_resources" {
        return Some(search_tool(args, client, scope, fallback_query).await);
    }
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

async fn search_tool(
    args: &ToolInvokeArgs,
    client: &KubeClient,
    scope: &str,
    fallback_query: &str,
) -> CopilotResponse {
    let raw = args.message.as_deref().unwrap_or(fallback_query);
    let q = super::search::parse_query(raw);
    let scope = q
        .namespace
        .clone()
        .or_else(|| args.namespace.clone())
        .unwrap_or_else(|| scope.to_string());
    let hits = super::search::search(client, &scope, &q, super::search::DEFAULT_LIMIT).await;
    let mut r = CopilotResponse::new(
        "Veyron Search",
        "search_resources",
        format!("Search: {raw}"),
    );
    r.summary = if hits.is_empty() {
        format!("No resources matched '{raw}' in scope '{scope}'.")
    } else {
        format!(
            "{} resource(s) matched '{raw}' in scope '{scope}'.",
            hits.len()
        )
    };
    r.evidence = hits
        .iter()
        .map(|h| format!("{} {} — {}", h.kind, h.id, h.status))
        .collect();
    r
}

/// VMs selected by a bulk query as `(namespace, name)`. The query is resolved once, now; the
/// plan stores the concrete list, so approving it later never expands the target set.
#[cfg(feature = "web")]
async fn resolve_bulk_targets(
    client: &KubeClient,
    scope: &str,
    query: Option<&str>,
) -> Result<Vec<(String, String)>, String> {
    use super::{plan, search};
    let raw = query.unwrap_or("").trim();
    let mut q = search::parse_query(raw);
    if q.words.is_empty() && q.namespace.is_none() && q.status.is_none() && q.os.is_none() {
        return Err(
            "a bulk action needs a query that narrows the VMs (status:, ns:, os: or words)".into(),
        );
    }
    q.kinds = vec!["vm".to_string()];
    let scope = q.namespace.clone().unwrap_or_else(|| scope.to_string());
    let hits = search::search(client, &scope, &q, plan::MAX_STEPS + 1).await;
    if hits.is_empty() {
        return Err(format!("no VMs match '{raw}'"));
    }
    if hits.len() > plan::MAX_STEPS {
        return Err(format!(
            "'{raw}' matches more than {} VMs, the most one plan can hold; narrow the query",
            plan::MAX_STEPS
        ));
    }
    Ok(hits
        .into_iter()
        .filter_map(|h| {
            h.id.split_once('/')
                .map(|(n, v)| (n.to_string(), v.to_string()))
        })
        .collect())
}

/// The agent-facing description of a bulk plan, led by its blast radius.
#[cfg(feature = "web")]
fn bulk_response(drafted: &super::plan::Plan, query: &str) -> CopilotResponse {
    let n = drafted.steps.len();
    let namespaces: std::collections::BTreeSet<&str> =
        drafted.steps.iter().map(|s| s.namespace.as_str()).collect();
    let blocked = drafted.steps.iter().filter(|s| s.blocked.is_some()).count();
    let mut r = CopilotResponse::new(
        "Veyron Agent",
        "propose_action",
        format!("Proposed bulk plan {}", drafted.id),
    );
    r.summary = format!(
        "Plan {} drafted for '{query}': blast radius {n} VM(s) in {} namespace(s), needs {} role. {}Awaiting the user's approval in the console - nothing has been executed.{}",
        drafted.id,
        namespaces.len(),
        drafted.min_role,
        if drafted.confirm_count.is_some() {
            "The user must type the VM count to confirm. "
        } else {
            ""
        },
        if blocked > 0 {
            format!(" {blocked} step(s) are currently blocked.")
        } else {
            String::new()
        }
    );
    r.evidence = drafted
        .steps
        .iter()
        .map(|s| match &s.blocked {
            Some(b) => format!("{} - BLOCKED: {b}", s.preview),
            None => s.preview.clone(),
        })
        .collect();
    r
}

/// Draft a plan from a `propose_*` tool call. Nothing is executed or persisted here.
#[cfg(feature = "web")]
pub async fn propose_tool(
    tool_name: &str,
    args: &ToolInvokeArgs,
    client: &KubeClient,
    scope: &str,
    prompt: &str,
) -> Result<(CopilotResponse, super::plan::Plan), String> {
    use super::plan;
    let action = args.action.clone().unwrap_or_default();
    if tool_name == "propose_bulk_vm_action" {
        let targets = resolve_bulk_targets(client, scope, args.query.as_deref()).await?;
        let mut steps =
            plan::build_target_steps(&action, &targets, args.cpus, args.memory.as_deref())?;
        plan::annotate_blockers(client, &mut steps).await;
        let drafted = plan::new_plan(prompt, "", steps, plan::ttl_secs());
        return Ok((
            bulk_response(&drafted, args.query.as_deref().unwrap_or("")),
            drafted,
        ));
    }
    let (kind, ns) = match tool_name {
        "propose_vm_action" => {
            let ns = args
                .namespace
                .clone()
                .or_else(|| (scope != "all").then(|| scope.to_string()))
                .ok_or("namespace is required to propose a VM action")?;
            ("vm", ns)
        }
        "propose_vm_resize" => {
            let ns = args
                .namespace
                .clone()
                .or_else(|| (scope != "all").then(|| scope.to_string()))
                .ok_or("namespace is required to propose a resize")?;
            ("resize", ns)
        }
        "propose_node_action" => ("node", String::new()),
        _ => return Err(format!("unknown proposal tool '{tool_name}'")),
    };
    let mut steps = if kind == "resize" {
        plan::build_hotplug_steps(&ns, &args.names, args.cpus, args.memory.as_deref())?
    } else {
        plan::build_steps(kind, &action, &ns, &args.names)?
    };
    plan::annotate_blockers(client, &mut steps).await;
    let drafted = plan::new_plan(prompt, "", steps, plan::ttl_secs());
    let mut r = CopilotResponse::new(
        "Veyron Agent",
        "propose_action",
        format!("Proposed plan {}", drafted.id),
    );
    let blocked = drafted.steps.iter().filter(|s| s.blocked.is_some()).count();
    r.summary = format!(
        "Plan {} drafted: {} step(s), needs {} role. Awaiting the user's approval in the console — nothing has been executed.{}",
        drafted.id,
        drafted.steps.len(),
        drafted.min_role,
        if blocked > 0 {
            format!(" {blocked} step(s) are currently blocked.")
        } else {
            String::new()
        }
    );
    r.evidence = drafted
        .steps
        .iter()
        .map(|s| match &s.blocked {
            Some(b) => format!("{} — BLOCKED: {b}", s.preview),
            None => s.preview.clone(),
        })
        .collect();
    Ok((r, drafted))
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
            if def.name == "search_resources" || def.name.starts_with("propose_") {
                continue; // handled directly in invoke_tool
            }
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
        assert!(names.iter().any(|n| n == "search_resources"));
        assert!(names.iter().any(|n| n == "propose_vm_action"));
        assert_eq!(names.len(), 32);
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
