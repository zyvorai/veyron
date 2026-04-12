use super::types::VirtualMachine;

/// Detailed VM status information
#[derive(Debug, Clone)]
pub struct VMStatus {
    pub name: String,
    pub namespace: String,
    pub running: bool,
    pub ready: bool,
    pub phase: String,
    pub cpu_cores: u32,
    pub memory: String,
    pub node: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: Option<String>,
    pub volumes: Vec<String>,
    pub networks: Vec<String>,
    pub conditions: Vec<StatusCondition>,
}

#[derive(Debug, Clone)]
pub struct StatusCondition {
    pub condition_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
}

impl VMStatus {
    pub fn from_vm(vm: &VirtualMachine) -> Self {
        let name = vm.metadata.name.clone().unwrap_or_default();
        let namespace = vm.metadata.namespace.clone().unwrap_or_default();
        let running = vm.spec.running.unwrap_or(false);

        let (ready, phase) = if let Some(status) = &vm.status {
            let ready = status.ready.unwrap_or(false);
            let phase = status
                .printable_status
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            (ready, phase)
        } else {
            (false, "Unknown".to_string())
        };

        let cpu_cores = vm
            .spec
            .template
            .spec
            .domain
            .cpu
            .as_ref()
            .and_then(|c| c.cores)
            .unwrap_or(0);

        let memory = vm
            .spec
            .template
            .spec
            .domain
            .memory
            .as_ref()
            .and_then(|m| m.guest.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let volumes = vm
            .spec
            .template
            .spec
            .volumes
            .as_ref()
            .map(|vols| vols.iter().map(|v| v.name.clone()).collect())
            .unwrap_or_default();

        let networks = vm
            .spec
            .template
            .spec
            .networks
            .as_ref()
            .map(|nets| nets.iter().map(|n| n.name.clone()).collect())
            .unwrap_or_default();

        let conditions = if let Some(status) = &vm.status {
            status
                .conditions
                .as_ref()
                .map(|conds| {
                    conds
                        .iter()
                        .map(|c| StatusCondition {
                            condition_type: c.type_.clone(),
                            status: c.status.clone(),
                            reason: c.reason.clone(),
                            message: c.message.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let created_at = vm
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|ts| ts.0.format("%Y-%m-%d %H:%M:%S UTC").to_string());

        Self {
            name,
            namespace,
            running,
            ready,
            phase,
            cpu_cores,
            memory,
            node: None, // Would need VMI for this
            ip_address: None,
            created_at,
            volumes,
            networks,
            conditions,
        }
    }

    /// Create a VMStatus from a VM with an optional IP address from VMI
    pub fn from_vm_with_ip(vm: &VirtualMachine, ip: Option<String>) -> Self {
        let mut status = Self::from_vm(vm);
        status.ip_address = ip;
        status
    }

    pub fn display(&self) {
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!(
            "║              VM Status: {}                    ",
            self.name
        );
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Basic Information:");
        println!("  Name:       {}", self.name);
        println!("  Namespace:  {}", self.namespace);
        println!(
            "  Running:    {}",
            if self.running { "Yes ✓" } else { "No ✗" }
        );
        println!(
            "  Ready:      {}",
            if self.ready { "Yes ✓" } else { "No ✗" }
        );
        println!("  Phase:      {}", self.phase);

        if let Some(created) = &self.created_at {
            println!("  Created:    {}", created);
        }

        if let Some(node) = &self.node {
            println!("  Node:       {}", node);
        }

        if let Some(ip) = &self.ip_address {
            println!("  IP:         {}", ip);
        }

        println!();
        println!("Resources:");
        println!("  CPU Cores:  {}", self.cpu_cores);
        println!("  Memory:     {}", self.memory);

        if !self.volumes.is_empty() {
            println!();
            println!("Volumes ({}):", self.volumes.len());
            for vol in &self.volumes {
                println!("  • {}", vol);
            }
        }

        if !self.networks.is_empty() {
            println!();
            println!("Networks ({}):", self.networks.len());
            for net in &self.networks {
                println!("  • {}", net);
            }
        }

        if !self.conditions.is_empty() {
            println!();
            println!("Conditions:");
            for cond in &self.conditions {
                let status_icon = if cond.status == "True" { "✓" } else { "✗" };
                println!(
                    "  {} {} - {}",
                    status_icon, cond.condition_type, cond.status
                );
                if let Some(reason) = &cond.reason {
                    println!("      Reason: {}", reason);
                }
                if let Some(message) = &cond.message {
                    println!("      Message: {}", message);
                }
            }
        }
    }
}

/// Resource summary across multiple VMs
#[derive(Debug, Default)]
pub struct ResourceSummary {
    pub total_vms: usize,
    pub running_vms: usize,
    pub stopped_vms: usize,
    pub total_cpu_cores: u32,
    pub total_memory_gi: f64,
}

impl ResourceSummary {
    pub fn from_vms(vms: &[VirtualMachine]) -> Self {
        let mut running_vms = 0;
        let mut stopped_vms = 0;
        let mut total_cpu_cores = 0u32;
        let mut total_memory_gi = 0.0f64;

        for vm in vms {
            if vm.spec.running.unwrap_or(false) {
                running_vms += 1;
            } else {
                stopped_vms += 1;
            }

            if let Some(cpu) = &vm.spec.template.spec.domain.cpu {
                let cores = cpu.cores.unwrap_or(1);
                let sockets = cpu.sockets.unwrap_or(1);
                let threads = cpu.threads.unwrap_or(1);
                total_cpu_cores += cores * sockets * threads;
            }

            if let Some(memory) = &vm.spec.template.spec.domain.memory {
                if let Some(mem_str) = &memory.guest {
                    if let Some(mem_gi) = parse_memory_to_gi(mem_str) {
                        total_memory_gi += mem_gi;
                    }
                }
            }
        }

        Self {
            total_vms: vms.len(),
            running_vms,
            stopped_vms,
            total_cpu_cores,
            total_memory_gi,
        }
    }

    pub fn display(&self) {
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║                   Resource Summary                            ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("VMs:");
        println!("  Total:      {}", self.total_vms);
        println!("  Running:    {} ✓", self.running_vms);
        println!("  Stopped:    {} ✗", self.stopped_vms);
        println!();
        println!("Resources (Requested):");
        println!("  Total CPU:    {} cores", self.total_cpu_cores);
        println!("  Total Memory: {:.2} Gi", self.total_memory_gi);

        if self.total_vms > 0 {
            println!();
            println!("Per VM Average:");
            println!(
                "  CPU:    {:.1} cores",
                self.total_cpu_cores as f64 / self.total_vms as f64
            );
            println!(
                "  Memory: {:.2} Gi",
                self.total_memory_gi / self.total_vms as f64
            );
        }
    }
}

fn parse_memory_to_gi(mem_str: &str) -> Option<f64> {
    let mem_str = mem_str.trim();

    if mem_str.ends_with("Gi") {
        mem_str.trim_end_matches("Gi").parse::<f64>().ok()
    } else if mem_str.ends_with("Mi") {
        mem_str
            .trim_end_matches("Mi")
            .parse::<f64>()
            .ok()
            .map(|m| m / 1024.0)
    } else if mem_str.ends_with("Ti") {
        mem_str
            .trim_end_matches("Ti")
            .parse::<f64>()
            .ok()
            .map(|m| m * 1024.0)
    } else if mem_str.ends_with("G") {
        mem_str.trim_end_matches("G").parse::<f64>().ok().map(|m| m * 1_000_000_000.0 / 1_073_741_824.0)
    } else if mem_str.ends_with("M") {
        mem_str
            .trim_end_matches("M")
            .parse::<f64>()
            .ok()
            .map(|m| m * 1_000_000.0 / 1_073_741_824.0)
    } else if mem_str.ends_with("T") {
        mem_str
            .trim_end_matches("T")
            .parse::<f64>()
            .ok()
            .map(|m| m * 1_000_000_000_000.0 / 1_073_741_824.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_to_gi() {
        assert_eq!(parse_memory_to_gi("4Gi"), Some(4.0));
        assert_eq!(parse_memory_to_gi("2048Mi"), Some(2.0));
        assert_eq!(parse_memory_to_gi("1Ti"), Some(1024.0));
        assert!((parse_memory_to_gi("8G").unwrap() - 7.45).abs() < 0.01);
        assert!((parse_memory_to_gi("512M").unwrap() - 0.477).abs() < 0.01);
        assert!(parse_memory_to_gi("invalid").is_none());
    }
}
