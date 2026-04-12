// Migration Strategy - Node selection and migration planning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node selector for migration target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelector {
    pub criteria: Vec<SelectionCriterion>,
    pub anti_affinity: Vec<AntiAffinityRule>,
}

impl NodeSelector {
    pub fn new() -> Self {
        Self {
            criteria: Vec::new(),
            anti_affinity: Vec::new(),
        }
    }

    pub fn add_criterion(mut self, criterion: SelectionCriterion) -> Self {
        self.criteria.push(criterion);
        self
    }

    pub fn add_anti_affinity(mut self, rule: AntiAffinityRule) -> Self {
        self.anti_affinity.push(rule);
        self
    }

    /// Score a node for migration suitability (0-100)
    pub fn score_node(&self, node: &NodeInfo) -> u8 {
        let mut score = 100u32;

        for criterion in &self.criteria {
            match criterion {
                SelectionCriterion::MinimumMemory(min_mem) => {
                    if node.available_memory < *min_mem {
                        return 0; // Hard requirement
                    }
                }
                SelectionCriterion::MinimumCPU(min_cpu) => {
                    if node.available_cpu < *min_cpu {
                        return 0; // Hard requirement
                    }
                }
                SelectionCriterion::PreferLowLoad => {
                    // Reduce score based on load
                    score = score.saturating_sub((node.cpu_load * 30.0) as u32);
                }
                SelectionCriterion::PreferMoreMemory => {
                    // Increase score for more available memory
                    let mem_bonus = (node.available_memory / 1024) as u32; // GB
                    score = score.saturating_add(mem_bonus.min(20));
                }
                SelectionCriterion::RequireLabel(key, value) => {
                    if !node.labels.get(key).map(|v| v == value).unwrap_or(false) {
                        return 0; // Hard requirement
                    }
                }
            }
        }

        // Apply anti-affinity penalties
        for rule in &self.anti_affinity {
            if self.matches_anti_affinity(rule, node) {
                score = score.saturating_sub(50);
            }
        }

        score.min(100) as u8
    }

    fn matches_anti_affinity(&self, rule: &AntiAffinityRule, node: &NodeInfo) -> bool {
        // Empty selector means no VMs to avoid — anti-affinity does not apply
        if rule.vm_selector.is_empty() {
            return false;
        }

        // For hostname-based anti-affinity, check if the node already hosts VMs
        // matching the selector labels
        if rule.topology_key == "kubernetes.io/hostname" {
            // Only conflict if the node has VMs and the selector labels match node labels
            return node.vm_count > 0
                && rule
                    .vm_selector
                    .iter()
                    .all(|(k, v)| node.labels.get(k).map(|nv| nv == v).unwrap_or(false));
        }

        // For zone/region-based topology, check if the node carries the topology
        // label and already hosts VMs with matching selector labels
        node.labels.contains_key(&rule.topology_key)
            && node.vm_count > 0
            && rule
                .vm_selector
                .iter()
                .all(|(k, v)| node.labels.get(k).map(|nv| nv == v).unwrap_or(false))
    }
}

impl Default for NodeSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Node selection criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionCriterion {
    MinimumMemory(u64),           // Bytes
    MinimumCPU(u32),              // Millicores
    PreferLowLoad,                // Prefer nodes with lower CPU load
    PreferMoreMemory,             // Prefer nodes with more available memory
    RequireLabel(String, String), // Require specific node label
}

/// Anti-affinity rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiAffinityRule {
    pub vm_selector: HashMap<String, String>,
    pub topology_key: String, // e.g., "kubernetes.io/hostname"
}

impl AntiAffinityRule {
    pub fn new(topology_key: impl Into<String>) -> Self {
        Self {
            vm_selector: HashMap::new(),
            topology_key: topology_key.into(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vm_selector.insert(key.into(), value.into());
        self
    }
}

/// Node information for migration planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub available_memory: u64, // Bytes
    pub available_cpu: u32,    // Millicores
    pub cpu_load: f64,         // 0.0 to 1.0
    pub vm_count: usize,
    pub labels: HashMap<String, String>,
    pub taints: Vec<Taint>,
    pub ready: bool,
}

impl NodeInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            available_memory: 0,
            available_cpu: 0,
            cpu_load: 0.0,
            vm_count: 0,
            labels: HashMap::new(),
            taints: Vec::new(),
            ready: true,
        }
    }

    pub fn with_resources(mut self, memory: u64, cpu: u32) -> Self {
        self.available_memory = memory;
        self.available_cpu = cpu;
        self
    }

    pub fn with_load(mut self, load: f64) -> Self {
        self.cpu_load = load;
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn is_suitable(&self, min_memory: u64, min_cpu: u32) -> bool {
        self.ready && self.available_memory >= min_memory && self.available_cpu >= min_cpu
    }
}

