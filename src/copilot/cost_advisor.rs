// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use std::collections::HashMap;

use super::{CopilotAction, CopilotResponse};
use crate::kube::KubeClient;
use crate::kube::types::VirtualMachine;

const CPU_RATE: f64 = 0.048;
const MEMORY_RATE: f64 = 0.006;
const STORAGE_RATE: f64 = 0.0001;
const NETWORK_RATE_PER_VM: f64 = 0.50;
const HOURS_PER_MONTH: f64 = 730.0;

#[derive(Debug, Clone)]
struct VmCostRow {
    namespace: String,
    vm_name: String,
    cpu_cost: f64,
    memory_cost: f64,
    storage_cost: f64,
    network_cost: f64,
    total_cost: f64,
}

/// Fleet cost posture: OpenCost allocation when configured, else reference rates.
pub async fn cost_advisor(client: &KubeClient, scope: &str) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Cost Advisor",
        "cost_advisor",
        "Cost analysis in workspace",
    );

    let vms = client.list_vms_for_scope(scope).await;
    if vms.is_empty() {
        r.summary = "No VirtualMachines in the active workspace scope.".into();
        r.actions = vec![CopilotAction {
            label: "Open Costs".into(),
            action: "navigate".into(),
            page: Some("costs".into()),
        }];
        return r;
    }

    let (rows, pricing_model) = resolve_vm_costs(client, &vms).await;
    let total: f64 = rows.iter().map(|x| x.total_cost).sum();
    let mut by_ns: HashMap<String, f64> = HashMap::new();
    for row in &rows {
        *by_ns.entry(row.namespace.clone()).or_default() += row.total_cost;
    }

    r.evidence.push(format!("VMs in scope: {}", rows.len()));
    r.evidence
        .push(format!("Estimated monthly total: ${total:.2} USD ({pricing_model})"));

    let mut top: Vec<_> = rows.iter().collect();
    top.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for row in top.iter().take(6) {
        r.evidence.push(format!(
            "Top spend: {}/{} — ${:.2}/mo",
            row.namespace, row.vm_name, row.total_cost
        ));
    }

    let mut ns_sorted: Vec<_> = by_ns.iter().collect();
    ns_sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (ns, amt) in ns_sorted.iter().take(4) {
        r.evidence.push(format!("Namespace {ns}: ${amt:.2}/mo"));
    }

    if pricing_model == "reference_rates" {
        r.recommendations.push(
            "Set VMROGUE_OPENCOST_URL (and VMROGUE_COST_BACKEND=opencost) for allocation-based costs from the cluster.".into(),
        );
    }

    if rows.len() >= 3 {
        let avg = total / rows.len() as f64;
        for row in top.iter().take(3) {
            if row.total_cost > avg * 2.5 {
                r.recommendations.push(format!(
                    "Review {}/{} — {:.0}% above fleet average; right-size CPU/RAM or add snapshots before changes.",
                    row.namespace,
                    row.vm_name,
                    ((row.total_cost / avg) - 1.0) * 100.0
                ));
            }
        }
    }

    r.summary = if pricing_model == "opencost_allocation" {
        format!(
            "Fleet monthly spend is about ${total:.2} from OpenCost (30d window)."
        )
    } else {
        format!(
            "Fleet monthly estimate is ${total:.2} using VMRogue reference rates (not cloud billing)."
        )
    };

    r.actions = vec![
        CopilotAction {
            label: "Open Costs".into(),
            action: "navigate".into(),
            page: Some("costs".into()),
        },
        CopilotAction {
            label: "Open Forecasting".into(),
            action: "navigate".into(),
            page: Some("forecasting".into()),
        },
    ];
    r
}

