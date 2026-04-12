// Cost Tracking - Track resource costs over time

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cost entry for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub timestamp: DateTime<Utc>,
    pub vm_name: String,
    pub namespace: String,
    pub cost: f64,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub tags: HashMap<String, String>,
}

impl CostEntry {
    pub fn new(vm_name: impl Into<String>, namespace: impl Into<String>, cost: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            vm_name: vm_name.into(),
            namespace: namespace.into(),
            cost,
            cpu_cores: 0,
            memory_gb: 0,
            storage_gb: 0,
            tags: HashMap::new(),
        }
    }

    pub fn with_resources(mut self, cpu_cores: u32, memory_gb: u32, storage_gb: u32) -> Self {
        self.cpu_cores = cpu_cores;
        self.memory_gb = memory_gb;
        self.storage_gb = storage_gb;
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Cost tracker
pub struct CostTracker {
    entries: Vec<CostEntry>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: CostEntry) {
        self.entries.push(entry);
    }

    pub fn get_entries(&self) -> &[CostEntry] {
        &self.entries
    }

    /// Get total cost for a time period
    pub fn total_cost(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .map(|e| e.cost)
            .sum()
    }

    /// Get costs by VM
    pub fn costs_by_vm(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> HashMap<String, f64> {
        let mut costs = HashMap::new();

        for entry in &self.entries {
            if entry.timestamp >= start && entry.timestamp <= end {
                *costs.entry(entry.vm_name.clone()).or_insert(0.0) += entry.cost;
            }
        }

        costs
    }

    /// Get costs by namespace
    pub fn costs_by_namespace(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> HashMap<String, f64> {
        let mut costs = HashMap::new();

        for entry in &self.entries {
            if entry.timestamp >= start && entry.timestamp <= end {
                *costs.entry(entry.namespace.clone()).or_insert(0.0) += entry.cost;
            }
        }

        costs
    }

    /// Get costs by tag
    pub fn costs_by_tag(
        &self,
        tag_key: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> HashMap<String, f64> {
        let mut costs = HashMap::new();

        for entry in &self.entries {
            if entry.timestamp >= start && entry.timestamp <= end {
                if let Some(tag_value) = entry.tags.get(tag_key) {
                    *costs.entry(tag_value.clone()).or_insert(0.0) += entry.cost;
                }
            }
        }

        costs
    }

    /// Get top N most expensive VMs
    pub fn top_vms(
        &self,
        n: usize,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<(String, f64)> {
        let mut vm_costs: Vec<_> = self.costs_by_vm(start, end).into_iter().collect();
        vm_costs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        vm_costs.truncate(n);
        vm_costs
    }

    /// Calculate cost trend (change compared to previous period)
    pub fn cost_trend(
        &self,
        current_start: DateTime<Utc>,
        current_end: DateTime<Utc>,
    ) -> CostTrend {
        let current_cost = self.total_cost(current_start, current_end);

        let period_duration = current_end.signed_duration_since(current_start);
        let previous_start = current_start - period_duration;
        let previous_end = current_start;
        let previous_cost = self.total_cost(previous_start, previous_end);

        let change = if previous_cost > 0.0 {
            ((current_cost - previous_cost) / previous_cost) * 100.0
        } else {
            0.0
        };

        CostTrend {
            current_cost,
            previous_cost,
            change_percent: change,
            trend_direction: if change > 0.0 {
                TrendDirection::Increasing
            } else if change < 0.0 {
                TrendDirection::Decreasing
            } else {
                TrendDirection::Stable
            },
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrend {
    pub current_cost: f64,
    pub previous_cost: f64,
    pub change_percent: f64,
    pub trend_direction: TrendDirection,
}

impl CostTrend {
    pub fn is_increasing(&self) -> bool {
        matches!(self.trend_direction, TrendDirection::Increasing)
    }

    pub fn is_decreasing(&self) -> bool {
        matches!(self.trend_direction, TrendDirection::Decreasing)
    }
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendDirection::Increasing => write!(f, "Increasing"),
            TrendDirection::Decreasing => write!(f, "Decreasing"),
            TrendDirection::Stable => write!(f, "Stable"),
        }
    }
}

/// Cost allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAllocation {
    pub dimension: String, // "team", "project", "environment", etc.
    pub allocations: HashMap<String, AllocationDetail>,
    pub total_cost: f64,
}

impl CostAllocation {
    pub fn new(dimension: impl Into<String>) -> Self {
        Self {
            dimension: dimension.into(),
            allocations: HashMap::new(),
            total_cost: 0.0,
        }
    }

    pub fn add_cost(&mut self, key: impl Into<String>, cost: f64, vm_count: usize) {
        let key = key.into();
        let detail = self
            .allocations
            .entry(key.clone())
            .or_insert_with(|| AllocationDetail {
                name: key,
                cost: 0.0,
                vm_count: 0,
                percentage: 0.0,
            });

        detail.cost += cost;
        detail.vm_count += vm_count;
        self.total_cost += cost;
    }

    pub fn calculate_percentages(&mut self) {
        for detail in self.allocations.values_mut() {
            if self.total_cost > 0.0 {
                detail.percentage = (detail.cost / self.total_cost) * 100.0;
            } else {
                detail.percentage = 0.0;
            }
        }
    }

    pub fn get_sorted_allocations(&self) -> Vec<&AllocationDetail> {
        let mut allocations: Vec<_> = self.allocations.values().collect();
        allocations.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        allocations
    }
}

/// Allocation detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationDetail {
    pub name: String,
    pub cost: f64,
    pub vm_count: usize,
    pub percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_cost_entry() {
        let entry = CostEntry::new("test-vm", "default", 10.0)
            .with_resources(2, 4, 20)
            .with_tag("team", "engineering");

        assert_eq!(entry.vm_name, "test-vm");
        assert_eq!(entry.cost, 10.0);
        assert_eq!(entry.cpu_cores, 2);
        assert_eq!(entry.tags.get("team"), Some(&"engineering".to_string()));
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();

        let now = Utc::now();
        tracker.add_entry(CostEntry::new("vm1", "default", 50.0));
        tracker.add_entry(CostEntry::new("vm2", "default", 30.0));
        tracker.add_entry(CostEntry::new("vm3", "prod", 20.0));

        assert_eq!(tracker.entry_count(), 3);

        let total = tracker.total_cost(now - Duration::hours(1), now + Duration::hours(1));
        assert_eq!(total, 100.0);
    }

    #[test]
    fn test_costs_by_vm() {
        let mut tracker = CostTracker::new();
        let now = Utc::now();

        tracker.add_entry(CostEntry::new("vm1", "default", 50.0));
        tracker.add_entry(CostEntry::new("vm1", "default", 30.0));
        tracker.add_entry(CostEntry::new("vm2", "default", 20.0));

        let costs = tracker.costs_by_vm(now - Duration::hours(1), now + Duration::hours(1));

        assert_eq!(costs.get("vm1"), Some(&80.0));
        assert_eq!(costs.get("vm2"), Some(&20.0));
    }

    #[test]
    fn test_costs_by_namespace() {
        let mut tracker = CostTracker::new();
        let now = Utc::now();

        tracker.add_entry(CostEntry::new("vm1", "default", 50.0));
        tracker.add_entry(CostEntry::new("vm2", "default", 30.0));
        tracker.add_entry(CostEntry::new("vm3", "prod", 20.0));

        let costs = tracker.costs_by_namespace(now - Duration::hours(1), now + Duration::hours(1));

        assert_eq!(costs.get("default"), Some(&80.0));
        assert_eq!(costs.get("prod"), Some(&20.0));
    }

    #[test]
    fn test_costs_by_tag() {
        let mut tracker = CostTracker::new();
        let now = Utc::now();

        tracker.add_entry(CostEntry::new("vm1", "default", 50.0).with_tag("team", "engineering"));
        tracker.add_entry(CostEntry::new("vm2", "default", 30.0).with_tag("team", "engineering"));
        tracker.add_entry(CostEntry::new("vm3", "prod", 20.0).with_tag("team", "sales"));

        let costs =
            tracker.costs_by_tag("team", now - Duration::hours(1), now + Duration::hours(1));

        assert_eq!(costs.get("engineering"), Some(&80.0));
        assert_eq!(costs.get("sales"), Some(&20.0));
    }

    #[test]
    fn test_top_vms() {
        let mut tracker = CostTracker::new();
        let now = Utc::now();

        tracker.add_entry(CostEntry::new("vm1", "default", 100.0));
        tracker.add_entry(CostEntry::new("vm2", "default", 50.0));
        tracker.add_entry(CostEntry::new("vm3", "default", 75.0));

        let top = tracker.top_vms(2, now - Duration::hours(1), now + Duration::hours(1));

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "vm1");
        assert_eq!(top[0].1, 100.0);
        assert_eq!(top[1].0, "vm3");
    }

    #[test]
    fn test_cost_trend() {
        let mut tracker = CostTracker::new();
        let now = Utc::now();

        // Previous period (now - 4h to now - 2h, which is 2 hours duration)
        let mut entry1 = CostEntry::new("vm1", "default", 50.0);
        entry1.timestamp = now - Duration::hours(3);
        tracker.add_entry(entry1);

        // Current period (now - 2h to now, which is also 2 hours duration)
        let mut entry2 = CostEntry::new("vm1", "default", 75.0);
        entry2.timestamp = now - Duration::hours(1);
        tracker.add_entry(entry2);

        let trend = tracker.cost_trend(now - Duration::hours(2), now);

        assert_eq!(trend.current_cost, 75.0);
        assert_eq!(trend.previous_cost, 50.0);
        assert_eq!(trend.change_percent, 50.0); // 50% increase
        assert!(trend.is_increasing());
    }

    #[test]
    fn test_cost_allocation() {
        let mut allocation = CostAllocation::new("team");

        allocation.add_cost("engineering", 100.0, 5);
        allocation.add_cost("engineering", 50.0, 2);
        allocation.add_cost("sales", 30.0, 1);

        allocation.calculate_percentages();

        assert_eq!(allocation.total_cost, 180.0);

        let eng = allocation.allocations.get("engineering").unwrap();
        assert_eq!(eng.cost, 150.0);
        assert_eq!(eng.vm_count, 7);
        assert!((eng.percentage - 83.33).abs() < 0.1);

        let sales = allocation.allocations.get("sales").unwrap();
        assert_eq!(sales.cost, 30.0);
    }

    #[test]
    fn test_sorted_allocations() {
        let mut allocation = CostAllocation::new("project");

        allocation.add_cost("project-a", 100.0, 5);
        allocation.add_cost("project-b", 200.0, 10);
        allocation.add_cost("project-c", 50.0, 2);

        let sorted = allocation.get_sorted_allocations();

        assert_eq!(sorted[0].name, "project-b");
        assert_eq!(sorted[1].name, "project-a");
        assert_eq!(sorted[2].name, "project-c");
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(TrendDirection::Increasing.to_string(), "Increasing");
        assert_eq!(TrendDirection::Decreasing.to_string(), "Decreasing");
        assert_eq!(TrendDirection::Stable.to_string(), "Stable");
    }
}
