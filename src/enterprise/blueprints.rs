// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
use super::planning::{Capacity, dns_label};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub name: String,
    pub namespace: String,
    pub machines: Vec<BlueprintMachine>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintMachine {
    pub name: String,
    pub resources: Capacity,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub image_ref: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BlueprintPlan {
    pub order: Vec<String>,
    pub total: Capacity,
    pub execution_supported: bool,
    pub warnings: Vec<String>,
}

pub fn validate(bp: &Blueprint) -> Result<BlueprintPlan, String> {
    if !dns_label(&bp.name)
        || !dns_label(&bp.namespace)
        || bp.namespace == "all"
        || bp.machines.is_empty()
        || bp.machines.len() > 100
    {
        return Err("blueprint needs DNS names, one namespace, and 1..100 machines".into());
    }
    let mut graph = BTreeMap::new();
    let mut total = Capacity::default();
    for vm in &bp.machines {
        if !dns_label(&vm.name)
            || graph
                .insert(vm.name.clone(), vm.depends_on.clone())
                .is_some()
        {
            return Err("invalid or duplicate machine name".into());
        }
        if vm.resources.cpu_millis == 0
            || vm.resources.memory_bytes == 0
            || vm.image_ref.trim().is_empty()
        {
            return Err("each machine needs CPU, memory and an image reference".into());
        }
        if vm.depends_on.iter().collect::<BTreeSet<_>>().len() != vm.depends_on.len() {
            return Err("duplicate dependency".into());
        }
        total.cpu_millis = total
            .cpu_millis
            .checked_add(vm.resources.cpu_millis)
            .ok_or("CPU overflow")?;
        total.memory_bytes = total
            .memory_bytes
            .checked_add(vm.resources.memory_bytes)
            .ok_or("memory overflow")?;
        total.gpu = total
            .gpu
            .checked_add(vm.resources.gpu)
            .ok_or("GPU overflow")?;
    }
    if graph
        .iter()
        .any(|(name, deps)| deps.iter().any(|d| d == name || !graph.contains_key(d)))
    {
        return Err("unknown or self dependency".into());
    }
    let mut order = Vec::new();
    let mut completed = BTreeSet::new();
    while order.len() < graph.len() {
        let ready = graph
            .iter()
            .filter(|(n, d)| !completed.contains(*n) && d.iter().all(|dep| completed.contains(dep)))
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err("dependency cycle".into());
        }
        for name in ready {
            completed.insert(name.clone());
            order.push(name);
        }
    }
    Ok(BlueprintPlan{order,total,execution_supported:false,warnings:vec!["Validation plan only; images, application health and storage ownership require cluster verification.".into()]})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn vm(n: &str, deps: Vec<String>) -> BlueprintMachine {
        BlueprintMachine {
            name: n.into(),
            resources: Capacity {
                cpu_millis: 1000,
                memory_bytes: 1024,
                gpu: 0,
            },
            depends_on: deps,
            image_ref: "registry.example/image@sha256:abc".into(),
        }
    }
    #[test]
    fn orders_dependencies_and_sums_resources() {
        let p = validate(&Blueprint {
            name: "stack".into(),
            namespace: "apps".into(),
            machines: vec![vm("web", vec!["db".into()]), vm("db", vec![])],
        })
        .unwrap();
        assert_eq!(p.order, vec!["db", "web"]);
        assert_eq!(p.total.cpu_millis, 2000);
    }
    #[test]
    fn rejects_cycles_and_missing_dependencies() {
        for machines in [
            vec![vm("a", vec!["b".into()]), vm("b", vec!["a".into()])],
            vec![vm("a", vec!["missing".into()])],
        ] {
            assert!(
                validate(&Blueprint {
                    name: "test".into(),
                    namespace: "apps".into(),
                    machines
                })
                .is_err()
            );
        }
    }
}
