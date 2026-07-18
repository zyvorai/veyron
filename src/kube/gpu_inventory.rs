// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! GPU node-inventory helpers: classify device-plugin extended resources
//! (`nvidia.com/gpu`, MIG slices, vGPU mdev profiles) and extract per-node
//! GPU allocatable/capacity from `Node` status. Pure functions — the API
//! handlers (`handlers/gpus.rs`, `handlers/operators.rs`) do the listing.

use k8s_openapi::api::core::v1::Node;
use serde::{Deserialize, Serialize};

/// Vendor extended-resource prefixes that denote a GPU device plugin.
pub const GPU_RESOURCE_PREFIXES: &[&str] = &["nvidia.com/", "amd.com/", "intel.com/"];

/// How a GPU extended resource is carved up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuResourceKind {
    /// Whole-GPU passthrough (`nvidia.com/gpu`, `amd.com/gpu`, or a
    /// PCI-id resource permitted via KubeVirt `permittedHostDevices`).
    Passthrough,
    /// NVIDIA MIG slice (`nvidia.com/mig-3g.20gb`).
    Mig,
    /// vGPU mediated device (`nvidia.com/GRID_T4-2Q` and other mdev names).
    Vgpu,
}

/// One GPU extended resource advertised by a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuResourceEntry {
    pub name: String,
    pub kind: GpuResourceKind,
    pub allocatable: i64,
    pub capacity: i64,
}

/// True when the extended-resource name comes from a GPU vendor device plugin.
/// KVM (`devices.kubevirt.io/kvm`) and other non-GPU extended resources don't match.
pub fn is_gpu_resource(name: &str) -> bool {
    GPU_RESOURCE_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Classify a GPU extended resource by its naming convention:
/// `*/mig-*` ⇒ MIG slice, an uppercase mdev profile (`GRID_*`, `*-2Q`) ⇒ vGPU,
/// anything else (`nvidia.com/gpu`, PCI-model slugs) ⇒ whole-GPU passthrough.
pub fn classify_gpu_resource(name: &str) -> GpuResourceKind {
    let suffix = name.rsplit('/').next().unwrap_or(name);
    if suffix.starts_with("mig-") {
        GpuResourceKind::Mig
    } else if suffix.starts_with("GRID_") || suffix.chars().any(|c| c.is_ascii_uppercase()) {
        // NVIDIA mdev profile names keep their marketing case (GRID_T4-2Q,
        // A100D-40C); device-plugin passthrough resources are lowercase.
        GpuResourceKind::Vgpu
    } else {
        GpuResourceKind::Passthrough
    }
}

/// Extract every GPU extended resource a node advertises.
pub fn node_gpu_resources(node: &Node) -> Vec<GpuResourceEntry> {
    let status = match node.status.as_ref() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let parse = |q: &k8s_openapi::apimachinery::pkg::api::resource::Quantity| {
        q.0.parse::<i64>().unwrap_or(0)
    };
    let allocatable = status.allocatable.as_ref();
    let capacity = status.capacity.as_ref();

    let mut names: Vec<&String> = allocatable
        .into_iter()
        .flat_map(|m| m.keys())
        .chain(capacity.into_iter().flat_map(|m| m.keys()))
        .filter(|k| is_gpu_resource(k))
        .collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| GpuResourceEntry {
            name: name.clone(),
            kind: classify_gpu_resource(name),
            allocatable: allocatable
                .and_then(|m| m.get(name))
                .map(parse)
                .unwrap_or(0),
            capacity: capacity.and_then(|m| m.get(name)).map(parse).unwrap_or(0),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::NodeStatus;
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
    use std::collections::BTreeMap;

    fn node_with_allocatable(entries: &[(&str, &str)]) -> Node {
        let map: BTreeMap<String, Quantity> = entries
            .iter()
            .map(|(k, v)| (k.to_string(), Quantity(v.to_string())))
            .collect();
        Node {
            status: Some(NodeStatus {
                allocatable: Some(map.clone()),
                capacity: Some(map),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn classifies_resource_kinds() {
        assert_eq!(
            classify_gpu_resource("nvidia.com/gpu"),
            GpuResourceKind::Passthrough
        );
        assert_eq!(
            classify_gpu_resource("nvidia.com/rtx4090"),
            GpuResourceKind::Passthrough
        );
        assert_eq!(
            classify_gpu_resource("nvidia.com/mig-3g.20gb"),
            GpuResourceKind::Mig
        );
        assert_eq!(
            classify_gpu_resource("nvidia.com/GRID_T4-2Q"),
            GpuResourceKind::Vgpu
        );
    }

    #[test]
    fn ignores_non_gpu_extended_resources() {
        assert!(!is_gpu_resource("devices.kubevirt.io/kvm"));
        assert!(!is_gpu_resource("cpu"));
        assert!(is_gpu_resource("nvidia.com/gpu"));
        assert!(is_gpu_resource("amd.com/gpu"));
    }

    #[test]
    fn extracts_node_resources() {
        let node = node_with_allocatable(&[
            ("cpu", "16"),
            ("devices.kubevirt.io/kvm", "110"),
            ("nvidia.com/gpu", "2"),
            ("nvidia.com/mig-1g.5gb", "7"),
        ]);
        let res = node_gpu_resources(&node);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].name, "nvidia.com/gpu");
        assert_eq!(res[0].allocatable, 2);
        assert_eq!(res[0].kind, GpuResourceKind::Passthrough);
        assert_eq!(res[1].kind, GpuResourceKind::Mig);
    }

    #[test]
    fn no_status_means_no_resources() {
        assert!(node_gpu_resources(&Node::default()).is_empty());
    }
}
