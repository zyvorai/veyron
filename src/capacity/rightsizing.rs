use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ResourceType;

/// Rightsizing action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RightsizingAction {
    Downsize,
    Upsize,
    NoChange,
}

impl std::fmt::Display for RightsizingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightsizingAction::Downsize => write!(f, "Downsize"),
            RightsizingAction::Upsize => write!(f, "Upsize"),
            RightsizingAction::NoChange => write!(f, "No Change"),
        }
    }
}

/// Rightsizing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsizingRecommendation {
    pub id: String,
    pub resource_id: String,
    pub resource_type: ResourceType,
    pub action: RightsizingAction,
    pub current_value: f64,
    pub recommended_value: f64,
    pub utilization_percent: f64,
    pub monthly_savings: f64,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

impl RightsizingRecommendation {
    pub fn new(
        resource_id: impl Into<String>,
        resource_type: ResourceType,
        current: f64,
        recommended: f64,
        utilization: f64,
    ) -> Self {
        let res_id = resource_id.into();
        let id = format!("rightsize-{}-{}", res_id, Utc::now().timestamp());

        let action = if recommended < current {
            RightsizingAction::Downsize
        } else if recommended > current {
            RightsizingAction::Upsize
        } else {
            RightsizingAction::NoChange
        };

        Self {
            id,
            resource_id: res_id,
            resource_type,
            action,
            current_value: current,
            recommended_value: recommended,
            utilization_percent: utilization,
            monthly_savings: 0.0,
            confidence: 0.85,
            created_at: Utc::now(),
        }
    }

    pub fn with_savings(mut self, savings: f64) -> Self {
        self.monthly_savings = savings;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn change_percent(&self) -> f64 {
        if self.current_value == 0.0 {
            return 0.0;
        }
        ((self.recommended_value - self.current_value) / self.current_value) * 100.0
    }

    pub fn is_significant(&self) -> bool {
        self.change_percent().abs() > 10.0
    }

    pub fn annual_savings(&self) -> f64 {
        self.monthly_savings * 12.0
    }
}

/// Rightsizing manager
pub struct RightsizingManager {
    recommendations: HashMap<String, RightsizingRecommendation>,
}

impl RightsizingManager {
    pub fn new() -> Self {
        Self {
            recommendations: HashMap::new(),
        }
    }

    pub fn add_recommendation(&mut self, rec: RightsizingRecommendation) -> String {
        let id = rec.id.clone();
        self.recommendations.insert(id.clone(), rec);
        id
    }

    pub fn get_recommendation(&self, id: &str) -> Option<&RightsizingRecommendation> {
        self.recommendations.get(id)
    }

    pub fn recommendation_count(&self) -> usize {
        self.recommendations.len()
    }

    pub fn by_action(&self, action: &RightsizingAction) -> Vec<&RightsizingRecommendation> {
        self.recommendations
            .values()
            .filter(|r| &r.action == action)
            .collect()
    }

    pub fn by_resource_type(
        &self,
        resource_type: &ResourceType,
    ) -> Vec<&RightsizingRecommendation> {
        self.recommendations
            .values()
            .filter(|r| &r.resource_type == resource_type)
            .collect()
    }

    pub fn significant_changes(&self) -> Vec<&RightsizingRecommendation> {
        self.recommendations
            .values()
            .filter(|r| r.is_significant())
            .collect()
    }

    pub fn total_monthly_savings(&self) -> f64 {
        self.recommendations
            .values()
            .map(|r| r.monthly_savings)
            .sum()
    }

    pub fn total_annual_savings(&self) -> f64 {
        self.total_monthly_savings() * 12.0
    }

    pub fn downsize_recommendations(&self) -> Vec<&RightsizingRecommendation> {
        self.by_action(&RightsizingAction::Downsize)
    }

    pub fn upsize_recommendations(&self) -> Vec<&RightsizingRecommendation> {
        self.by_action(&RightsizingAction::Upsize)
    }
}

impl Default for RightsizingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rightsizing_action_display() {
        assert_eq!(RightsizingAction::Downsize.to_string(), "Downsize");
        assert_eq!(RightsizingAction::Upsize.to_string(), "Upsize");
        assert_eq!(RightsizingAction::NoChange.to_string(), "No Change");
    }

    #[test]
    fn test_rightsizing_recommendation_downsize() {
        let rec = RightsizingRecommendation::new("vm-1", ResourceType::CPU, 8.0, 4.0, 25.0);

        assert_eq!(rec.resource_id, "vm-1");
        assert_eq!(rec.resource_type, ResourceType::CPU);
        assert_eq!(rec.action, RightsizingAction::Downsize);
        assert_eq!(rec.current_value, 8.0);
        assert_eq!(rec.recommended_value, 4.0);
        assert_eq!(rec.utilization_percent, 25.0);
    }

    #[test]
    fn test_rightsizing_recommendation_upsize() {
        let rec = RightsizingRecommendation::new("vm-2", ResourceType::Memory, 16.0, 32.0, 90.0);

        assert_eq!(rec.action, RightsizingAction::Upsize);
        assert_eq!(rec.current_value, 16.0);
        assert_eq!(rec.recommended_value, 32.0);
    }

    #[test]
    fn test_rightsizing_recommendation_no_change() {
        let rec = RightsizingRecommendation::new("vm-3", ResourceType::Storage, 100.0, 100.0, 60.0);

        assert_eq!(rec.action, RightsizingAction::NoChange);
    }

    #[test]
    fn test_recommendation_builder() {
        let rec = RightsizingRecommendation::new("vm-4", ResourceType::CPU, 8.0, 4.0, 30.0)
            .with_savings(150.0)
            .with_confidence(0.92);

        assert_eq!(rec.monthly_savings, 150.0);
        assert_eq!(rec.confidence, 0.92);
    }

