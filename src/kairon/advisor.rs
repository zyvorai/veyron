// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Migration advisor: which VMs to move off a node that is stalling on CPU, memory
//! or IO, and where to. Pure logic over host pressure (PSI) and memory samples; the
//! caller collects the samples from the kairon-node relay.

use serde::Serialize;

/// PSI "some" avg60 (percent of time tasks stalled) above which a node is hot.
pub const CPU_PSI_HOT: f64 = 25.0;
pub const MEM_PSI_HOT: f64 = 10.0;
pub const IO_PSI_HOT: f64 = 30.0;
/// Free memory below this share of total also marks a node hot.
pub const MEM_FREE_HOT: f64 = 0.10;
/// A target must keep this much of its free memory after receiving a VM.
const TARGET_HEADROOM: f64 = 0.85;
const MAX_PER_NODE: usize = 3;

#[derive(Debug, Clone, Default, Serialize)]
pub struct NodeLoad {
    pub name: String,
    pub cpu_psi: f64,
    pub mem_psi: f64,
    pub io_psi: f64,
    pub mem_total_mib: Option<u64>,
    pub mem_available_mib: Option<u64>,
    /// Set when the node could not be sampled; such a node is neither source nor target.
    pub error: Option<String>,
}

impl NodeLoad {
    pub fn hot_reasons(&self) -> Vec<String> {
        let mut r = Vec::new();
        if self.cpu_psi >= CPU_PSI_HOT {
            r.push(format!("CPU stalled {:.0}% of the time", self.cpu_psi));
        }
        if self.mem_psi >= MEM_PSI_HOT {
            r.push(format!("memory stalled {:.0}% of the time", self.mem_psi));
        }
        if self.io_psi >= IO_PSI_HOT {
            r.push(format!("IO stalled {:.0}% of the time", self.io_psi));
        }
        if let (Some(total), Some(avail)) = (self.mem_total_mib, self.mem_available_mib)
            && total > 0
            && (avail as f64 / total as f64) < MEM_FREE_HOT
        {
            r.push(format!("only {avail} MiB of {total} MiB memory free"));
        }
        r
    }

