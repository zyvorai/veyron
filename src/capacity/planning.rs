use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ResourceType;

/// Capacity plan status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Proposed,
    Approved,
    Implementing,
    Completed,
    Cancelled,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Draft => write!(f, "Draft"),
            PlanStatus::Proposed => write!(f, "Proposed"),
            PlanStatus::Approved => write!(f, "Approved"),
            PlanStatus::Implementing => write!(f, "Implementing"),
            PlanStatus::Completed => write!(f, "Completed"),
            PlanStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Capacity expansion plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityPlan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: PlanStatus,
    pub expansions: Vec<CapacityExpansion>,
    pub estimated_cost: f64,
    pub timeline_days: u32,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityExpansion {
    pub resource_type: ResourceType,
    pub current_capacity: f64,
    pub additional_capacity: f64,
    pub justification: String,
}

impl CapacityPlan {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name_str = name.into();
        let id = format!(
            "plan-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            description: description.into(),
            status: PlanStatus::Draft,
            expansions: Vec::new(),
            estimated_cost: 0.0,
            timeline_days: 30,
            created_at: Utc::now(),
            approved_at: None,
            completed_at: None,
        }
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = cost;
        self
    }

    pub fn with_timeline(mut self, days: u32) -> Self {
        self.timeline_days = days;
        self
    }

    pub fn add_expansion(&mut self, expansion: CapacityExpansion) {
        self.expansions.push(expansion);
    }

    pub fn propose(&mut self) {
        self.status = PlanStatus::Proposed;
    }

    pub fn approve(&mut self) {
        self.status = PlanStatus::Approved;
        self.approved_at = Some(Utc::now());
    }

    pub fn start_implementation(&mut self) {
        self.status = PlanStatus::Implementing;
    }

    pub fn complete(&mut self) {
        self.status = PlanStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = PlanStatus::Cancelled;
    }

    pub fn expansion_count(&self) -> usize {
        self.expansions.len()
    }

    pub fn total_additional_cpu(&self) -> f64 {
        self.expansions
            .iter()
            .filter(|e| e.resource_type == ResourceType::CPU)
            .map(|e| e.additional_capacity)
            .sum()
    }

    pub fn is_approved(&self) -> bool {
        matches!(
            self.status,
            PlanStatus::Approved | PlanStatus::Implementing | PlanStatus::Completed
        )
    }
}

impl CapacityExpansion {
    pub fn new(
        resource_type: ResourceType,
        current: f64,
        additional: f64,
        justification: impl Into<String>,
    ) -> Self {
        Self {
            resource_type,
            current_capacity: current,
            additional_capacity: additional,
            justification: justification.into(),
        }
    }

    pub fn growth_percent(&self) -> f64 {
        if self.current_capacity == 0.0 {
            return 0.0;
        }
        (self.additional_capacity / self.current_capacity) * 100.0
    }
}

/// Capacity planner
pub struct CapacityPlanner {
    plans: HashMap<String, CapacityPlan>,
}

impl CapacityPlanner {
    pub fn new() -> Self {
        Self {
            plans: HashMap::new(),
        }
    }

    pub fn add_plan(&mut self, plan: CapacityPlan) -> String {
        let id = plan.id.clone();
        self.plans.insert(id.clone(), plan);
        id
    }

    pub fn get_plan(&self, id: &str) -> Option<&CapacityPlan> {
        self.plans.get(id)
    }

    pub fn get_plan_mut(&mut self, id: &str) -> Option<&mut CapacityPlan> {
        self.plans.get_mut(id)
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }

    pub fn by_status(&self, status: &PlanStatus) -> Vec<&CapacityPlan> {
        self.plans
            .values()
            .filter(|p| &p.status == status)
            .collect()
    }

    pub fn approved_plans(&self) -> Vec<&CapacityPlan> {
        self.plans.values().filter(|p| p.is_approved()).collect()
    }

    pub fn pending_plans(&self) -> Vec<&CapacityPlan> {
        self.plans
            .values()
            .filter(|p| matches!(p.status, PlanStatus::Draft | PlanStatus::Proposed))
            .collect()
    }

    pub fn total_estimated_cost(&self) -> f64 {
        self.plans.values().map(|p| p.estimated_cost).sum()
    }
}

