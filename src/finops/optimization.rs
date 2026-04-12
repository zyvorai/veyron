use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimization type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationType {
    RightSizing,
    ReservedInstances,
    SpotInstances,
    StorageOptimization,
    UnusedResources,
    ScheduledShutdown,
    RegionOptimization,
}

/// Savings estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsEstimate {
    pub monthly_savings: f64,
    pub annual_savings: f64,
    pub currency: String,
    pub confidence_level: f64,
}

impl SavingsEstimate {
    pub fn new(monthly: f64) -> Self {
        Self {
            monthly_savings: monthly,
            annual_savings: monthly * 12.0,
            currency: "USD".to_string(),
            confidence_level: 0.85,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence_level = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }
}

/// Cost optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimization {
    pub id: String,
    pub title: String,
    pub description: String,
    pub optimization_type: OptimizationType,
    pub resource_id: String,
    pub current_cost: f64,
    pub optimized_cost: f64,
    pub savings: SavingsEstimate,
    pub implementation_effort: ImplementationEffort,
    pub risk_level: RiskLevel,
    pub priority_score: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl CostOptimization {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        optimization_type: OptimizationType,
        resource_id: impl Into<String>,
        current_cost: f64,
        optimized_cost: f64,
    ) -> Self {
        let title_str = title.into();
        let id = format!(
            "opt-{}-{}",
            title_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp_micros()
        );

        let monthly_savings = (current_cost - optimized_cost).max(0.0);

        Self {
            id,
            title: title_str,
            description: description.into(),
            optimization_type,
            resource_id: resource_id.into(),
            current_cost,
            optimized_cost,
            savings: SavingsEstimate::new(monthly_savings),
            implementation_effort: ImplementationEffort::Medium,
            risk_level: RiskLevel::Low,
            priority_score: 5,
            created_at: Utc::now(),
        }
    }

    pub fn with_effort(mut self, effort: ImplementationEffort) -> Self {
        self.implementation_effort = effort;
        self
    }

    pub fn with_risk(mut self, risk: RiskLevel) -> Self {
        self.risk_level = risk;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority_score = priority.min(10);
        self
    }

    pub fn savings_percent(&self) -> f64 {
        if self.current_cost == 0.0 {
            return 0.0;
        }
        ((self.current_cost - self.optimized_cost) / self.current_cost) * 100.0
    }

    pub fn is_high_priority(&self) -> bool {
        self.priority_score >= 8
    }

    pub fn is_quick_win(&self) -> bool {
        self.implementation_effort == ImplementationEffort::Low
            && self.savings.monthly_savings > 50.0
            && self.risk_level == RiskLevel::Low
    }
}

/// Reserved instance recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedInstanceRec {
    pub id: String,
    pub resource_type: String,
    pub instance_type: String,
    pub current_on_demand_cost: f64,
    pub reserved_cost: f64,
    pub term_months: u32,
    pub upfront_cost: f64,
    pub monthly_savings: f64,
    pub break_even_months: u32,
    pub created_at: DateTime<Utc>,
}

impl ReservedInstanceRec {
    pub fn new(
        resource_type: impl Into<String>,
        instance_type: impl Into<String>,
        on_demand_cost: f64,
        reserved_cost: f64,
        term_months: u32,
    ) -> Self {
        let res_type = resource_type.into();
        let inst_type = instance_type.into();
        let id = format!("ri-{}-{}-{}", res_type, inst_type, Utc::now().timestamp());

        let monthly_savings = on_demand_cost - reserved_cost;
        let upfront = 0.0;
        let break_even = if monthly_savings > 0.0 {
            (upfront / monthly_savings).ceil() as u32
        } else {
            term_months
        };

        Self {
            id,
            resource_type: res_type,
            instance_type: inst_type,
            current_on_demand_cost: on_demand_cost,
            reserved_cost,
            term_months,
            upfront_cost: upfront,
            monthly_savings,
            break_even_months: break_even,
            created_at: Utc::now(),
        }
    }

    pub fn with_upfront(mut self, upfront: f64) -> Self {
        self.upfront_cost = upfront;
        self.break_even_months = if self.monthly_savings > 0.0 {
            (upfront / self.monthly_savings).ceil() as u32
        } else {
            self.term_months
        };
        self
    }

    pub fn total_savings(&self) -> f64 {
        self.monthly_savings * self.term_months as f64 - self.upfront_cost
    }

    pub fn is_worthwhile(&self) -> bool {
        self.break_even_months < self.term_months && self.total_savings() > 0.0
    }
}