    fn sampled(&self) -> bool {
        self.error.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct VmLoad {
    pub namespace: String,
    pub name: String,
    pub node: String,
    pub memory_mib: u64,
    pub running: bool,
    /// VFIO passthrough: cannot live-migrate.
    pub passthrough: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Advice {
    pub vm: String,
    pub from: String,
    pub to: String,
    pub reason: String,
    /// `POST /api/v1/vms/:ns/:name/migrate` body that carries the advice out.
    pub request: serde_json::Value,
}

/// Recommendations, hottest source node first. At most [`MAX_PER_NODE`] VMs leave a node,
/// largest first, and a target's free memory is reduced as it is promised to a VM so two
/// VMs are never advised onto room that only one fits.
pub fn advise(nodes: &[NodeLoad], vms: &[VmLoad]) -> Vec<Advice> {
    let mut free: Vec<(String, f64)> = nodes
        .iter()
        .filter(|n| n.sampled() && n.hot_reasons().is_empty())
        .map(|n| (n.name.clone(), n.mem_available_mib.unwrap_or(0) as f64))
        .collect();

    let mut hot: Vec<(&NodeLoad, Vec<String>)> = nodes
        .iter()
        .filter(|n| n.sampled())
        .map(|n| (n, n.hot_reasons()))
        .filter(|(_, r)| !r.is_empty())
        .collect();
    hot.sort_by(|a, b| {
        let score = |n: &NodeLoad| n.cpu_psi + n.mem_psi + n.io_psi;
        score(b.0)
            .partial_cmp(&score(a.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = Vec::new();
    for (node, reasons) in hot {
        let mut movable: Vec<&VmLoad> = vms
            .iter()
            .filter(|v| v.node == node.name && v.running && !v.passthrough)
            .collect();
        movable.sort_by_key(|v| std::cmp::Reverse(v.memory_mib));
        for vm in movable.into_iter().take(MAX_PER_NODE) {
            let need = vm.memory_mib as f64 / TARGET_HEADROOM;
            let Some(slot) = free
                .iter_mut()
                .filter(|(_, f)| *f >= need)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            else {
                continue;
            };
            slot.1 -= vm.memory_mib as f64;
            out.push(Advice {
                vm: format!("{}/{}", vm.namespace, vm.name),
                from: node.name.clone(),
                to: slot.0.clone(),
                reason: format!("{} is under pressure: {}", node.name, reasons.join("; ")),
                request: serde_json::json!({ "target_hostname": slot.0, "strategy": "live" }),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, cpu: f64, total: u64, avail: u64) -> NodeLoad {
        NodeLoad {
            name: name.into(),
            cpu_psi: cpu,
            mem_total_mib: Some(total),
            mem_available_mib: Some(avail),
            ..Default::default()
        }
    }

    fn vm(name: &str, node: &str, mib: u64) -> VmLoad {
        VmLoad {
            namespace: "default".into(),
            name: name.into(),
            node: node.into(),
            memory_mib: mib,
            running: true,
            passthrough: false,
        }
    }

    #[test]
    fn moves_the_biggest_vms_from_a_hot_node_to_the_roomiest_one() {
        let nodes = [
            node("a", 60.0, 32_000, 8_000),
            node("b", 2.0, 32_000, 20_000),
            node("c", 1.0, 32_000, 12_000),
        ];
        let vms = [vm("small", "a", 1_000), vm("big", "a", 8_000)];
        let adv = advise(&nodes, &vms);
        assert_eq!(adv.len(), 2);
        assert_eq!(adv[0].vm, "default/big");
        assert_eq!(adv[0].to, "b");
        assert_eq!(adv[0].request["target_hostname"], "b");
        assert!(adv[0].reason.contains("CPU stalled 60%"));
    }

    #[test]
    fn never_overbooks_a_target() {
        let nodes = [
            node("a", 60.0, 32_000, 8_000),
            node("b", 1.0, 32_000, 10_000),
        ];
        let vms = [vm("x", "a", 6_000), vm("y", "a", 6_000)];
        let adv = advise(&nodes, &vms);
        assert_eq!(adv.len(), 1, "room for only one 6 GiB VM");
    }

    #[test]
    fn skips_passthrough_stopped_and_unsampled_nodes() {
        let mut broken = node("c", 0.0, 32_000, 30_000);
        broken.error = Some("relay timeout".into());
        let nodes = [
            node("a", 60.0, 32_000, 8_000),
            broken,
            node("b", 1.0, 32_000, 20_000),
        ];
        let mut gpu = vm("gpu", "a", 2_000);
        gpu.passthrough = true;
        let mut off = vm("off", "a", 2_000);
        off.running = false;
        assert!(advise(&nodes, &[gpu, off]).is_empty());
        let adv = advise(&nodes, &[vm("ok", "a", 1_000)]);
        assert_eq!(adv[0].to, "b", "an unsampled node is never a target");
    }

    #[test]
    fn healthy_cluster_gets_no_advice_and_low_memory_alone_is_hot() {
        let calm = [
            node("a", 1.0, 32_000, 16_000),
            node("b", 2.0, 32_000, 16_000),
        ];
        assert!(advise(&calm, &[vm("v", "a", 1_000)]).is_empty());
        let tight = node("a", 0.0, 32_000, 1_000);
        assert_eq!(tight.hot_reasons().len(), 1);
    }

    #[test]
    fn caps_the_number_of_vms_moved_per_node() {
        let nodes = [
            node("a", 90.0, 64_000, 4_000),
            node("b", 0.0, 256_000, 200_000),
        ];
        let vms: Vec<VmLoad> = (0..6).map(|i| vm(&format!("v{i}"), "a", 1_000)).collect();
        assert_eq!(advise(&nodes, &vms).len(), 3);
    }
}
