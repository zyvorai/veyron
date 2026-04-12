// VM Placement Engine - Optimized VM placement across cluster nodes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PlacementEngine {
    pub nodes: Vec<PlacementNode>,
    pub constraints: Vec<PlacementConstraint>,
    pub config: PlacementConfig,
}

#[derive(Debug, Clone)]
pub struct PlacementNode {
    pub name: String,
    pub available_cpu: f64,
    pub available_memory_gb: f64,
    pub available_storage_gb: f64,
    pub current_vm_count: usize,
    pub max_vms: usize,
    pub zone: String,
    pub labels: HashMap<String, String>,
    pub taints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConstraint {
    pub constraint_type: ConstraintType,
    pub priority: ConstraintPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    Affinity { node_selector: HashMap<String, String> },
    AntiAffinity { avoid_nodes: Vec<String> },
    ResourceRequirement { min_cpu: f64, min_memory_gb: f64 },
    ZoneSpread { zones: Vec<String> },
    MaxVmsPerNode { max: usize },
    TaintToleration { tolerate: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintPriority { Required, Preferred, BestEffort }

#[derive(Debug, Clone)]
pub struct PlacementConfig {
    pub strategy: PlacementStrategy,
    pub rebalance_threshold: f64,
    pub max_migrations_per_rebalance: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementStrategy { BinPacking, Spread, Random, LeastLoaded }

impl Default for PlacementConfig {
    fn default() -> Self {
        Self { strategy: PlacementStrategy::LeastLoaded, rebalance_threshold: 0.3, max_migrations_per_rebalance: 5 }
    }
}

#[derive(Debug, Clone)]
pub struct PlacementDecision {
    pub vm_name: String,
    pub selected_node: String,
    pub score: f64,
    pub reason: String,
    pub alternatives: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
pub struct RebalanceSuggestion {
    pub vm_name: String,
    pub from_node: String,
    pub to_node: String,
    pub improvement_score: f64,
    pub reason: String,
}

impl PlacementEngine {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), constraints: Vec::new(), config: PlacementConfig::default() }
    }

    pub fn add_node(&mut self, node: PlacementNode) { self.nodes.push(node); }
    pub fn add_constraint(&mut self, constraint: PlacementConstraint) { self.constraints.push(constraint); }

    pub fn find_placement(&self, cpu_req: f64, memory_req: f64, storage_req: f64) -> Option<PlacementDecision> {
        let mut candidates: Vec<(usize, f64)> = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            if node.available_cpu < cpu_req || node.available_memory_gb < memory_req || node.available_storage_gb < storage_req {
                continue;
            }
            if node.current_vm_count >= node.max_vms { continue; }

            let score = self.score_node(node, cpu_req, memory_req);
            candidates.push((i, score));
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        candidates.first().map(|&(idx, score)| {
            let node = &self.nodes[idx];
            PlacementDecision {
                vm_name: String::new(),
                selected_node: node.name.clone(),
                score,
                reason: format!("Best fit: {:.1} CPU, {:.1}Gi memory available", node.available_cpu, node.available_memory_gb),
                alternatives: candidates.iter().skip(1).take(3).filter_map(|&(i, s)| {
                    self.nodes.get(i).map(|n| (n.name.clone(), s))
                }).collect(),
            }
        })
    }

    fn score_node(&self, node: &PlacementNode, cpu_req: f64, memory_req: f64) -> f64 {
        match self.config.strategy {
            PlacementStrategy::LeastLoaded => {
                let cpu_free_ratio = node.available_cpu / (node.available_cpu + cpu_req);
                let mem_free_ratio = node.available_memory_gb / (node.available_memory_gb + memory_req);
                (cpu_free_ratio + mem_free_ratio) / 2.0
            }
            PlacementStrategy::BinPacking => {
                let cpu_util = 1.0 - (node.available_cpu / (node.available_cpu + cpu_req));
                let mem_util = 1.0 - (node.available_memory_gb / (node.available_memory_gb + memory_req));
                (cpu_util + mem_util) / 2.0
            }
            PlacementStrategy::Spread => {
                1.0 / (node.current_vm_count as f64 + 1.0)
            }
            PlacementStrategy::Random => rand::random::<f64>(),
        }
    }

    pub fn suggest_rebalance(&self) -> Vec<RebalanceSuggestion> {
        let mut suggestions = Vec::new();
        if self.nodes.len() < 2 { return suggestions; }

        let avg_load: f64 = self.nodes.iter().map(|n| n.current_vm_count as f64).sum::<f64>() / self.nodes.len() as f64;

        for node in &self.nodes {
            if node.current_vm_count as f64 > avg_load * (1.0 + self.config.rebalance_threshold) {
                if let Some(target) = self.nodes.iter().find(|n| {
                    n.name != node.name && (n.current_vm_count as f64) < avg_load * (1.0 - self.config.rebalance_threshold)
                }) {
                    suggestions.push(RebalanceSuggestion {
                        vm_name: String::new(),
                        from_node: node.name.clone(),
                        to_node: target.name.clone(),
                        improvement_score: (node.current_vm_count as f64 - avg_load) / avg_load,
                        reason: format!("Node {} has {} VMs (avg: {:.1})", node.name, node.current_vm_count, avg_load),
                    });
                }
            }
        }

        suggestions.truncate(self.config.max_migrations_per_rebalance);
        suggestions
    }
}

impl Default for PlacementEngine {
    fn default() -> Self { Self::new() }
}