/// Optimization manager
pub struct OptimizationManager {
    optimizations: HashMap<String, CostOptimization>,
    reserved_instance_recs: HashMap<String, ReservedInstanceRec>,
}

impl OptimizationManager {
    pub fn new() -> Self {
        Self {
            optimizations: HashMap::new(),
            reserved_instance_recs: HashMap::new(),
        }
    }

    pub fn add_optimization(&mut self, opt: CostOptimization) -> String {
        let id = opt.id.clone();
        self.optimizations.insert(id.clone(), opt);
        id
    }

    pub fn get_optimization(&self, id: &str) -> Option<&CostOptimization> {
        self.optimizations.get(id)
    }

    pub fn optimization_count(&self) -> usize {
        self.optimizations.len()
    }

    pub fn add_reserved_instance_rec(&mut self, rec: ReservedInstanceRec) -> String {
        let id = rec.id.clone();
        self.reserved_instance_recs.insert(id.clone(), rec);
        id
    }

    pub fn reserved_instance_count(&self) -> usize {
        self.reserved_instance_recs.len()
    }

    pub fn by_type(&self, opt_type: &OptimizationType) -> Vec<&CostOptimization> {
        self.optimizations
            .values()
            .filter(|o| &o.optimization_type == opt_type)
            .collect()
    }

    pub fn high_priority(&self) -> Vec<&CostOptimization> {
        self.optimizations
            .values()
            .filter(|o| o.is_high_priority())
            .collect()
    }

    pub fn quick_wins(&self) -> Vec<&CostOptimization> {
        self.optimizations
            .values()
            .filter(|o| o.is_quick_win())
            .collect()
    }

    pub fn total_potential_savings(&self) -> f64 {
        self.optimizations
            .values()
            .map(|o| o.savings.monthly_savings)
            .sum()
    }

    pub fn worthwhile_reserved_instances(&self) -> Vec<&ReservedInstanceRec> {
        self.reserved_instance_recs
            .values()
            .filter(|r| r.is_worthwhile())
            .collect()
    }

    pub fn by_effort(&self, effort: &ImplementationEffort) -> Vec<&CostOptimization> {
        self.optimizations
            .values()
            .filter(|o| &o.implementation_effort == effort)
            .collect()
    }

    pub fn by_risk(&self, risk: &RiskLevel) -> Vec<&CostOptimization> {
        self.optimizations
            .values()
            .filter(|o| &o.risk_level == risk)
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
    fn test_savings_estimate() {
        let estimate = SavingsEstimate::new(500.0);

        assert_eq!(estimate.monthly_savings, 500.0);
        assert_eq!(estimate.annual_savings, 6000.0);
        assert_eq!(estimate.currency, "USD");
        assert_eq!(estimate.confidence_level, 0.85);
    }

    #[test]
    fn test_estimate_with_confidence() {
        let estimate = SavingsEstimate::new(300.0).with_confidence(0.95);

        assert_eq!(estimate.confidence_level, 0.95);
    }

    #[test]
    fn test_estimate_confidence_clamping() {
        let estimate1 = SavingsEstimate::new(100.0).with_confidence(1.5);
        assert_eq!(estimate1.confidence_level, 1.0);

        let estimate2 = SavingsEstimate::new(100.0).with_confidence(-0.2);
        assert_eq!(estimate2.confidence_level, 0.0);
    }

    #[test]
    fn test_estimate_with_currency() {
        let estimate = SavingsEstimate::new(200.0).with_currency("EUR");

        assert_eq!(estimate.currency, "EUR");
    }

    #[test]
    fn test_cost_optimization() {
        let opt = CostOptimization::new(
            "Downsize VM",
            "VM is overprovisioned",
            OptimizationType::RightSizing,
            "vm-123",
            500.0,
            300.0,
        );

        assert_eq!(opt.title, "Downsize VM");
        assert_eq!(opt.optimization_type, OptimizationType::RightSizing);
        assert_eq!(opt.current_cost, 500.0);
        assert_eq!(opt.optimized_cost, 300.0);
        assert_eq!(opt.savings.monthly_savings, 200.0);
    }

    #[test]
    fn test_optimization_with_effort() {
        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::UnusedResources,
            "vm-1",
            100.0,
            0.0,
        )
        .with_effort(ImplementationEffort::Low);

        assert_eq!(opt.implementation_effort, ImplementationEffort::Low);
    }

    #[test]
    fn test_optimization_with_risk() {
        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::SpotInstances,
            "vm-1",
            100.0,
            50.0,
        )
        .with_risk(RiskLevel::High);