/// Per-VM cost breakdown.
pub async fn cost_advisor_vm(
    client: &KubeClient,
    namespace: &str,
    name: &str,
) -> CopilotResponse {
    let mut r = CopilotResponse::new(
        "VMRogue Cost Advisor",
        "cost_advisor_vm",
        format!("Cost view: {namespace}/{name}"),
    );

    let vm = match client.get_vm(namespace, name).await {
        Ok(v) => v,
        Err(e) => {
            r.summary = format!("Could not load VirtualMachine: {e}");
            return r;
        }
    };

    let (rows, pricing_model) = resolve_vm_costs(client, std::slice::from_ref(&vm)).await;
    let Some(row) = rows.into_iter().find(|x| x.vm_name == name) else {
        r.summary = format!("No cost row for {name}.");
        return r;
    };

    r.evidence.push(format!("Pricing model: {pricing_model}"));
    if pricing_model == "reference_rates" {
        r.evidence.push(format!("CPU: ${:.2}/mo", row.cpu_cost));
        r.evidence.push(format!("Memory: ${:.2}/mo", row.memory_cost));
        r.evidence.push(format!("Storage: ${:.2}/mo", row.storage_cost));
        r.evidence.push(format!("Network (flat): ${:.2}/mo", row.network_cost));
    }
    r.evidence
        .push(format!("Total monthly: ${:.2} USD", row.total_cost));

    r.summary = format!("{name} estimated at ${:.2}/mo ({pricing_model}).", row.total_cost);
    if pricing_model == "reference_rates" {
        r.recommendations.push(
            "Reference rates are illustrative — configure OpenCost for real allocation.".into(),
        );
    }

    r.actions = vec![
        CopilotAction {
            label: "Open VM detail".into(),
            action: "open_vm".into(),
            page: Some(format!("{namespace}:{name}")),
        },
        CopilotAction {
            label: "Open Costs".into(),
            action: "navigate".into(),
            page: Some("costs".into()),
        },
    ];
    r
}

async fn resolve_vm_costs(
    client: &KubeClient,
    vms: &[VirtualMachine],
) -> (Vec<VmCostRow>, &'static str) {
    #[cfg(feature = "web")]
    {
        if crate::api::integrations::cost_backend() == "opencost" {
            if let Ok(oc) =
                crate::api::opencost::vm_costs_from_opencost(&client.client(), vms).await
            {
                if oc.iter().any(|(_, _, c)| *c > 0.0) {
                    let rows = oc
                        .into_iter()
                        .map(|(namespace, vm_name, total_cost)| VmCostRow {
                            namespace,
                            vm_name,
                            cpu_cost: 0.0,
                            memory_cost: 0.0,
                            storage_cost: 0.0,
                            network_cost: 0.0,
                            total_cost: round2(total_cost),
                        })
                        .collect();
                    return (rows, "opencost_allocation");
                }
            }
        }
    }

    (reference_cost_rows(vms), "reference_rates")
}

fn reference_cost_rows(vms: &[VirtualMachine]) -> Vec<VmCostRow> {
    vms.iter().map(reference_cost_row).collect()
}

fn reference_cost_row(vm: &VirtualMachine) -> VmCostRow {
    let name = vm.metadata.name.clone().unwrap_or_default();
    let namespace = vm.metadata.namespace.clone().unwrap_or_default();

    let cpu_cores = vm
        .spec
        .template
        .spec
        .domain
        .cpu
        .as_ref()
        .map(|c| c.cores.unwrap_or(1) as f64)
        .unwrap_or(1.0);

    let memory_str = vm
        .spec
        .template
        .spec
        .domain
        .resources
        .requests
        .as_ref()
        .and_then(|r| r.get("memory"))
        .map(|v| v.as_str())
        .unwrap_or("0");
    let memory_gib = parse_memory_gib(memory_str);

    let cpu_cost = cpu_cores * CPU_RATE * HOURS_PER_MONTH;
    let memory_cost = memory_gib * MEMORY_RATE * HOURS_PER_MONTH;
    let total_disk_gib: f64 = vm
        .spec
        .template
        .spec
        .domain
        .resources
        .requests
        .as_ref()
        .and_then(|r| r.get("storage"))
        .map(|v| parse_memory_gib(v.as_str()))
        .unwrap_or(20.0);
    let storage_cost = total_disk_gib * STORAGE_RATE * HOURS_PER_MONTH;
    let network_cost = NETWORK_RATE_PER_VM;
    let total_cost = cpu_cost + memory_cost + storage_cost + network_cost;

    VmCostRow {
        namespace,
        vm_name: name,
        cpu_cost: round2(cpu_cost),
        memory_cost: round2(memory_cost),
        storage_cost: round2(storage_cost),
        network_cost: round2(network_cost),
        total_cost: round2(total_cost),
    }
}

fn parse_memory_gib(s: &str) -> f64 {
    let s = s.trim();
    if let Some(gi) = s.strip_suffix("Gi") {
        return gi.parse().unwrap_or(0.0);
    }
    if let Some(mi) = s.strip_suffix("Mi") {
        return mi.parse::<f64>().unwrap_or(0.0) / 1024.0;
    }
    if let Some(ki) = s.strip_suffix("Ki") {
        return ki.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0);
    }
    s.parse().unwrap_or(0.0)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_memory_gib_handles_gi() {
        assert!((parse_memory_gib("8Gi") - 8.0).abs() < 0.01);
    }

    #[test]
    fn round2_works() {
        assert_eq!(round2(1.234), 1.23);
    }
}
