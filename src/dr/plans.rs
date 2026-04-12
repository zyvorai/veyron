use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{DRStrategy, RPO, RTO};

/// DR plan status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Active,
    Testing,
    Suspended,
    Archived,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Draft => write!(f, "Draft"),
            PlanStatus::Active => write!(f, "Active"),
            PlanStatus::Testing => write!(f, "Testing"),
            PlanStatus::Suspended => write!(f, "Suspended"),
            PlanStatus::Archived => write!(f, "Archived"),
        }
    }
}

/// DR plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DRPlan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub strategy: DRStrategy,
    pub status: PlanStatus,
    pub rpo: RPO,
    pub rto: RTO,
    pub steps: Vec<DRStep>,
    pub last_tested: Option<DateTime<Utc>>,
    pub next_test_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DRStep {
    pub order: u32,
    pub name: String,
    pub description: String,
    pub automated: bool,
    pub estimated_duration_seconds: u64,
    pub dependencies: Vec<u32>,
}

impl DRPlan {
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
            strategy: DRStrategy::ActivePassive,
            status: PlanStatus::Draft,
            rpo: RPO::hours(1),
            rto: RTO::hours(4),
            steps: Vec::new(),
            last_tested: None,
            next_test_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_strategy(mut self, strategy: DRStrategy) -> Self {
        self.strategy = strategy;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_rpo_rto(mut self, rpo: RPO, rto: RTO) -> Self {
        self.rpo = rpo;
        self.rto = rto;
        self.updated_at = Utc::now();
        self
    }

    pub fn add_step(&mut self, step: DRStep) {
        self.steps.push(step);
        self.updated_at = Utc::now();
    }

    pub fn activate(&mut self) {
        self.status = PlanStatus::Active;
        self.updated_at = Utc::now();
    }

    pub fn suspend(&mut self) {
        self.status = PlanStatus::Suspended;
        self.updated_at = Utc::now();
    }

    pub fn archive(&mut self) {
        self.status = PlanStatus::Archived;
        self.updated_at = Utc::now();
    }

    pub fn mark_tested(&mut self) {
        self.last_tested = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn schedule_next_test(&mut self, days: i64) {
        self.next_test_date = Some(Utc::now() + chrono::TimeDelta::days(days));
        self.updated_at = Utc::now();
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn automated_step_count(&self) -> usize {
        self.steps.iter().filter(|s| s.automated).count()
    }

    pub fn total_estimated_duration(&self) -> u64 {
        self.steps
            .iter()
            .map(|s| s.estimated_duration_seconds)
            .sum()
    }

    pub fn is_active(&self) -> bool {
        self.status == PlanStatus::Active
    }

    pub fn is_overdue_for_testing(&self) -> bool {
        if let Some(next_test) = self.next_test_date {
            Utc::now() > next_test
        } else {
            false
        }
    }
}

impl DRStep {
    pub fn new(order: u32, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            order,
            name: name.into(),
            description: description.into(),
            automated: false,
            estimated_duration_seconds: 300,
            dependencies: Vec::new(),
        }
    }

    pub fn with_automation(mut self, automated: bool) -> Self {
        self.automated = automated;
        self
    }

    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.estimated_duration_seconds = seconds;
        self
    }

    pub fn add_dependency(&mut self, step_order: u32) {
        if !self.dependencies.contains(&step_order) {
            self.dependencies.push(step_order);
        }
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }
}

/// DR plan manager
pub struct DRPlanManager {
    plans: HashMap<String, DRPlan>,
}

impl DRPlanManager {
    pub fn new() -> Self {
        Self {
            plans: HashMap::new(),
        }
    }

    pub fn add_plan(&mut self, plan: DRPlan) -> String {
        let id = plan.id.clone();
        self.plans.insert(id.clone(), plan);
        id
    }

    pub fn get_plan(&self, id: &str) -> Option<&DRPlan> {
        self.plans.get(id)
    }

    pub fn get_plan_mut(&mut self, id: &str) -> Option<&mut DRPlan> {
        self.plans.get_mut(id)
    }

    pub fn remove_plan(&mut self, id: &str) -> bool {
        self.plans.remove(id).is_some()
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }

    pub fn active_plans(&self) -> Vec<&DRPlan> {
        self.plans.values().filter(|p| p.is_active()).collect()
    }

    pub fn by_strategy(&self, strategy: &DRStrategy) -> Vec<&DRPlan> {
        self.plans
            .values()
            .filter(|p| &p.strategy == strategy)
            .collect()
    }

    pub fn overdue_tests(&self) -> Vec<&DRPlan> {
        self.plans
            .values()
            .filter(|p| p.is_overdue_for_testing())
            .collect()
    }
}

impl Default for DRPlanManager {
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
        assert_eq!(PlanStatus::Active.to_string(), "Active");
        assert_eq!(PlanStatus::Suspended.to_string(), "Suspended");
    }

    #[test]
    fn test_dr_plan() {
        let plan = DRPlan::new("Production DR", "DR plan for production workloads");

        assert_eq!(plan.name, "Production DR");
        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(plan.step_count(), 0);
        assert!(!plan.is_active());
    }

    #[test]
    fn test_plan_builder() {
        let plan = DRPlan::new("DR Plan", "Test plan")
            .with_strategy(DRStrategy::WarmStandby)
            .with_rpo_rto(RPO::minutes(30), RTO::hours(2));

        assert_eq!(plan.strategy, DRStrategy::WarmStandby);
        assert_eq!(plan.rpo.as_minutes(), 30);
        assert_eq!(plan.rto.as_hours(), 2);
    }