        assert_eq!(opt.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_optimization_with_priority() {
        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::RightSizing,
            "vm-1",
            100.0,
            50.0,
        )
        .with_priority(9);

        assert_eq!(opt.priority_score, 9);
    }

    #[test]
    fn test_optimization_priority_clamping() {
        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::RightSizing,
            "vm-1",
            100.0,
            50.0,
        )
        .with_priority(15);

        assert_eq!(opt.priority_score, 10);
    }

    #[test]
    fn test_optimization_savings_percent() {
        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::RightSizing,
            "vm-1",
            1000.0,
            600.0,
        );

        assert_eq!(opt.savings_percent(), 40.0);
    }

    #[test]
    fn test_optimization_is_high_priority() {
        let opt1 = CostOptimization::new(
            "T1",
            "D1",
            OptimizationType::RightSizing,
            "vm-1",
            100.0,
            50.0,
        )
        .with_priority(9);
        assert!(opt1.is_high_priority());

        let opt2 = CostOptimization::new(
            "T2",
            "D2",
            OptimizationType::RightSizing,
            "vm-2",
            100.0,
            50.0,
        )
        .with_priority(5);
        assert!(!opt2.is_high_priority());
    }

    #[test]
    fn test_optimization_is_quick_win() {
        let opt1 = CostOptimization::new(
            "T1",
            "D1",
            OptimizationType::UnusedResources,
            "vm-1",
            200.0,
            0.0,
        )
        .with_effort(ImplementationEffort::Low)
        .with_risk(RiskLevel::Low);
        assert!(opt1.is_quick_win());

        let opt2 = CostOptimization::new(
            "T2",
            "D2",
            OptimizationType::RightSizing,
            "vm-2",
            100.0,
            80.0,
        )
        .with_effort(ImplementationEffort::Low)
        .with_risk(RiskLevel::Low);
        assert!(!opt2.is_quick_win()); // Low savings

        let opt3 = CostOptimization::new(
            "T3",
            "D3",
            OptimizationType::RightSizing,
            "vm-3",
            200.0,
            0.0,
        )
        .with_effort(ImplementationEffort::High)
        .with_risk(RiskLevel::Low);
        assert!(!opt3.is_quick_win()); // High effort
    }

    #[test]
    fn test_reserved_instance_rec() {
        let rec = ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12);

        assert_eq!(rec.resource_type, "compute");
        assert_eq!(rec.instance_type, "c5.large");
        assert_eq!(rec.current_on_demand_cost, 100.0);
        assert_eq!(rec.reserved_cost, 70.0);
        assert_eq!(rec.monthly_savings, 30.0);
        assert_eq!(rec.term_months, 12);
    }

    #[test]
    fn test_reserved_instance_with_upfront() {
        let rec =
            ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12).with_upfront(100.0);

        assert_eq!(rec.upfront_cost, 100.0);
        assert_eq!(rec.break_even_months, 4); // 100 / 30 = 3.33, ceil = 4
    }

    #[test]
    fn test_reserved_instance_total_savings() {
        let rec =
            ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12).with_upfront(100.0);

        // 30 * 12 - 100 = 260
        assert_eq!(rec.total_savings(), 260.0);
    }

    #[test]
    fn test_reserved_instance_is_worthwhile() {
        let rec1 =
            ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12).with_upfront(100.0);
        assert!(rec1.is_worthwhile()); // break_even: 4 months < 12 months

        let rec2 =
            ReservedInstanceRec::new("compute", "c5.large", 100.0, 90.0, 12).with_upfront(200.0);
        assert!(!rec2.is_worthwhile()); // break_even: 20 months > 12 months
    }

    #[test]
    fn test_optimization_manager() {
        let mut manager = OptimizationManager::new();

        let opt = CostOptimization::new(
            "Test",
            "Description",
            OptimizationType::RightSizing,
            "vm-1",
            100.0,
            50.0,
        );
        let id = manager.add_optimization(opt);

        assert_eq!(manager.optimization_count(), 1);
        assert!(manager.get_optimization(&id).is_some());
    }

    #[test]
    fn test_manager_add_reserved_instance() {
        let mut manager = OptimizationManager::new();

        let rec = ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12);
        manager.add_reserved_instance_rec(rec);

        assert_eq!(manager.reserved_instance_count(), 1);
    }

    #[test]
    fn test_manager_by_type() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(CostOptimization::new(
            "O1",
            "D1",
            OptimizationType::RightSizing,
            "vm-1",
            100.0,
            50.0,
        ));
        manager.add_optimization(CostOptimization::new(
            "O2",
            "D2",
            OptimizationType::UnusedResources,
            "vm-2",
            200.0,
            0.0,
        ));
        manager.add_optimization(CostOptimization::new(
            "O3",
            "D3",
            OptimizationType::RightSizing,
            "vm-3",
            150.0,
            100.0,
        ));

        let rightsizing = manager.by_type(&OptimizationType::RightSizing);
        assert_eq!(rightsizing.len(), 2);
    }

    #[test]
    fn test_manager_high_priority() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(
            CostOptimization::new(
                "O1",
                "D1",
                OptimizationType::RightSizing,
                "vm-1",
                100.0,
                50.0,
            )
            .with_priority(9),
        );
        manager.add_optimization(
            CostOptimization::new(
                "O2",
                "D2",
                OptimizationType::RightSizing,
                "vm-2",
                100.0,
                50.0,
            )
            .with_priority(5),
        );

        let high_priority = manager.high_priority();
        assert_eq!(high_priority.len(), 1);
    }

    #[test]
    fn test_manager_quick_wins() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(
            CostOptimization::new(
                "O1",
                "D1",
                OptimizationType::UnusedResources,
                "vm-1",
                200.0,
                0.0,
            )
            .with_effort(ImplementationEffort::Low)
            .with_risk(RiskLevel::Low),
        );
        manager.add_optimization(
            CostOptimization::new(
                "O2",
                "D2",
                OptimizationType::RightSizing,
                "vm-2",
                100.0,
                80.0,
            )
            .with_effort(ImplementationEffort::Low)
            .with_risk(RiskLevel::Low),
        );

        let quick_wins = manager.quick_wins();
        assert_eq!(quick_wins.len(), 1);
    }

    #[test]
    fn test_manager_total_potential_savings() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(CostOptimization::new(
            "O1",
            "D1",
            OptimizationType::RightSizing,
            "vm-1",
            200.0,
            100.0,
        ));
        manager.add_optimization(CostOptimization::new(
            "O2",
            "D2",
            OptimizationType::UnusedResources,
            "vm-2",
            150.0,
            0.0,
        ));
        manager.add_optimization(CostOptimization::new(
            "O3",
            "D3",
            OptimizationType::StorageOptimization,
            "vol-1",
            80.0,
            50.0,
        ));

        assert_eq!(manager.total_potential_savings(), 280.0); // 100 + 150 + 30
    }

    #[test]
    fn test_manager_worthwhile_reserved_instances() {
        let mut manager = OptimizationManager::new();

        manager.add_reserved_instance_rec(
            ReservedInstanceRec::new("compute", "c5.large", 100.0, 70.0, 12).with_upfront(100.0),
        );
        manager.add_reserved_instance_rec(
            ReservedInstanceRec::new("compute", "c5.xlarge", 200.0, 190.0, 12).with_upfront(500.0),
        );

        let worthwhile = manager.worthwhile_reserved_instances();
        assert_eq!(worthwhile.len(), 1);
    }

    #[test]
    fn test_manager_by_effort() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(
            CostOptimization::new(
                "O1",
                "D1",
                OptimizationType::RightSizing,
                "vm-1",
                100.0,
                50.0,
            )
            .with_effort(ImplementationEffort::Low),
        );
        manager.add_optimization(
            CostOptimization::new(
                "O2",
                "D2",
                OptimizationType::RightSizing,
                "vm-2",
                100.0,
                50.0,
            )
            .with_effort(ImplementationEffort::High),
        );

        let low_effort = manager.by_effort(&ImplementationEffort::Low);
        assert_eq!(low_effort.len(), 1);
    }

    #[test]
    fn test_manager_by_risk() {
        let mut manager = OptimizationManager::new();

        manager.add_optimization(
            CostOptimization::new(
                "O1",
                "D1",
                OptimizationType::SpotInstances,
                "vm-1",
                100.0,
                30.0,
            )
            .with_risk(RiskLevel::High),
        );
        manager.add_optimization(
            CostOptimization::new(
                "O2",
                "D2",
                OptimizationType::RightSizing,
                "vm-2",
                100.0,
                70.0,
            )
            .with_risk(RiskLevel::Low),
        );

        let high_risk = manager.by_risk(&RiskLevel::High);
        assert_eq!(high_risk.len(), 1);
    }

    #[test]
    fn test_optimization_type_equality() {
        assert_eq!(OptimizationType::RightSizing, OptimizationType::RightSizing);
        assert_ne!(
            OptimizationType::RightSizing,
            OptimizationType::SpotInstances
        );
    }
}
