// Topology view rendering helpers

use super::*;

/// Render topology map as ASCII art for terminal display
pub fn render_topology_ascii(map: &TopologyMap) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Cluster: {} | Nodes: {} | VMs: {}",
        map.metadata.cluster_name, map.metadata.total_nodes, map.metadata.total_vms
    ));
    lines.push(String::new());

    for node in &map.nodes {
        let status_icon = match node.status {
            NodeStatus::Ready => "●",
            NodeStatus::NotReady => "○",
            NodeStatus::Unknown => "?",
            NodeStatus::SchedulingDisabled => "⊘",
        };
        let role = match node.role {
            NodeRole::ControlPlane => "control-plane",
            NodeRole::Worker => "worker",
            NodeRole::Edge => "edge",
        };
        lines.push(format!(
            "{} {} [{}] CPU: {:.0}/{:.0} MEM: {:.1}/{:.1}Gi VMs: {}",
            status_icon,
            node.name,
            role,
            node.cpu_usage,
            node.cpu_capacity,
            node.memory_usage,
            node.memory_capacity,
            node.vm_count
        ));

        let vms = map.vms_on_node(&node.name);
        for vm in vms {
            let vm_status = match vm.status {
                VmStatus::Running => "▶",
                VmStatus::Stopped => "■",
                VmStatus::Paused => "⏸",
                VmStatus::Migrating => "↔",
                VmStatus::Unknown => "?",
            };
            lines.push(format!(
                "  {} {}/{} ({}cpu, {}Gi)",
                vm_status, vm.namespace, vm.name, vm.cpu_cores, vm.memory_gb
            ));
        }
        lines.push(String::new());
    }

    lines
}