impl Default for CapacityPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_status_display() {
        assert_eq!(PlanStatus::Draft.to_string(), "Draft");
        assert_eq!(PlanStatus::Approved.to_string(), "Approved");
        assert_eq!(PlanStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_capacity_plan() {
        let plan = CapacityPlan::new("Q2 Expansion", "Expand compute capacity for Q2");

        assert_eq!(plan.name, "Q2 Expansion");
        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(plan.timeline_days, 30);
        assert!(!plan.is_approved());
    }

    #[test]
    fn test_plan_builder() {
        let plan = CapacityPlan::new("Test Plan", "Description")
            .with_cost(50000.0)
            .with_timeline(60);

        assert_eq!(plan.estimated_cost, 50000.0);
        assert_eq!(plan.timeline_days, 60);
    }

    #[test]
    fn test_plan_lifecycle() {
        let mut plan = CapacityPlan::new("Test", "Description");

        assert_eq!(plan.status, PlanStatus::Draft);

        plan.propose();
        assert_eq!(plan.status, PlanStatus::Proposed);

        plan.approve();
        assert_eq!(plan.status, PlanStatus::Approved);
        assert!(plan.approved_at.is_some());
        assert!(plan.is_approved());

        plan.start_implementation();
        assert_eq!(plan.status, PlanStatus::Implementing);

        plan.complete();
        assert_eq!(plan.status, PlanStatus::Completed);
        assert!(plan.completed_at.is_some());
    }

    #[test]
    fn test_plan_cancel() {
        let mut plan = CapacityPlan::new("Test", "Description");

        plan.cancel();
        assert_eq!(plan.status, PlanStatus::Cancelled);
    }

    #[test]
    fn test_plan_add_expansion() {
        let mut plan = CapacityPlan::new("Test", "Description");

        plan.add_expansion(CapacityExpansion::new(
            ResourceType::CPU,
            100.0,
            50.0,
            "Increased demand",
        ));

        plan.add_expansion(CapacityExpansion::new(
            ResourceType::Memory,
            1000.0,
            500.0,
            "New workloads",
        ));

        assert_eq!(plan.expansion_count(), 2);
    }

    #[test]
    fn test_plan_total_additional_cpu() {
        let mut plan = CapacityPlan::new("Test", "Description");

        plan.add_expansion(CapacityExpansion::new(
            ResourceType::CPU,
            100.0,
            50.0,
            "Reason 1",
        ));
        plan.add_expansion(CapacityExpansion::new(
            ResourceType::Memory,
            1000.0,
            500.0,
            "Reason 2",
        ));
        plan.add_expansion(CapacityExpansion::new(
            ResourceType::CPU,
            200.0,
            30.0,
            "Reason 3",
        ));

        assert_eq!(plan.total_additional_cpu(), 80.0);
    }

    #[test]
    fn test_capacity_expansion() {
        let expansion =
            CapacityExpansion::new(ResourceType::CPU, 100.0, 50.0, "Peak load requirements");

        assert_eq!(expansion.resource_type, ResourceType::CPU);
        assert_eq!(expansion.current_capacity, 100.0);
        assert_eq!(expansion.additional_capacity, 50.0);
        assert_eq!(expansion.justification, "Peak load requirements");
    }

    #[test]
    fn test_expansion_growth_percent() {
        let expansion = CapacityExpansion::new(ResourceType::Memory, 1000.0, 250.0, "Growth");

        assert_eq!(expansion.growth_percent(), 25.0);
    }

    #[test]
    fn test_expansion_zero_current() {
        let expansion = CapacityExpansion::new(ResourceType::GPU, 0.0, 10.0, "New resource");

        assert_eq!(expansion.growth_percent(), 0.0);
    }

    #[test]
    fn test_capacity_planner() {
        let mut planner = CapacityPlanner::new();

        let plan = CapacityPlan::new("Test", "Description");
        let id = planner.add_plan(plan);

        assert_eq!(planner.plan_count(), 1);
        assert!(planner.get_plan(&id).is_some());
    }

    #[test]
    fn test_planner_by_status() {
        let mut planner = CapacityPlanner::new();

        let mut plan1 = CapacityPlan::new("Plan 1", "Description");
        plan1.approve();

        let plan2 = CapacityPlan::new("Plan 2", "Description");

        planner.add_plan(plan1);
        planner.add_plan(plan2);

        let approved = planner.by_status(&PlanStatus::Approved);
        assert_eq!(approved.len(), 1);
    }

    #[test]
    fn test_planner_approved_plans() {
        let mut planner = CapacityPlanner::new();

        let mut plan1 = CapacityPlan::new("Plan 1", "Description");
        plan1.approve();

        let mut plan2 = CapacityPlan::new("Plan 2", "Description");
        plan2.propose();

        let mut plan3 = CapacityPlan::new("Plan 3", "Description");
        plan3.approve();
        plan3.complete();

        planner.add_plan(plan1);
        planner.add_plan(plan2);
        planner.add_plan(plan3);

        let approved = planner.approved_plans();
        assert_eq!(approved.len(), 2);
    }

    #[test]
    fn test_planner_pending_plans() {
        let mut planner = CapacityPlanner::new();

        let plan1 = CapacityPlan::new("Plan 1", "Description");

        let mut plan2 = CapacityPlan::new("Plan 2", "Description");
        plan2.propose();

        let mut plan3 = CapacityPlan::new("Plan 3", "Description");
        plan3.approve();

        planner.add_plan(plan1);
        planner.add_plan(plan2);
        planner.add_plan(plan3);

        let pending = planner.pending_plans();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_planner_total_estimated_cost() {
        let mut planner = CapacityPlanner::new();

        planner.add_plan(CapacityPlan::new("P1", "D1").with_cost(10000.0));
        planner.add_plan(CapacityPlan::new("P2", "D2").with_cost(25000.0));
        planner.add_plan(CapacityPlan::new("P3", "D3").with_cost(15000.0));

        assert_eq!(planner.total_estimated_cost(), 50000.0);
    }

    #[test]
    fn test_planner_get_plan_mut() {
        let mut planner = CapacityPlanner::new();

        let plan = CapacityPlan::new("Test", "Description");
        let id = planner.add_plan(plan);

        if let Some(plan_mut) = planner.get_plan_mut(&id) {
            plan_mut.approve();
        }

        let plan = planner.get_plan(&id).unwrap();
        assert!(plan.is_approved());
    }

    #[test]
    fn test_plan_status_equality() {
        assert_eq!(PlanStatus::Approved, PlanStatus::Approved);
        assert_ne!(PlanStatus::Approved, PlanStatus::Draft);
    }
}
