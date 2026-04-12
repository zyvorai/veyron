use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimization strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    CostReduction,
    PerformanceImprovement,
    Balanced,
    GreenComputing,
}

impl std::fmt::Display for OptimizationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimizationStrategy::CostReduction => write!(f, "Cost Reduction"),
            OptimizationStrategy::PerformanceImprovement => write!(f, "Performance Improvement"),
            OptimizationStrategy::Balanced => write!(f, "Balanced"),
            OptimizationStrategy::GreenComputing => write!(f, "Green Computing"),
        }
    }
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub strategy: OptimizationStrategy,
    pub resource_id: String,
    pub current_config: HashMap<String, String>,
    pub recommended_config: HashMap<String, String>,
    pub estimated_savings: f64,
    pub implementation_effort: ImplementationEffort,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
}

impl OptimizationRecommendation {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        strategy: OptimizationStrategy,
        resource_id: impl Into<String>,
    ) -> Self {
        let title_str = title.into();
        let id = format!(
            "rec-{}-{}",
            title_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            title: title_str,
            description: description.into(),
            strategy,
            resource_id: resource_id.into(),
            current_config: HashMap::new(),
            recommended_config: HashMap::new(),
            estimated_savings: 0.0,
            implementation_effort: ImplementationEffort::Medium,
            priority: 5,
            created_at: Utc::now(),
        }
    }

    pub fn with_savings(mut self, savings: f64) -> Self {
        self.estimated_savings = savings;
        self
    }

    pub fn with_effort(mut self, effort: ImplementationEffort) -> Self {
        self.implementation_effort = effort;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn add_current_config(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.current_config.insert(key.into(), value.into());
    }

    pub fn add_recommended_config(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.recommended_config.insert(key.into(), value.into());
    }

    pub fn is_high_priority(&self) -> bool {
        self.priority >= 8
    }

    pub fn is_quick_win(&self) -> bool {
        self.implementation_effort == ImplementationEffort::Low && self.estimated_savings > 100.0
    }
}

/// Optimization manager
pub struct OptimizationManager {
    recommendations: HashMap<String, OptimizationRecommendation>,
}

impl OptimizationManager {
    pub fn new() -> Self {
        Self {
            recommendations: HashMap::new(),
        }
    }

    pub fn add_recommendation(&mut self, rec: OptimizationRecommendation) -> String {
        let id = rec.id.clone();
        self.recommendations.insert(id.clone(), rec);
        id
    }

    pub fn get_recommendation(&self, id: &str) -> Option<&OptimizationRecommendation> {
        self.recommendations.get(id)
    }

    pub fn recommendation_count(&self) -> usize {
        self.recommendations.len()
    }

    pub fn by_strategy(&self, strategy: &OptimizationStrategy) -> Vec<&OptimizationRecommendation> {
        self.recommendations
            .values()
            .filter(|r| &r.strategy == strategy)
            .collect()
    }

    pub fn high_priority(&self) -> Vec<&OptimizationRecommendation> {
        self.recommendations
            .values()
            .filter(|r| r.is_high_priority())
            .collect()
    }

    pub fn quick_wins(&self) -> Vec<&OptimizationRecommendation> {
        self.recommendations
            .values()
            .filter(|r| r.is_quick_win())
            .collect()
    }

    pub fn total_potential_savings(&self) -> f64 {
        self.recommendations
            .values()
            .map(|r| r.estimated_savings)
            .sum()
    }

    pub fn by_effort(&self, effort: &ImplementationEffort) -> Vec<&OptimizationRecommendation> {
        self.recommendations
            .values()
            .filter(|r| &r.implementation_effort == effort)
            .collect()
    }
}

