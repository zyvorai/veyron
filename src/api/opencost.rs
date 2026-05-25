//! OpenCost allocation API integration for real cost estimates.

#[cfg(feature = "web")]
use anyhow::{Context, Result};
#[cfg(feature = "web")]
use std::collections::HashMap;

#[cfg(feature = "web")]
use super::integrations;

/// Monthly cost per `(namespace, pod_name)` from OpenCost allocation rows.
#[cfg(feature = "web")]
pub async fn namespace_pod_costs() -> Result<HashMap<(String, String), f64>> {
    let base = integrations::env_var("VMROGUE_OPENCOST_URL")
        .context("VMROGUE_OPENCOST_URL not set")?;
    let url = format!(
        "{}/allocation/compute?window=30d&aggregate=pod,namespace&step=1d",
        base.trim_end_matches('/')
    );
    let v = integrations::get_json(&url, integrations::env_var("VMROGUE_OPENCOST_TOKEN").as_deref())
        .await?;

    let mut out = HashMap::new();
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| v.as_array());

    let Some(rows) = data else {
        return Ok(out);
    };

    for row in rows {
        parse_allocation_row(row, &mut out);
    }
    Ok(out)
}

#[cfg(feature = "web")]
fn parse_allocation_row(row: &serde_json::Value, out: &mut HashMap<(String, String), f64>) {
    if let Some(obj) = row.as_object() {
        for (key, val) in obj {
            if key.contains("pod") || key.contains("namespace") {
                if let Some(inner) = val.as_object() {
                    let ns = inner
                        .get("properties")
                        .and_then(|p| p.get("namespace"))
                        .or_else(|| inner.get("namespace"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let pod = inner
                        .get("properties")
                        .and_then(|p| p.get("pod"))
                        .or_else(|| inner.get("pod"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cost = inner
                        .get("totalCost")
                        .or_else(|| inner.get("totalAmortizedCost"))
                        .and_then(|c| c.as_f64())
                        .unwrap_or(0.0);
                    if !ns.is_empty() && !pod.is_empty() && cost > 0.0 {
                        out.insert((ns, pod), cost);
                    }
                }
            }
            parse_allocation_row(val, out);
        }
    } else if let Some(arr) = row.as_array() {
        for item in arr {
            parse_allocation_row(item, out);
        }
    }
}

/// Map virt-launcher pods to VM names via `kubevirt.io/domain` label on running VMIs.
#[cfg(feature = "web")]
pub async fn vm_costs_from_opencost(
    client: &kube::Client,
    vms: &[crate::kube::types::VirtualMachine],
) -> Result<Vec<(String, String, f64)>> {
    let pod_costs = namespace_pod_costs().await?;
    let mut vm_totals: HashMap<(String, String), f64> = HashMap::new();

    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, api::ListParams};

    let pods: Api<Pod> = Api::all(client.clone());
    let lp = ListParams::default().labels("kubevirt.io/domain");
    let pod_items = match pods.list(&lp).await {
        Ok(list) => list.items,
        Err(_) => Vec::new(),
    };

    for pod in &pod_items {
        let ns = pod.metadata.namespace.clone().unwrap_or_default();
        let pname = pod.metadata.name.clone().unwrap_or_default();
        let domain = pod
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("kubevirt.io/domain"))
            .cloned()
            .unwrap_or_default();
        if let Some(&cost) = pod_costs.get(&(ns.clone(), pname)) {
            *vm_totals.entry((ns, domain)).or_default() += cost;
        }
    }

    let mut results = Vec::new();
    for vm in vms {
        let name = vm.metadata.name.clone().unwrap_or_default();
        let ns = vm.metadata.namespace.clone().unwrap_or_default();
        let cost = vm_totals.get(&(ns.clone(), name.clone())).copied().unwrap_or(0.0);
        results.push((ns, name, cost));
    }
    Ok(results)
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use super::*;

    #[test]
    fn parse_nested_allocation() {
        let mut m = HashMap::new();
        let row = serde_json::json!({
            "0": {
                "properties": { "namespace": "default", "pod": "virt-launcher-vm1" },
                "totalCost": 12.5
            }
        });
        parse_allocation_row(&row, &mut m);
        assert_eq!(
            m.get(&("default".to_string(), "virt-launcher-vm1".to_string())),
            Some(&12.5)
        );
    }
}