    #[test]
    fn test_plan_lifecycle() {
        let mut plan = DRPlan::new("Plan", "Test");

        assert_eq!(plan.status, PlanStatus::Draft);

        plan.activate();
        assert_eq!(plan.status, PlanStatus::Active);
        assert!(plan.is_active());

        plan.suspend();
        assert_eq!(plan.status, PlanStatus::Suspended);

        plan.archive();
        assert_eq!(plan.status, PlanStatus::Archived);
    }

    #[test]
    fn test_plan_add_steps() {
        let mut plan = DRPlan::new("Plan", "Test");

        plan.add_step(DRStep::new(1, "Step 1", "First step"));
        plan.add_step(DRStep::new(2, "Step 2", "Second step"));

        assert_eq!(plan.step_count(), 2);
    }

    #[test]
    fn test_plan_testing() {
        let mut plan = DRPlan::new("Plan", "Test");

        assert!(plan.last_tested.is_none());

        plan.mark_tested();
        assert!(plan.last_tested.is_some());

        plan.schedule_next_test(90);
        assert!(plan.next_test_date.is_some());
        assert!(!plan.is_overdue_for_testing());
    }

    #[test]
    fn test_plan_overdue() {
        let mut plan = DRPlan::new("Plan", "Test");

        plan.next_test_date = Some(Utc::now() - chrono::TimeDelta::days(1));
        assert!(plan.is_overdue_for_testing());
    }

    #[test]
    fn test_dr_step() {
        let step = DRStep::new(1, "Failover DB", "Failover database to secondary");

        assert_eq!(step.order, 1);
        assert_eq!(step.name, "Failover DB");
        assert!(!step.automated);
        assert!(!step.has_dependencies());
    }

    #[test]
    fn test_step_builder() {
        let step = DRStep::new(1, "Step", "Description")
            .with_automation(true)
            .with_duration(600);

        assert!(step.automated);
        assert_eq!(step.estimated_duration_seconds, 600);
    }

    #[test]
    fn test_step_dependencies() {
        let mut step = DRStep::new(3, "Step 3", "Third step");

        step.add_dependency(1);
        step.add_dependency(2);

        assert!(step.has_dependencies());
        assert_eq!(step.dependencies.len(), 2);
    }

    #[test]
    fn test_step_duplicate_dependency() {
        let mut step = DRStep::new(2, "Step", "Description");

        step.add_dependency(1);
        step.add_dependency(1);

        assert_eq!(step.dependencies.len(), 1);
    }

    #[test]
    fn test_plan_automated_steps() {
        let mut plan = DRPlan::new("Plan", "Test");

        plan.add_step(DRStep::new(1, "S1", "D1").with_automation(true));
        plan.add_step(DRStep::new(2, "S2", "D2"));
        plan.add_step(DRStep::new(3, "S3", "D3").with_automation(true));

        assert_eq!(plan.automated_step_count(), 2);
    }

    #[test]
    fn test_plan_total_duration() {
        let mut plan = DRPlan::new("Plan", "Test");

        plan.add_step(DRStep::new(1, "S1", "D1").with_duration(300));
        plan.add_step(DRStep::new(2, "S2", "D2").with_duration(600));
        plan.add_step(DRStep::new(3, "S3", "D3").with_duration(900));

        assert_eq!(plan.total_estimated_duration(), 1800);
    }

    #[test]
    fn test_plan_manager() {
        let mut manager = DRPlanManager::new();

        let plan = DRPlan::new("Plan", "Test");
        let id = manager.add_plan(plan);

        assert_eq!(manager.plan_count(), 1);
        assert!(manager.get_plan(&id).is_some());
    }

    #[test]
    fn test_manager_active_plans() {
        let mut manager = DRPlanManager::new();

        let mut plan1 = DRPlan::new("Plan 1", "Test");
        plan1.activate();

        let plan2 = DRPlan::new("Plan 2", "Test");

        manager.add_plan(plan1);
        manager.add_plan(plan2);

        let active = manager.active_plans();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_manager_by_strategy() {
        let mut manager = DRPlanManager::new();

        manager.add_plan(DRPlan::new("P1", "T1").with_strategy(DRStrategy::ActiveActive));
        manager.add_plan(DRPlan::new("P2", "T2").with_strategy(DRStrategy::WarmStandby));
        manager.add_plan(DRPlan::new("P3", "T3").with_strategy(DRStrategy::ActiveActive));

        let active_active = manager.by_strategy(&DRStrategy::ActiveActive);
        assert_eq!(active_active.len(), 2);
    }

    #[test]
    fn test_manager_overdue_tests() {
        let mut manager = DRPlanManager::new();

        let mut plan1 = DRPlan::new("Plan 1", "Test");
        plan1.next_test_date = Some(Utc::now() - chrono::TimeDelta::days(1));

        let plan2 = DRPlan::new("Plan 2", "Test");

        manager.add_plan(plan1);
        manager.add_plan(plan2);

        let overdue = manager.overdue_tests();
        assert_eq!(overdue.len(), 1);
    }

    #[test]
    fn test_manager_remove_plan() {
        let mut manager = DRPlanManager::new();

        let plan = DRPlan::new("Plan", "Test");
        let id = manager.add_plan(plan);

        assert!(manager.remove_plan(&id));
        assert_eq!(manager.plan_count(), 0);
    }

    #[test]
    fn test_manager_get_plan_mut() {
        let mut manager = DRPlanManager::new();

        let plan = DRPlan::new("Plan", "Test");
        let id = manager.add_plan(plan);

        if let Some(plan_mut) = manager.get_plan_mut(&id) {
            plan_mut.activate();
        }

        let plan = manager.get_plan(&id).unwrap();
        assert!(plan.is_active());
    }

    #[test]
    fn test_plan_status_equality() {
        assert_eq!(PlanStatus::Active, PlanStatus::Active);
        assert_ne!(PlanStatus::Active, PlanStatus::Draft);
    }
}
