use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod allocation;
pub mod budgets;
pub mod optimization;
pub mod reports;
pub mod waste;

/// Cost metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetric {
    pub resource_id: String,
    pub resource_type: String,
    pub cost: f64,
    pub currency: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub tags: HashMap<String, String>,
}

impl CostMetric {
    pub fn new(
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
        cost: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            resource_id: resource_id.into(),
            resource_type: resource_type.into(),
            cost,
            currency: "USD".to_string(),
            period_start: now,
            period_end: now,
            tags: HashMap::new(),
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

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    pub fn duration_hours(&self) -> f64 {
        (self.period_end - self.period_start).num_seconds() as f64 / 3600.0
    }

    pub fn hourly_cost(&self) -> f64 {
        let hours = self.duration_hours();
        if hours > 0.0 {
            self.cost / hours
        } else {
            0.0
        }
    }
}

/// Cost center
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostCenter {
    pub id: String,
    pub name: String,
    pub department: String,
    pub owner: String,
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl CostCenter {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        department: impl Into<String>,
        owner: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            department: department.into(),
            owner: owner.into(),
            tags: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }
}

/// Cost manager
pub struct CostManager {
    metrics: Vec<CostMetric>,
    cost_centers: HashMap<String, CostCenter>,
}

impl CostManager {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            cost_centers: HashMap::new(),
        }
    }

    pub fn add_metric(&mut self, metric: CostMetric) {
        self.metrics.push(metric);
    }

    pub fn add_cost_center(&mut self, center: CostCenter) -> String {
        let id = center.id.clone();
        self.cost_centers.insert(id.clone(), center);
        id
    }

    pub fn get_cost_center(&self, id: &str) -> Option<&CostCenter> {
        self.cost_centers.get(id)
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    pub fn cost_center_count(&self) -> usize {
        self.cost_centers.len()
    }

    pub fn total_cost(&self) -> f64 {
        self.metrics.iter().map(|m| m.cost).sum()
    }

    pub fn cost_by_resource_type(&self, resource_type: &str) -> f64 {
        self.metrics
            .iter()
            .filter(|m| m.resource_type == resource_type)
            .map(|m| m.cost)
            .sum()
    }

    pub fn cost_by_tag(&self, key: &str, value: &str) -> f64 {
        self.metrics
            .iter()
            .filter(|m| m.tags.get(key).is_some_and(|v| v == value))
            .map(|m| m.cost)
            .sum()
    }

    pub fn metrics_in_period(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&CostMetric> {
        self.metrics
            .iter()
            .filter(|m| m.period_start >= start && m.period_end <= end)
            .collect()
    }

    pub fn top_cost_resources(&self, limit: usize) -> Vec<(&str, f64)> {
        let mut costs: HashMap<&str, f64> = HashMap::new();

        for metric in &self.metrics {
            *costs.entry(metric.resource_id.as_str()).or_insert(0.0) += metric.cost;
        }

        let mut sorted: Vec<_> = costs.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        sorted
    }
}

impl Default for CostManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_metric() {
        let metric = CostMetric::new("vm-123", "compute", 100.0);

        assert_eq!(metric.resource_id, "vm-123");
        assert_eq!(metric.resource_type, "compute");
        assert_eq!(metric.cost, 100.0);
        assert_eq!(metric.currency, "USD");
    }

    #[test]
    fn test_metric_with_currency() {
        let metric = CostMetric::new("vol-456", "storage", 50.0).with_currency("EUR");

        assert_eq!(metric.currency, "EUR");
    }

    #[test]
    fn test_metric_with_period() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::hours(24);

        let metric = CostMetric::new("vm-1", "compute", 120.0).with_period(start, end);

        assert_eq!(metric.period_start, start);
        assert_eq!(metric.period_end, end);
    }

    #[test]
    fn test_metric_add_tag() {
        let mut metric = CostMetric::new("vm-1", "compute", 100.0);

        metric.add_tag("environment", "production");
        metric.add_tag("team", "platform");

        assert_eq!(metric.tags.len(), 2);
        assert_eq!(
            metric.tags.get("environment"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_metric_duration_hours() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::hours(10);

        let metric = CostMetric::new("vm-1", "compute", 100.0).with_period(start, end);

        assert!((metric.duration_hours() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_metric_hourly_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::hours(24);

        let metric = CostMetric::new("vm-1", "compute", 120.0).with_period(start, end);

        assert_eq!(metric.hourly_cost(), 5.0);
    }

    #[test]
    fn test_metric_hourly_cost_zero_duration() {
        let metric = CostMetric::new("vm-1", "compute", 100.0);
        assert_eq!(metric.hourly_cost(), 0.0);
    }

    #[test]
    fn test_cost_center() {
        let center = CostCenter::new("cc-1", "Engineering", "R&D", "john.doe");

        assert_eq!(center.id, "cc-1");
        assert_eq!(center.name, "Engineering");
        assert_eq!(center.department, "R&D");
        assert_eq!(center.owner, "john.doe");
    }

    #[test]
    fn test_cost_center_add_tag() {
        let mut center = CostCenter::new("cc-2", "Marketing", "Sales", "jane.smith");

        center.add_tag("region", "us-west");
        center.add_tag("budget_code", "MKT-2024");

        assert_eq!(center.tags.len(), 2);
        assert_eq!(center.tags.get("region"), Some(&"us-west".to_string()));
    }

    #[test]
    fn test_cost_manager() {
        let mut manager = CostManager::new();

        let metric = CostMetric::new("vm-1", "compute", 100.0);
        manager.add_metric(metric);

        assert_eq!(manager.metric_count(), 1);
    }

    #[test]
    fn test_manager_add_cost_center() {
        let mut manager = CostManager::new();

        let center = CostCenter::new("cc-1", "Engineering", "R&D", "john");
        let id = manager.add_cost_center(center);

        assert_eq!(manager.cost_center_count(), 1);
        assert!(manager.get_cost_center(&id).is_some());
    }

    #[test]
    fn test_manager_total_cost() {
        let mut manager = CostManager::new();

        manager.add_metric(CostMetric::new("vm-1", "compute", 100.0));
        manager.add_metric(CostMetric::new("vm-2", "compute", 150.0));
        manager.add_metric(CostMetric::new("vol-1", "storage", 25.0));

        assert_eq!(manager.total_cost(), 275.0);
    }

    #[test]
    fn test_manager_cost_by_resource_type() {
        let mut manager = CostManager::new();

        manager.add_metric(CostMetric::new("vm-1", "compute", 100.0));
        manager.add_metric(CostMetric::new("vm-2", "compute", 150.0));
        manager.add_metric(CostMetric::new("vol-1", "storage", 25.0));

        assert_eq!(manager.cost_by_resource_type("compute"), 250.0);
        assert_eq!(manager.cost_by_resource_type("storage"), 25.0);
    }

    #[test]
    fn test_manager_cost_by_tag() {
        let mut manager = CostManager::new();

        let mut m1 = CostMetric::new("vm-1", "compute", 100.0);
        m1.add_tag("environment", "production");

        let mut m2 = CostMetric::new("vm-2", "compute", 150.0);
        m2.add_tag("environment", "production");

        let mut m3 = CostMetric::new("vm-3", "compute", 50.0);
        m3.add_tag("environment", "staging");

        manager.add_metric(m1);
        manager.add_metric(m2);
        manager.add_metric(m3);

        assert_eq!(manager.cost_by_tag("environment", "production"), 250.0);
        assert_eq!(manager.cost_by_tag("environment", "staging"), 50.0);
    }

    #[test]
    fn test_manager_metrics_in_period() {
        let mut manager = CostManager::new();

        let start = Utc::now();
        let mid = start + chrono::TimeDelta::hours(12);
        let end = start + chrono::TimeDelta::hours(24);

        let m1 = CostMetric::new("vm-1", "compute", 100.0).with_period(start, mid);
        let m2 = CostMetric::new("vm-2", "compute", 150.0).with_period(mid, end);
        let m3 = CostMetric::new("vm-3", "compute", 75.0).with_period(start, end);

        manager.add_metric(m1);
        manager.add_metric(m2);
        manager.add_metric(m3);

        let in_period = manager.metrics_in_period(start, end);
        assert_eq!(in_period.len(), 3);

        let in_first_half = manager.metrics_in_period(start, mid);
        assert_eq!(in_first_half.len(), 1);
    }

    #[test]
    fn test_manager_top_cost_resources() {
        let mut manager = CostManager::new();

        manager.add_metric(CostMetric::new("vm-1", "compute", 100.0));
        manager.add_metric(CostMetric::new("vm-1", "compute", 50.0)); // Same resource
        manager.add_metric(CostMetric::new("vm-2", "compute", 200.0));
        manager.add_metric(CostMetric::new("vm-3", "compute", 75.0));

        let top = manager.top_cost_resources(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "vm-2");
        assert_eq!(top[0].1, 200.0);
        assert_eq!(top[1].0, "vm-1");
        assert_eq!(top[1].1, 150.0);
    }

    #[test]
    fn test_cost_center_equality() {
        let c1 = CostCenter::new("cc-1", "Engineering", "R&D", "john");
        let c2 = CostCenter::new("cc-1", "Engineering", "R&D", "john");
        let c3 = CostCenter::new("cc-2", "Marketing", "Sales", "jane");

        // Compare fields excluding created_at which will differ
        assert_eq!(c1.id, c2.id);
        assert_eq!(c1.name, c2.name);
        assert_eq!(c1.department, c2.department);
        assert_eq!(c1.owner, c2.owner);

        assert_ne!(c1.id, c3.id);
    }
}
