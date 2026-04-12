use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Allocation method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationMethod {
    DirectTag,
    Proportional,
    EqualSplit,
    UsageBased,
    Custom,
}

/// Allocation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationRule {
    pub id: String,
    pub name: String,
    pub method: AllocationMethod,
    pub source_tag: Option<String>,
    pub target_cost_centers: Vec<String>,
    pub weights: HashMap<String, f64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl AllocationRule {
    pub fn new(name: impl Into<String>, method: AllocationMethod) -> Self {
        let name_str = name.into();
        let id = format!(
            "rule-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            method,
            source_tag: None,
            target_cost_centers: Vec::new(),
            weights: HashMap::new(),
            active: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_source_tag(mut self, tag: impl Into<String>) -> Self {
        self.source_tag = Some(tag.into());
        self
    }

    pub fn add_target(&mut self, cost_center_id: impl Into<String>) {
        self.target_cost_centers.push(cost_center_id.into());
    }

    pub fn set_weight(&mut self, cost_center_id: impl Into<String>, weight: f64) {
        self.weights.insert(cost_center_id.into(), weight);
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }
}

/// Cost allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAllocation {
    pub id: String,
    pub cost_center_id: String,
    pub resource_id: String,
    pub amount: f64,
    pub currency: String,
    pub allocation_method: AllocationMethod,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl CostAllocation {
    pub fn new(
        cost_center_id: impl Into<String>,
        resource_id: impl Into<String>,
        amount: f64,
        method: AllocationMethod,
    ) -> Self {
        let cc_id = cost_center_id.into();
        let res_id = resource_id.into();
        let id = format!(
            "alloc-{}-{}-{}",
            cc_id,
            res_id,
            Utc::now().timestamp_micros()
        );

        let now = Utc::now();
        Self {
            id,
            cost_center_id: cc_id,
            resource_id: res_id,
            amount,
            currency: "USD".to_string(),
            allocation_method: method,
            period_start: now,
            period_end: now,
            metadata: HashMap::new(),
            created_at: now,
        }
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn with_period(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.period_start = start;
        self.period_end = end;
        self
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Showback report entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowbackEntry {
    pub cost_center_id: String,
    pub department: String,
    pub total_cost: f64,
    pub resource_breakdown: HashMap<String, f64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl ShowbackEntry {
    pub fn new(
        cost_center_id: impl Into<String>,
        department: impl Into<String>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        Self {
            cost_center_id: cost_center_id.into(),
            department: department.into(),
            total_cost: 0.0,
            resource_breakdown: HashMap::new(),
            period_start: start,
            period_end: end,
        }
    }

    pub fn add_cost(&mut self, resource_type: impl Into<String>, amount: f64) {
        let resource = resource_type.into();
        *self.resource_breakdown.entry(resource).or_insert(0.0) += amount;
        self.total_cost += amount;
    }

    pub fn resource_count(&self) -> usize {
        self.resource_breakdown.len()
    }
}

/// Allocation manager
pub struct AllocationManager {
    rules: HashMap<String, AllocationRule>,
    allocations: Vec<CostAllocation>,
}

impl AllocationManager {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            allocations: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: AllocationRule) -> String {
        let id = rule.id.clone();
        self.rules.insert(id.clone(), rule);
        id
    }

    pub fn get_rule(&self, id: &str) -> Option<&AllocationRule> {
        self.rules.get(id)
    }

    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut AllocationRule> {
        self.rules.get_mut(id)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn active_rules(&self) -> Vec<&AllocationRule> {
        self.rules.values().filter(|r| r.active).collect()
    }

    pub fn add_allocation(&mut self, allocation: CostAllocation) {
        self.allocations.push(allocation);
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn allocations_by_cost_center(&self, cost_center_id: &str) -> Vec<&CostAllocation> {
        self.allocations
            .iter()
            .filter(|a| a.cost_center_id == cost_center_id)
            .collect()
    }

    pub fn total_allocated_to_cost_center(&self, cost_center_id: &str) -> f64 {
        self.allocations_by_cost_center(cost_center_id)
            .iter()
            .map(|a| a.amount)
            .sum()
    }

    pub fn allocations_by_method(&self, method: &AllocationMethod) -> Vec<&CostAllocation> {
        self.allocations
            .iter()
            .filter(|a| &a.allocation_method == method)
            .collect()
    }

    pub fn generate_showback(
        &self,
        cost_center_id: &str,
        department: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> ShowbackEntry {
        let mut entry = ShowbackEntry::new(cost_center_id, department, start, end);

        for allocation in self.allocations_by_cost_center(cost_center_id) {
            if allocation.period_start >= start && allocation.period_end <= end {
                entry.add_cost(&allocation.resource_id, allocation.amount);
            }
        }

        entry
    }
}

impl Default for AllocationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_rule() {
        let rule = AllocationRule::new("Team Split", AllocationMethod::EqualSplit);

        assert_eq!(rule.name, "Team Split");
        assert_eq!(rule.method, AllocationMethod::EqualSplit);
        assert!(rule.active);
    }

    #[test]
    fn test_rule_with_source_tag() {
        let rule = AllocationRule::new("Tag Based", AllocationMethod::DirectTag)
            .with_source_tag("environment");

        assert_eq!(rule.source_tag, Some("environment".to_string()));
    }

    #[test]
    fn test_rule_add_target() {
        let mut rule = AllocationRule::new("Multi Target", AllocationMethod::Proportional);

        rule.add_target("cc-eng");
        rule.add_target("cc-ops");

        assert_eq!(rule.target_cost_centers.len(), 2);
    }

    #[test]
    fn test_rule_set_weight() {
        let mut rule = AllocationRule::new("Weighted", AllocationMethod::Proportional);

        rule.set_weight("cc-eng", 0.7);
        rule.set_weight("cc-ops", 0.3);

        assert_eq!(rule.weights.len(), 2);
        assert_eq!(rule.weights.get("cc-eng"), Some(&0.7));
    }

    #[test]
    fn test_rule_deactivate_activate() {
        let mut rule = AllocationRule::new("Test", AllocationMethod::EqualSplit);

        assert!(rule.active);

        rule.deactivate();
        assert!(!rule.active);

        rule.activate();
        assert!(rule.active);
    }

    #[test]
    fn test_cost_allocation() {
        let allocation =
            CostAllocation::new("cc-eng", "vm-123", 500.0, AllocationMethod::DirectTag);

        assert_eq!(allocation.cost_center_id, "cc-eng");
        assert_eq!(allocation.resource_id, "vm-123");
        assert_eq!(allocation.amount, 500.0);
        assert_eq!(allocation.allocation_method, AllocationMethod::DirectTag);
        assert_eq!(allocation.currency, "USD");
    }

    #[test]
    fn test_allocation_with_currency() {
        let allocation = CostAllocation::new("cc-1", "vm-1", 100.0, AllocationMethod::EqualSplit)
            .with_currency("EUR");

        assert_eq!(allocation.currency, "EUR");
    }

    #[test]
    fn test_allocation_with_period() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let allocation = CostAllocation::new("cc-1", "vm-1", 100.0, AllocationMethod::UsageBased)
            .with_period(start, end);

        assert_eq!(allocation.period_start, start);
        assert_eq!(allocation.period_end, end);
    }

    #[test]
    fn test_allocation_add_metadata() {
        let mut allocation = CostAllocation::new("cc-1", "vm-1", 100.0, AllocationMethod::Custom);

        allocation.add_metadata("project", "alpha");
        allocation.add_metadata("owner", "john");

        assert_eq!(allocation.metadata.len(), 2);
        assert_eq!(
            allocation.metadata.get("project"),
            Some(&"alpha".to_string())
        );
    }

    #[test]
    fn test_showback_entry() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let entry = ShowbackEntry::new("cc-eng", "Engineering", start, end);

        assert_eq!(entry.cost_center_id, "cc-eng");
        assert_eq!(entry.department, "Engineering");
        assert_eq!(entry.total_cost, 0.0);
    }

    #[test]
    fn test_showback_add_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut entry = ShowbackEntry::new("cc-eng", "Engineering", start, end);

        entry.add_cost("compute", 500.0);
        entry.add_cost("storage", 100.0);
        entry.add_cost("compute", 300.0);

        assert_eq!(entry.total_cost, 900.0);
        assert_eq!(entry.resource_breakdown.get("compute"), Some(&800.0));
        assert_eq!(entry.resource_breakdown.get("storage"), Some(&100.0));
    }

    #[test]
    fn test_showback_resource_count() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut entry = ShowbackEntry::new("cc-eng", "Engineering", start, end);

        entry.add_cost("compute", 500.0);
        entry.add_cost("storage", 100.0);
        entry.add_cost("network", 50.0);

        assert_eq!(entry.resource_count(), 3);
    }

    #[test]
    fn test_allocation_manager() {
        let mut manager = AllocationManager::new();

        let rule = AllocationRule::new("Test", AllocationMethod::EqualSplit);
        let id = manager.add_rule(rule);

        assert_eq!(manager.rule_count(), 1);
        assert!(manager.get_rule(&id).is_some());
    }

    #[test]
    fn test_manager_active_rules() {
        let mut manager = AllocationManager::new();

        let rule1 = AllocationRule::new("R1", AllocationMethod::EqualSplit);
        let mut rule2 = AllocationRule::new("R2", AllocationMethod::Proportional);
        rule2.deactivate();

        manager.add_rule(rule1);
        manager.add_rule(rule2);

        let active = manager.active_rules();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_add_allocation() {
        let mut manager = AllocationManager::new();

        let allocation = CostAllocation::new("cc-1", "vm-1", 100.0, AllocationMethod::DirectTag);
        manager.add_allocation(allocation);

        assert_eq!(manager.allocation_count(), 1);
    }

    #[test]
    fn test_manager_allocations_by_cost_center() {
        let mut manager = AllocationManager::new();

        manager.add_allocation(CostAllocation::new(
            "cc-eng",
            "vm-1",
            100.0,
            AllocationMethod::DirectTag,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-ops",
            "vm-2",
            200.0,
            AllocationMethod::DirectTag,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-eng",
            "vm-3",
            150.0,
            AllocationMethod::DirectTag,
        ));

        let eng_allocations = manager.allocations_by_cost_center("cc-eng");
        assert_eq!(eng_allocations.len(), 2);
    }

    #[test]
    fn test_manager_total_allocated_to_cost_center() {
        let mut manager = AllocationManager::new();

        manager.add_allocation(CostAllocation::new(
            "cc-eng",
            "vm-1",
            100.0,
            AllocationMethod::DirectTag,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-eng",
            "vm-2",
            250.0,
            AllocationMethod::DirectTag,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-ops",
            "vm-3",
            150.0,
            AllocationMethod::DirectTag,
        ));

        assert_eq!(manager.total_allocated_to_cost_center("cc-eng"), 350.0);
        assert_eq!(manager.total_allocated_to_cost_center("cc-ops"), 150.0);
    }

    #[test]
    fn test_manager_allocations_by_method() {
        let mut manager = AllocationManager::new();

        manager.add_allocation(CostAllocation::new(
            "cc-1",
            "vm-1",
            100.0,
            AllocationMethod::DirectTag,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-2",
            "vm-2",
            200.0,
            AllocationMethod::Proportional,
        ));
        manager.add_allocation(CostAllocation::new(
            "cc-3",
            "vm-3",
            150.0,
            AllocationMethod::DirectTag,
        ));

        let direct_tag = manager.allocations_by_method(&AllocationMethod::DirectTag);
        assert_eq!(direct_tag.len(), 2);
    }

    #[test]
    fn test_manager_generate_showback() {
        let mut manager = AllocationManager::new();

        let start = Utc::now();
        let mid = start + chrono::TimeDelta::days(15);
        let end = start + chrono::TimeDelta::days(30);

        let alloc1 = CostAllocation::new("cc-eng", "vm-1", 100.0, AllocationMethod::DirectTag)
            .with_period(start, mid);
        let alloc2 = CostAllocation::new("cc-eng", "vm-2", 200.0, AllocationMethod::DirectTag)
            .with_period(start, mid);

        manager.add_allocation(alloc1);
        manager.add_allocation(alloc2);

        let showback = manager.generate_showback("cc-eng", "Engineering", start, end);

        assert_eq!(showback.total_cost, 300.0);
        assert_eq!(showback.resource_count(), 2);
    }

    #[test]
    fn test_allocation_method_equality() {
        assert_eq!(AllocationMethod::DirectTag, AllocationMethod::DirectTag);
        assert_ne!(AllocationMethod::DirectTag, AllocationMethod::Proportional);
    }
}
