use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Waste type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasteType {
    Idle,
    Underutilized,
    Orphaned,
    Unattached,
    Oversized,
    Duplicate,
    Expired,
}

impl std::fmt::Display for WasteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasteType::Idle => write!(f, "Idle"),
            WasteType::Underutilized => write!(f, "Underutilized"),
            WasteType::Orphaned => write!(f, "Orphaned"),
            WasteType::Unattached => write!(f, "Unattached"),
            WasteType::Oversized => write!(f, "Oversized"),
            WasteType::Duplicate => write!(f, "Duplicate"),
            WasteType::Expired => write!(f, "Expired"),
        }
    }
}

/// Waste severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasteSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Wasteful resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WastefulResource {
    pub id: String,
    pub resource_id: String,
    pub resource_type: String,
    pub waste_type: WasteType,
    pub severity: WasteSeverity,
    pub monthly_waste: f64,
    pub currency: String,
    pub utilization_percent: f64,
    pub age_days: u32,
    pub recommendation: String,
    pub detected_at: DateTime<Utc>,
}

impl WastefulResource {
    pub fn new(
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
        waste_type: WasteType,
        monthly_waste: f64,
    ) -> Self {
        let res_id = resource_id.into();
        let id = format!("waste-{}-{}", res_id, Utc::now().timestamp_micros());

        Self {
            id,
            resource_id: res_id,
            resource_type: resource_type.into(),
            waste_type,
            severity: WasteSeverity::Medium,
            monthly_waste,
            currency: "USD".to_string(),
            utilization_percent: 0.0,
            age_days: 0,
            recommendation: String::new(),
            detected_at: Utc::now(),
        }
    }

    pub fn with_severity(mut self, severity: WasteSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_utilization(mut self, utilization: f64) -> Self {
        self.utilization_percent = utilization.clamp(0.0, 100.0);
        self
    }

    pub fn with_age(mut self, days: u32) -> Self {
        self.age_days = days;
        self
    }

    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.recommendation = recommendation.into();
        self
    }

    pub fn annual_waste(&self) -> f64 {
        self.monthly_waste * 12.0
    }

    pub fn is_critical(&self) -> bool {
        self.severity == WasteSeverity::Critical || self.monthly_waste > 1000.0
    }

    pub fn is_old(&self) -> bool {
        self.age_days > 90
    }
}