/// Node taint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taint {
    pub key: String,
    pub value: String,
    pub effect: TaintEffect,
}

/// Taint effect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaintEffect {
    NoSchedule,
    PreferNoSchedule,
    NoExecute,
}

/// Migration strategy
pub struct MigrationStrategy {
    selector: NodeSelector,
}

impl MigrationStrategy {
    pub fn new(selector: NodeSelector) -> Self {
        Self { selector }
    }

    /// Select best migration target from available nodes
    pub fn select_target(&self, nodes: &[NodeInfo]) -> Option<String> {
        let mut scored_nodes: Vec<(String, u8)> = nodes
            .iter()
            .filter(|n| n.ready)
            .map(|n| (n.name.clone(), self.selector.score_node(n)))
            .filter(|(_, score)| *score > 0)
            .collect();

        scored_nodes.sort_by(|a, b| b.1.cmp(&a.1));

        scored_nodes.first().map(|(name, _)| name.clone())
    }

    /// Rank all suitable nodes
    pub fn rank_nodes(&self, nodes: &[NodeInfo]) -> Vec<(String, u8)> {
        let mut scored: Vec<(String, u8)> = nodes
            .iter()
            .filter(|n| n.ready)
            .map(|n| (n.name.clone(), self.selector.score_node(n)))
            .filter(|(_, score)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_selector() {
        let selector = NodeSelector::new()
            .add_criterion(SelectionCriterion::MinimumMemory(4 * 1024 * 1024 * 1024))
            .add_criterion(SelectionCriterion::MinimumCPU(2000));

        assert_eq!(selector.criteria.len(), 2);
    }

    #[test]
    fn test_node_scoring() {
        let selector = NodeSelector::new()
            .add_criterion(SelectionCriterion::MinimumMemory(4 * 1024 * 1024 * 1024))
            .add_criterion(SelectionCriterion::PreferLowLoad);

        let node1 = NodeInfo::new("node1")
            .with_resources(8 * 1024 * 1024 * 1024, 4000)
            .with_load(0.2);

        let node2 = NodeInfo::new("node2")
            .with_resources(8 * 1024 * 1024 * 1024, 4000)
            .with_load(0.8);

        let score1 = selector.score_node(&node1);
        let score2 = selector.score_node(&node2);

        assert!(score1 > score2); // Lower load = higher score
    }

    #[test]
    fn test_node_info() {
        let node = NodeInfo::new("test-node")
            .with_resources(16 * 1024 * 1024 * 1024, 8000)
            .with_label("zone", "us-east-1a");

        assert_eq!(node.name, "test-node");
        assert_eq!(node.labels.get("zone"), Some(&"us-east-1a".to_string()));
        assert!(node.is_suitable(8 * 1024 * 1024 * 1024, 4000));
    }

    #[test]
    fn test_migration_strategy() {
        let selector = NodeSelector::new()
            .add_criterion(SelectionCriterion::PreferLowLoad)
            .add_criterion(SelectionCriterion::PreferMoreMemory);

        let strategy = MigrationStrategy::new(selector);

        let nodes = vec![
            NodeInfo::new("node1")
                .with_resources(8 * 1024 * 1024 * 1024, 4000)
                .with_load(0.3),
            NodeInfo::new("node2")
                .with_resources(16 * 1024 * 1024 * 1024, 8000)
                .with_load(0.5),
            NodeInfo::new("node3")
                .with_resources(4 * 1024 * 1024 * 1024, 2000)
                .with_load(0.1),
        ];

        let target = strategy.select_target(&nodes);
        assert!(target.is_some());

        let ranked = strategy.rank_nodes(&nodes);
        assert_eq!(ranked.len(), 3);
    }

    #[test]
    fn test_anti_affinity_rule() {
        let rule = AntiAffinityRule::new("kubernetes.io/hostname").with_label("app", "database");

        assert_eq!(rule.topology_key, "kubernetes.io/hostname");
        assert_eq!(rule.vm_selector.get("app"), Some(&"database".to_string()));
    }

    #[test]
    fn test_minimum_requirements() {
        let selector = NodeSelector::new()
            .add_criterion(SelectionCriterion::MinimumMemory(16 * 1024 * 1024 * 1024))
            .add_criterion(SelectionCriterion::MinimumCPU(8000));

        let node_ok = NodeInfo::new("node-ok").with_resources(20 * 1024 * 1024 * 1024, 10000);

        let node_low_mem =
            NodeInfo::new("node-low-mem").with_resources(8 * 1024 * 1024 * 1024, 10000);

        assert!(selector.score_node(&node_ok) > 0);
        assert_eq!(selector.score_node(&node_low_mem), 0);
    }
}
