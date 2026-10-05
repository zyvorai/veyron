// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Blueprint Validation Logic

use super::{Blueprint, VMSpec};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};

/// Validate a blueprint
pub fn validate_blueprint(blueprint: &Blueprint) -> Result<()> {
    // Validate blueprint name
    if blueprint.name.is_empty() {
        return Err(anyhow!("Blueprint name cannot be empty"));
    }

    if blueprint.name.len() > 63 {
        return Err(anyhow!("Blueprint name cannot exceed 63 characters"));
    }

    // Validate VMs list is not empty
    if blueprint.vms.is_empty() {
        return Err(anyhow!("Blueprint must contain at least one VM"));
    }

    // Check for duplicate VM names
    check_duplicate_vm_names(&blueprint.vms)?;

    // Validate dependencies
    validate_dependencies(&blueprint.vms)?;

    // Check for circular dependencies
    check_circular_dependencies(&blueprint.vms)?;

    // Validate each VM spec
    for vm in &blueprint.vms {
        validate_vm_spec(vm)?;
    }

    Ok(())
}

/// Check for duplicate VM names within a blueprint
fn check_duplicate_vm_names(vms: &[VMSpec]) -> Result<()> {
    let mut seen = HashSet::new();

    for vm in vms {
        if !seen.insert(&vm.name) {
            return Err(anyhow!("Duplicate VM name: {}", vm.name));
        }
    }

    Ok(())
}

/// Validate VM spec
fn validate_vm_spec(vm: &VMSpec) -> Result<()> {
    // Validate name
    if vm.name.is_empty() {
        return Err(anyhow!("VM name cannot be empty"));
    }

    // Validate template
    if vm.template.is_empty() {
        return Err(anyhow!("VM template cannot be empty for '{}'", vm.name));
    }

    // Validate CPU if specified
    if let Some(cpu) = vm.cpu {
        if cpu == 0 {
            return Err(anyhow!(
                "VM '{}': CPU cores must be greater than 0",
                vm.name
            ));
        }
        if cpu > 128 {
            return Err(anyhow!("VM '{}': CPU cores cannot exceed 128", vm.name));
        }
    }

    Ok(())
}

/// Validate that all dependency targets exist
fn validate_dependencies(vms: &[VMSpec]) -> Result<()> {
    let vm_names: HashSet<_> = vms.iter().map(|vm| &vm.name).collect();

    for vm in vms {
        for dep in &vm.depends_on {
            if !vm_names.contains(dep) {
                return Err(anyhow!(
                    "VM '{}' depends on '{}' which doesn't exist in blueprint",
                    vm.name,
                    dep
                ));
            }
        }
    }

    Ok(())
}

/// Check for circular dependencies using DFS
pub fn check_circular_dependencies(vms: &[VMSpec]) -> Result<()> {
    let mut graph: HashMap<&String, Vec<&String>> = HashMap::new();

    // Build dependency graph
    for vm in vms {
        graph.insert(&vm.name, vm.depends_on.iter().collect());
    }

    // Check each VM for cycles
    for vm in vms {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        if has_cycle(&vm.name, &graph, &mut visited, &mut rec_stack) {
            return Err(anyhow!(
                "Circular dependency detected involving VM '{}'",
                vm.name
            ));
        }
    }

    Ok(())
}

/// DFS-based cycle detection
fn has_cycle<'a>(
    node: &'a String,
    graph: &HashMap<&'a String, Vec<&'a String>>,
    visited: &mut HashSet<&'a String>,
    rec_stack: &mut HashSet<&'a String>,
) -> bool {
    if rec_stack.contains(node) {
        return true; // Cycle detected
    }

    if visited.contains(node) {
        return false; // Already processed
    }

    visited.insert(node);
    rec_stack.insert(node);

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if has_cycle(neighbor, graph, visited, rec_stack) {
                return true;
            }
        }
    }

    rec_stack.remove(node);
    false
}

/// Get deployment order using topological sort
pub fn resolve_deployment_order(vms: &[VMSpec]) -> Result<Vec<String>> {
    let mut graph: HashMap<&String, Vec<&String>> = HashMap::new();
    let mut in_degree: HashMap<&String, usize> = HashMap::new();

    // Initialize
    for vm in vms {
        graph.insert(&vm.name, vec![]);
        in_degree.insert(&vm.name, 0);
    }

    // Build graph
    for vm in vms {
        for dep in &vm.depends_on {
            if let Some(edges) = graph.get_mut(dep) {
                edges.push(&vm.name);
            } else {
                return Err(anyhow!("Unknown dependency '{}' in VM '{}'", dep, vm.name));
            }
            if let Some(deg) = in_degree.get_mut(&vm.name) {
                *deg += 1;
            }
        }
    }

    // Topological sort using Kahn's algorithm
    let mut queue: Vec<&String> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    if result.len() != vms.len() {
        return Err(anyhow!("Circular dependency detected in blueprint"));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vm(name: &str, depends_on: Vec<String>) -> VMSpec {
        VMSpec {
            name: name.to_string(),
            template: "ubuntu".to_string(),
            profile: None,
            cpu: None,
            memory: None,
            disk_size: None,
            depends_on,
            labels: HashMap::new(),
        }
    }

    #[test]
    fn test_validate_dependencies() {
        let vms = vec![
            create_test_vm("vm1", vec![]),
            create_test_vm("vm2", vec!["vm1".to_string()]),
        ];

        assert!(validate_dependencies(&vms).is_ok());

        // Invalid dependency
        let vms = vec![
            create_test_vm("vm1", vec![]),
            create_test_vm("vm2", vec!["nonexistent".to_string()]),
        ];

        assert!(validate_dependencies(&vms).is_err());
    }

    #[test]
    fn test_circular_dependencies() {
        // Valid: A -> B -> C
        let vms = vec![
            create_test_vm("A", vec![]),
            create_test_vm("B", vec!["A".to_string()]),
            create_test_vm("C", vec!["B".to_string()]),
        ];

        assert!(check_circular_dependencies(&vms).is_ok());

        // Invalid: A -> B -> A (cycle)
        let vms = vec![
            create_test_vm("A", vec!["B".to_string()]),
            create_test_vm("B", vec!["A".to_string()]),
        ];

        assert!(check_circular_dependencies(&vms).is_err());
    }

    #[test]
    fn test_deployment_order() {
        let vms = vec![
            create_test_vm("db", vec![]),
            create_test_vm("cache", vec![]),
            create_test_vm("api", vec!["db".to_string(), "cache".to_string()]),
            create_test_vm("web", vec!["api".to_string()]),
        ];

        let order = resolve_deployment_order(&vms).unwrap();

        // db and cache should come before api
        let db_idx = order.iter().position(|n| n == "db").unwrap();
        let cache_idx = order.iter().position(|n| n == "cache").unwrap();
        let api_idx = order.iter().position(|n| n == "api").unwrap();
        let web_idx = order.iter().position(|n| n == "web").unwrap();

        assert!(db_idx < api_idx);
        assert!(cache_idx < api_idx);
        assert!(api_idx < web_idx);
    }

    #[test]
    fn test_duplicate_vm_names() {
        let vms = vec![
            create_test_vm("vm1", vec![]),
            create_test_vm("vm1", vec![]), // Duplicate
        ];

        assert!(check_duplicate_vm_names(&vms).is_err());
    }
}