/// Waste detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteDetectionRule {
    pub id: String,
    pub name: String,
    pub waste_type: WasteType,
    pub resource_type: String,
    pub threshold_utilization: Option<f64>,
    pub threshold_age_days: Option<u32>,
    pub threshold_cost: Option<f64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl WasteDetectionRule {
    pub fn new(
        name: impl Into<String>,
        waste_type: WasteType,
        resource_type: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = format!(
            "rule-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            waste_type,
            resource_type: resource_type.into(),
            threshold_utilization: None,
            threshold_age_days: None,
            threshold_cost: None,
            active: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_utilization_threshold(mut self, threshold: f64) -> Self {
        self.threshold_utilization = Some(threshold.clamp(0.0, 100.0));
        self
    }

    pub fn with_age_threshold(mut self, days: u32) -> Self {
        self.threshold_age_days = Some(days);
        self
    }

    pub fn with_cost_threshold(mut self, cost: f64) -> Self {
        self.threshold_cost = Some(cost);
        self
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }
}

/// Waste manager
pub struct WasteManager {
    wasteful_resources: HashMap<String, WastefulResource>,
    detection_rules: HashMap<String, WasteDetectionRule>,
}

impl WasteManager {
    pub fn new() -> Self {
        Self {
            wasteful_resources: HashMap::new(),
            detection_rules: HashMap::new(),
        }
    }

    pub fn add_wasteful_resource(&mut self, resource: WastefulResource) -> String {
        let id = resource.id.clone();
        self.wasteful_resources.insert(id.clone(), resource);
        id
    }

    pub fn get_wasteful_resource(&self, id: &str) -> Option<&WastefulResource> {
        self.wasteful_resources.get(id)
    }

    pub fn wasteful_resource_count(&self) -> usize {
        self.wasteful_resources.len()
    }

    pub fn add_detection_rule(&mut self, rule: WasteDetectionRule) -> String {
        let id = rule.id.clone();
        self.detection_rules.insert(id.clone(), rule);
        id
    }

    pub fn get_detection_rule(&self, id: &str) -> Option<&WasteDetectionRule> {
        self.detection_rules.get(id)
    }

    pub fn detection_rule_count(&self) -> usize {
        self.detection_rules.len()
    }

    pub fn active_rules(&self) -> Vec<&WasteDetectionRule> {
        self.detection_rules.values().filter(|r| r.active).collect()
    }

    pub fn by_waste_type(&self, waste_type: &WasteType) -> Vec<&WastefulResource> {
        self.wasteful_resources
            .values()
            .filter(|r| &r.waste_type == waste_type)
            .collect()
    }

    pub fn by_severity(&self, severity: &WasteSeverity) -> Vec<&WastefulResource> {
        self.wasteful_resources
            .values()
            .filter(|r| &r.severity == severity)
            .collect()
    }

    pub fn critical_waste(&self) -> Vec<&WastefulResource> {
        self.wasteful_resources
            .values()
            .filter(|r| r.is_critical())
            .collect()
    }

    pub fn old_resources(&self) -> Vec<&WastefulResource> {
        self.wasteful_resources
            .values()
            .filter(|r| r.is_old())
            .collect()
    }

    pub fn total_monthly_waste(&self) -> f64 {
        self.wasteful_resources
            .values()
            .map(|r| r.monthly_waste)
            .sum()
    }

    pub fn total_annual_waste(&self) -> f64 {
        self.total_monthly_waste() * 12.0
    }

    pub fn by_resource_type(&self, resource_type: &str) -> Vec<&WastefulResource> {
        self.wasteful_resources
            .values()
            .filter(|r| r.resource_type == resource_type)
            .collect()
    }

    pub fn top_wasteful_resources(&self, limit: usize) -> Vec<&WastefulResource> {
        let mut resources: Vec<_> = self.wasteful_resources.values().collect();
        resources.sort_by(|a, b| {
            b.monthly_waste
                .partial_cmp(&a.monthly_waste)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        resources.truncate(limit);
        resources
    }
}

impl Default for WasteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waste_type_display() {
        assert_eq!(WasteType::Idle.to_string(), "Idle");
        assert_eq!(WasteType::Underutilized.to_string(), "Underutilized");
        assert_eq!(WasteType::Orphaned.to_string(), "Orphaned");
    }

    #[test]
    fn test_wasteful_resource() {
        let resource = WastefulResource::new("vm-123", "compute", WasteType::Idle, 150.0);

        assert_eq!(resource.resource_id, "vm-123");
        assert_eq!(resource.resource_type, "compute");
        assert_eq!(resource.waste_type, WasteType::Idle);
        assert_eq!(resource.monthly_waste, 150.0);
        assert_eq!(resource.severity, WasteSeverity::Medium);
    }

    #[test]
    fn test_resource_with_severity() {
        let resource = WastefulResource::new("vm-1", "compute", WasteType::Idle, 200.0)
            .with_severity(WasteSeverity::High);

        assert_eq!(resource.severity, WasteSeverity::High);
    }

    #[test]
    fn test_resource_with_utilization() {
        let resource = WastefulResource::new("vm-1", "compute", WasteType::Underutilized, 100.0)
            .with_utilization(15.0);

        assert_eq!(resource.utilization_percent, 15.0);
    }

    #[test]
    fn test_resource_utilization_clamping() {
        let resource1 = WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0)
            .with_utilization(150.0);
        assert_eq!(resource1.utilization_percent, 100.0);

        let resource2 = WastefulResource::new("vm-2", "compute", WasteType::Idle, 100.0)
            .with_utilization(-10.0);
        assert_eq!(resource2.utilization_percent, 0.0);
    }

    #[test]
    fn test_resource_with_age() {
        let resource =
            WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0).with_age(120);

        assert_eq!(resource.age_days, 120);
    }

    #[test]
    fn test_resource_with_recommendation() {
        let resource = WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0)
            .with_recommendation("Consider terminating");

        assert_eq!(resource.recommendation, "Consider terminating");
    }

    #[test]
    fn test_resource_annual_waste() {
        let resource = WastefulResource::new("vm-1", "compute", WasteType::Idle, 150.0);

        assert_eq!(resource.annual_waste(), 1800.0);
    }

    #[test]
    fn test_resource_is_critical() {
        let resource1 = WastefulResource::new("vm-1", "compute", WasteType::Idle, 1500.0);
        assert!(resource1.is_critical()); // High monthly waste

        let resource2 = WastefulResource::new("vm-2", "compute", WasteType::Idle, 500.0)
            .with_severity(WasteSeverity::Critical);
        assert!(resource2.is_critical()); // Critical severity

        let resource3 = WastefulResource::new("vm-3", "compute", WasteType::Idle, 100.0);
        assert!(!resource3.is_critical());
    }

    #[test]
    fn test_resource_is_old() {
        let resource1 =
            WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0).with_age(100);
        assert!(resource1.is_old());

        let resource2 =
            WastefulResource::new("vm-2", "compute", WasteType::Idle, 100.0).with_age(30);
        assert!(!resource2.is_old());
    }

    #[test]
    fn test_waste_detection_rule() {
        let rule = WasteDetectionRule::new("Idle VMs", WasteType::Idle, "compute");

        assert_eq!(rule.name, "Idle VMs");
        assert_eq!(rule.waste_type, WasteType::Idle);
        assert_eq!(rule.resource_type, "compute");
        assert!(rule.active);
    }

    #[test]
    fn test_rule_with_utilization_threshold() {
        let rule = WasteDetectionRule::new("Low Util", WasteType::Underutilized, "compute")
            .with_utilization_threshold(20.0);

        assert_eq!(rule.threshold_utilization, Some(20.0));
    }

    #[test]
    fn test_rule_utilization_threshold_clamping() {
        let rule1 = WasteDetectionRule::new("R1", WasteType::Underutilized, "compute")
            .with_utilization_threshold(150.0);
        assert_eq!(rule1.threshold_utilization, Some(100.0));

        let rule2 = WasteDetectionRule::new("R2", WasteType::Underutilized, "compute")
            .with_utilization_threshold(-10.0);
        assert_eq!(rule2.threshold_utilization, Some(0.0));
    }

    #[test]
    fn test_rule_with_age_threshold() {
        let rule = WasteDetectionRule::new("Old Resources", WasteType::Orphaned, "storage")
            .with_age_threshold(90);

        assert_eq!(rule.threshold_age_days, Some(90));
    }

    #[test]
    fn test_rule_with_cost_threshold() {
        let rule = WasteDetectionRule::new("Expensive Waste", WasteType::Idle, "compute")
            .with_cost_threshold(100.0);

        assert_eq!(rule.threshold_cost, Some(100.0));
    }

    #[test]
    fn test_rule_deactivate_activate() {
        let mut rule = WasteDetectionRule::new("Test", WasteType::Idle, "compute");

        assert!(rule.active);

        rule.deactivate();
        assert!(!rule.active);

        rule.activate();
        assert!(rule.active);
    }

    #[test]
    fn test_waste_manager() {
        let mut manager = WasteManager::new();

        let resource = WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0);
        let id = manager.add_wasteful_resource(resource);

        assert_eq!(manager.wasteful_resource_count(), 1);
        assert!(manager.get_wasteful_resource(&id).is_some());
    }

    #[test]
    fn test_manager_add_detection_rule() {
        let mut manager = WasteManager::new();

        let rule = WasteDetectionRule::new("Test Rule", WasteType::Idle, "compute");
        let id = manager.add_detection_rule(rule);

        assert_eq!(manager.detection_rule_count(), 1);
        assert!(manager.get_detection_rule(&id).is_some());
    }

    #[test]
    fn test_manager_active_rules() {
        let mut manager = WasteManager::new();

        let rule1 = WasteDetectionRule::new("R1", WasteType::Idle, "compute");
        let mut rule2 = WasteDetectionRule::new("R2", WasteType::Underutilized, "storage");
        rule2.deactivate();

        manager.add_detection_rule(rule1);
        manager.add_detection_rule(rule2);

        let active = manager.active_rules();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_by_waste_type() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(WastefulResource::new(
            "vm-1",
            "compute",
            WasteType::Idle,
            100.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vol-1",
            "storage",
            WasteType::Unattached,
            50.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-2",
            "compute",
            WasteType::Idle,
            150.0,
        ));

        let idle = manager.by_waste_type(&WasteType::Idle);
        assert_eq!(idle.len(), 2);
    }

    #[test]
    fn test_manager_by_severity() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(
            WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0)
                .with_severity(WasteSeverity::High),
        );
        manager.add_wasteful_resource(
            WastefulResource::new("vm-2", "compute", WasteType::Idle, 50.0)
                .with_severity(WasteSeverity::Low),
        );

        let high = manager.by_severity(&WasteSeverity::High);
        assert_eq!(high.len(), 1);
    }

    #[test]
    fn test_manager_critical_waste() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(WastefulResource::new(
            "vm-1",
            "compute",
            WasteType::Idle,
            1500.0,
        ));
        manager.add_wasteful_resource(
            WastefulResource::new("vm-2", "compute", WasteType::Idle, 200.0)
                .with_severity(WasteSeverity::Critical),
        );
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-3",
            "compute",
            WasteType::Idle,
            100.0,
        ));

        let critical = manager.critical_waste();
        assert_eq!(critical.len(), 2);
    }

    #[test]
    fn test_manager_old_resources() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(
            WastefulResource::new("vm-1", "compute", WasteType::Idle, 100.0).with_age(120),
        );
        manager.add_wasteful_resource(
            WastefulResource::new("vm-2", "compute", WasteType::Idle, 100.0).with_age(30),
        );

        let old = manager.old_resources();
        assert_eq!(old.len(), 1);
    }

    #[test]
    fn test_manager_total_waste() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(WastefulResource::new(
            "vm-1",
            "compute",
            WasteType::Idle,
            100.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-2",
            "compute",
            WasteType::Idle,
            150.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vol-1",
            "storage",
            WasteType::Unattached,
            50.0,
        ));

        assert_eq!(manager.total_monthly_waste(), 300.0);
        assert_eq!(manager.total_annual_waste(), 3600.0);
    }

    #[test]
    fn test_manager_by_resource_type() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(WastefulResource::new(
            "vm-1",
            "compute",
            WasteType::Idle,
            100.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vol-1",
            "storage",
            WasteType::Unattached,
            50.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-2",
            "compute",
            WasteType::Idle,
            150.0,
        ));

        let compute = manager.by_resource_type("compute");
        assert_eq!(compute.len(), 2);
    }

    #[test]
    fn test_manager_top_wasteful_resources() {
        let mut manager = WasteManager::new();

        manager.add_wasteful_resource(WastefulResource::new(
            "vm-1",
            "compute",
            WasteType::Idle,
            300.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-2",
            "compute",
            WasteType::Idle,
            150.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-3",
            "compute",
            WasteType::Idle,
            500.0,
        ));
        manager.add_wasteful_resource(WastefulResource::new(
            "vm-4",
            "compute",
            WasteType::Idle,
            75.0,
        ));

        let top = manager.top_wasteful_resources(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].monthly_waste, 500.0);
        assert_eq!(top[1].monthly_waste, 300.0);
    }

    #[test]
    fn test_waste_type_equality() {
        assert_eq!(WasteType::Idle, WasteType::Idle);
        assert_ne!(WasteType::Idle, WasteType::Underutilized);
    }

    #[test]
    fn test_waste_severity_equality() {
        assert_eq!(WasteSeverity::High, WasteSeverity::High);
        assert_ne!(WasteSeverity::High, WasteSeverity::Low);
    }
}