impl Default for OptimizationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_strategy_display() {
        assert_eq!(
            OptimizationStrategy::CostReduction.to_string(),
            "Cost Reduction"
        );
        assert_eq!(
            OptimizationStrategy::PerformanceImprovement.to_string(),
            "Performance Improvement"
        );
        assert_eq!(
            OptimizationStrategy::GreenComputing.to_string(),
            "Green Computing"
        );
    }

    #[test]
    fn test_optimization_recommendation() {
        let rec = OptimizationRecommendation::new(
            "Reduce VM size",
            "VM is overprovisioned",
            OptimizationStrategy::CostReduction,
            "vm-123",
        );

        assert_eq!(rec.title, "Reduce VM size");
        assert_eq!(rec.strategy, OptimizationStrategy::CostReduction);
        assert_eq!(rec.resource_id, "vm-123");
        assert_eq!(rec.priority, 5);
    }

    #[test]
    fn test_recommendation_builder() {
        let rec = OptimizationRecommendation::new(
            "Upgrade instance",
            "Better performance",
            OptimizationStrategy::PerformanceImprovement,
            "vm-456",
        )
        .with_savings(500.0)
        .with_effort(ImplementationEffort::Low)
        .with_priority(9);

        assert_eq!(rec.estimated_savings, 500.0);
        assert_eq!(rec.implementation_effort, ImplementationEffort::Low);
        assert_eq!(rec.priority, 9);
        assert!(rec.is_high_priority());
    }

    #[test]
    fn test_recommendation_config() {
        let mut rec = OptimizationRecommendation::new(
            "Test",
            "Description",
            OptimizationStrategy::Balanced,
            "res-1",
        );

        rec.add_current_config("cpu", "4");
        rec.add_current_config("memory", "16GB");

        rec.add_recommended_config("cpu", "2");
        rec.add_recommended_config("memory", "8GB");

        assert_eq!(rec.current_config.len(), 2);
        assert_eq!(rec.recommended_config.len(), 2);
        assert_eq!(rec.current_config.get("cpu"), Some(&"4".to_string()));
        assert_eq!(
            rec.recommended_config.get("memory"),
            Some(&"8GB".to_string())
        );
    }

    #[test]
    fn test_recommendation_is_quick_win() {
        let rec1 = OptimizationRecommendation::new(
            "R1",
            "D1",
            OptimizationStrategy::CostReduction,
            "vm-1",
        )
        .with_savings(200.0)
        .with_effort(ImplementationEffort::Low);

        let rec2 = OptimizationRecommendation::new(
            "R2",
            "D2",
            OptimizationStrategy::CostReduction,
            "vm-2",
        )
        .with_savings(50.0)
        .with_effort(ImplementationEffort::Low);

        let rec3 = OptimizationRecommendation::new(
            "R3",
            "D3",
            OptimizationStrategy::CostReduction,
            "vm-3",
        )
        .with_savings(200.0)
        .with_effort(ImplementationEffort::High);

        assert!(rec1.is_quick_win());
        assert!(!rec2.is_quick_win()); // Low savings
        assert!(!rec3.is_quick_win()); // High effort
    }

    #[test]
    fn test_optimization_manager() {
        let mut manager = OptimizationManager::new();

        let rec = OptimizationRecommendation::new(
            "Test",
            "Description",
            OptimizationStrategy::Balanced,
            "res-1",
        );
        let id = manager.add_recommendation(rec);

        assert_eq!(manager.recommendation_count(), 1);
        assert!(manager.get_recommendation(&id).is_some());
    }

    #[test]
    fn test_manager_by_strategy() {
        let mut manager = OptimizationManager::new();

        manager.add_recommendation(OptimizationRecommendation::new(
            "R1",
            "D1",
            OptimizationStrategy::CostReduction,
            "vm-1",
        ));
        manager.add_recommendation(OptimizationRecommendation::new(
            "R2",
            "D2",
            OptimizationStrategy::PerformanceImprovement,
            "vm-2",
        ));
        manager.add_recommendation(OptimizationRecommendation::new(
            "R3",
            "D3",
            OptimizationStrategy::CostReduction,
            "vm-3",
        ));

        let cost_reduction = manager.by_strategy(&OptimizationStrategy::CostReduction);
        assert_eq!(cost_reduction.len(), 2);
    }

    #[test]
    fn test_manager_high_priority() {
        let mut manager = OptimizationManager::new();

        manager.add_recommendation(
            OptimizationRecommendation::new("R1", "D1", OptimizationStrategy::Balanced, "vm-1")
                .with_priority(9),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new("R2", "D2", OptimizationStrategy::Balanced, "vm-2")
                .with_priority(5),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new("R3", "D3", OptimizationStrategy::Balanced, "vm-3")
                .with_priority(8),
        );

        let high_priority = manager.high_priority();
        assert_eq!(high_priority.len(), 2);
    }

    #[test]
    fn test_manager_quick_wins() {
        let mut manager = OptimizationManager::new();

        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R1",
                "D1",
                OptimizationStrategy::CostReduction,
                "vm-1",
            )
            .with_savings(200.0)
            .with_effort(ImplementationEffort::Low),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R2",
                "D2",
                OptimizationStrategy::CostReduction,
                "vm-2",
            )
            .with_savings(50.0)
            .with_effort(ImplementationEffort::Low),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R3",
                "D3",
                OptimizationStrategy::CostReduction,
                "vm-3",
            )
            .with_savings(300.0)
            .with_effort(ImplementationEffort::Low),
        );

        let quick_wins = manager.quick_wins();
        assert_eq!(quick_wins.len(), 2);
    }

    #[test]
    fn test_manager_total_potential_savings() {
        let mut manager = OptimizationManager::new();

        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R1",
                "D1",
                OptimizationStrategy::CostReduction,
                "vm-1",
            )
            .with_savings(100.0),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R2",
                "D2",
                OptimizationStrategy::CostReduction,
                "vm-2",
            )
            .with_savings(250.0),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new(
                "R3",
                "D3",
                OptimizationStrategy::CostReduction,
                "vm-3",
            )
            .with_savings(150.0),
        );

        assert_eq!(manager.total_potential_savings(), 500.0);
    }

    #[test]
    fn test_manager_by_effort() {
        let mut manager = OptimizationManager::new();

        manager.add_recommendation(
            OptimizationRecommendation::new("R1", "D1", OptimizationStrategy::Balanced, "vm-1")
                .with_effort(ImplementationEffort::Low),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new("R2", "D2", OptimizationStrategy::Balanced, "vm-2")
                .with_effort(ImplementationEffort::High),
        );
        manager.add_recommendation(
            OptimizationRecommendation::new("R3", "D3", OptimizationStrategy::Balanced, "vm-3")
                .with_effort(ImplementationEffort::Low),
        );

        let low_effort = manager.by_effort(&ImplementationEffort::Low);
        assert_eq!(low_effort.len(), 2);
    }

    #[test]
    fn test_optimization_strategy_equality() {
        assert_eq!(
            OptimizationStrategy::CostReduction,
            OptimizationStrategy::CostReduction
        );
        assert_ne!(
            OptimizationStrategy::CostReduction,
            OptimizationStrategy::PerformanceImprovement
        );
    }

    #[test]
    fn test_implementation_effort_equality() {
        assert_eq!(ImplementationEffort::Low, ImplementationEffort::Low);
        assert_ne!(ImplementationEffort::Low, ImplementationEffort::High);
    }
}