    #[test]
    fn test_recommendation_confidence_clamping() {
        let rec1 = RightsizingRecommendation::new("vm-5", ResourceType::CPU, 4.0, 2.0, 20.0)
            .with_confidence(1.5);
        assert_eq!(rec1.confidence, 1.0);

        let rec2 = RightsizingRecommendation::new("vm-6", ResourceType::CPU, 4.0, 2.0, 20.0)
            .with_confidence(-0.5);
        assert_eq!(rec2.confidence, 0.0);
    }

    #[test]
    fn test_recommendation_change_percent() {
        let rec1 = RightsizingRecommendation::new("vm-7", ResourceType::CPU, 8.0, 4.0, 25.0);
        assert_eq!(rec1.change_percent(), -50.0);

        let rec2 = RightsizingRecommendation::new("vm-8", ResourceType::Memory, 16.0, 32.0, 85.0);
        assert_eq!(rec2.change_percent(), 100.0);
    }

    #[test]
    fn test_recommendation_is_significant() {
        let rec1 = RightsizingRecommendation::new("vm-9", ResourceType::CPU, 8.0, 4.0, 25.0);
        assert!(rec1.is_significant()); // 50% change

        let rec2 =
            RightsizingRecommendation::new("vm-10", ResourceType::Memory, 100.0, 105.0, 60.0);
        assert!(!rec2.is_significant()); // 5% change
    }

    #[test]
    fn test_recommendation_annual_savings() {
        let rec = RightsizingRecommendation::new("vm-11", ResourceType::CPU, 8.0, 4.0, 30.0)
            .with_savings(100.0);

        assert_eq!(rec.annual_savings(), 1200.0);
    }

    #[test]
    fn test_recommendation_zero_current() {
        let rec = RightsizingRecommendation::new("vm-12", ResourceType::GPU, 0.0, 2.0, 0.0);

        assert_eq!(rec.change_percent(), 0.0);
    }

    #[test]
    fn test_rightsizing_manager() {
        let mut manager = RightsizingManager::new();

        let rec = RightsizingRecommendation::new("vm-1", ResourceType::CPU, 8.0, 4.0, 30.0);
        let id = manager.add_recommendation(rec);

        assert_eq!(manager.recommendation_count(), 1);
        assert!(manager.get_recommendation(&id).is_some());
    }

    #[test]
    fn test_manager_by_action() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-1",
            ResourceType::CPU,
            8.0,
            4.0,
            30.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-2",
            ResourceType::Memory,
            16.0,
            32.0,
            85.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-3",
            ResourceType::CPU,
            4.0,
            2.0,
            25.0,
        ));

        let downsize = manager.by_action(&RightsizingAction::Downsize);
        assert_eq!(downsize.len(), 2);

        let upsize = manager.by_action(&RightsizingAction::Upsize);
        assert_eq!(upsize.len(), 1);
    }

    #[test]
    fn test_manager_by_resource_type() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-1",
            ResourceType::CPU,
            8.0,
            4.0,
            30.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-2",
            ResourceType::Memory,
            16.0,
            8.0,
            40.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-3",
            ResourceType::CPU,
            4.0,
            2.0,
            25.0,
        ));

        let cpu = manager.by_resource_type(&ResourceType::CPU);
        assert_eq!(cpu.len(), 2);
    }

    #[test]
    fn test_manager_significant_changes() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-1",
            ResourceType::CPU,
            8.0,
            4.0,
            30.0,
        )); // 50% change
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-2",
            ResourceType::Memory,
            100.0,
            102.0,
            60.0,
        )); // 2% change
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-3",
            ResourceType::Storage,
            1000.0,
            800.0,
            55.0,
        )); // 20% change

        let significant = manager.significant_changes();
        assert_eq!(significant.len(), 2);
    }

    #[test]
    fn test_manager_total_savings() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(
            RightsizingRecommendation::new("vm-1", ResourceType::CPU, 8.0, 4.0, 30.0)
                .with_savings(100.0),
        );
        manager.add_recommendation(
            RightsizingRecommendation::new("vm-2", ResourceType::Memory, 32.0, 16.0, 40.0)
                .with_savings(75.0),
        );
        manager.add_recommendation(
            RightsizingRecommendation::new("vm-3", ResourceType::Storage, 500.0, 250.0, 35.0)
                .with_savings(50.0),
        );

        assert_eq!(manager.total_monthly_savings(), 225.0);
        assert_eq!(manager.total_annual_savings(), 2700.0);
    }

    #[test]
    fn test_manager_downsize_recommendations() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-1",
            ResourceType::CPU,
            8.0,
            4.0,
            30.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-2",
            ResourceType::Memory,
            16.0,
            32.0,
            85.0,
        ));

        let downsize = manager.downsize_recommendations();
        assert_eq!(downsize.len(), 1);
    }

    #[test]
    fn test_manager_upsize_recommendations() {
        let mut manager = RightsizingManager::new();

        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-1",
            ResourceType::CPU,
            8.0,
            4.0,
            30.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-2",
            ResourceType::Memory,
            16.0,
            32.0,
            85.0,
        ));
        manager.add_recommendation(RightsizingRecommendation::new(
            "vm-3",
            ResourceType::CPU,
            2.0,
            4.0,
            95.0,
        ));

        let upsize = manager.upsize_recommendations();
        assert_eq!(upsize.len(), 2);
    }

    #[test]
    fn test_rightsizing_action_equality() {
        assert_eq!(RightsizingAction::Downsize, RightsizingAction::Downsize);
        assert_ne!(RightsizingAction::Downsize, RightsizingAction::Upsize);
    }
}
