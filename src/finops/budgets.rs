use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Budget period
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annually,
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetPeriod::Daily => write!(f, "Daily"),
            BudgetPeriod::Weekly => write!(f, "Weekly"),
            BudgetPeriod::Monthly => write!(f, "Monthly"),
            BudgetPeriod::Quarterly => write!(f, "Quarterly"),
            BudgetPeriod::Annually => write!(f, "Annually"),
        }
    }
}

/// Budget status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetStatus {
    Active,
    Exceeded,
    Warning,
    Expired,
}

/// Budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub name: String,
    pub amount: f64,
    pub currency: String,
    pub period: BudgetPeriod,
    pub cost_center_id: Option<String>,
    pub current_spend: f64,
    pub status: BudgetStatus,
    pub warning_threshold: f64,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Budget {
    pub fn new(name: impl Into<String>, amount: f64, period: BudgetPeriod) -> Self {
        let name_str = name.into();
        let id = format!(
            "budget-{}-{}",
            name_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name: name_str,
            amount,
            currency: "USD".to_string(),
            period,
            cost_center_id: None,
            current_spend: 0.0,
            status: BudgetStatus::Active,
            warning_threshold: 0.8,
            start_date: Utc::now(),
            end_date: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn with_cost_center(mut self, cost_center_id: impl Into<String>) -> Self {
        self.cost_center_id = Some(cost_center_id.into());
        self
    }

    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_dates(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_date = start;
        self.end_date = Some(end);
        self
    }

    pub fn add_spend(&mut self, amount: f64) {
        self.current_spend += amount;
        self.update_status();
    }

    pub fn remaining(&self) -> f64 {
        (self.amount - self.current_spend).max(0.0)
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.amount == 0.0 {
            return 0.0;
        }
        (self.current_spend / self.amount) * 100.0
    }

    fn update_status(&mut self) {
        if self.amount == 0.0 {
            self.status = if self.current_spend > 0.0 {
                BudgetStatus::Exceeded
            } else {
                BudgetStatus::Active
            };
            return;
        }
        let utilization = self.current_spend / self.amount;

        if utilization >= 1.0 {
            self.status = BudgetStatus::Exceeded;
        } else if utilization >= self.warning_threshold {
            self.status = BudgetStatus::Warning;
        } else {
            self.status = BudgetStatus::Active;
        }
    }

    pub fn is_exceeded(&self) -> bool {
        self.status == BudgetStatus::Exceeded
    }

    pub fn needs_alert(&self) -> bool {
        matches!(self.status, BudgetStatus::Exceeded | BudgetStatus::Warning)
    }
}

/// Budget alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub id: String,
    pub budget_id: String,
    pub alert_type: AlertType,
    pub message: String,
    pub utilization_percent: f64,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    Warning,
    Exceeded,
    Critical,
}

impl BudgetAlert {
    pub fn new(
        budget_id: impl Into<String>,
        alert_type: AlertType,
        message: impl Into<String>,
        utilization: f64,
    ) -> Self {
        let budget_id_str = budget_id.into();
        let id = format!("alert-{}-{}", budget_id_str, Utc::now().timestamp_micros());

        Self {
            id,
            budget_id: budget_id_str,
            alert_type,
            message: message.into(),
            utilization_percent: utilization,
            triggered_at: Utc::now(),
            acknowledged: false,
        }
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}

/// Budget manager
pub struct BudgetManager {
    budgets: HashMap<String, Budget>,
    alerts: Vec<BudgetAlert>,
}

impl BudgetManager {
    pub fn new() -> Self {
        Self {
            budgets: HashMap::new(),
            alerts: Vec::new(),
        }
    }

    pub fn add_budget(&mut self, budget: Budget) -> String {
        let id = budget.id.clone();
        self.budgets.insert(id.clone(), budget);
        id
    }

    pub fn get_budget(&self, id: &str) -> Option<&Budget> {
        self.budgets.get(id)
    }

    pub fn get_budget_mut(&mut self, id: &str) -> Option<&mut Budget> {
        self.budgets.get_mut(id)
    }

    pub fn budget_count(&self) -> usize {
        self.budgets.len()
    }

    pub fn add_alert(&mut self, alert: BudgetAlert) {
        self.alerts.push(alert);
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }

    pub fn unacknowledged_alerts(&self) -> Vec<&BudgetAlert> {
        self.alerts.iter().filter(|a| !a.acknowledged).collect()
    }

    pub fn exceeded_budgets(&self) -> Vec<&Budget> {
        self.budgets.values().filter(|b| b.is_exceeded()).collect()
    }

    pub fn budgets_needing_alerts(&self) -> Vec<&Budget> {
        self.budgets.values().filter(|b| b.needs_alert()).collect()
    }

    pub fn by_cost_center(&self, cost_center_id: &str) -> Vec<&Budget> {
        self.budgets
            .values()
            .filter(|b| b.cost_center_id.as_deref() == Some(cost_center_id))
            .collect()
    }

    pub fn total_budget_amount(&self) -> f64 {
        self.budgets.values().map(|b| b.amount).sum()
    }

    pub fn total_current_spend(&self) -> f64 {
        self.budgets.values().map(|b| b.current_spend).sum()
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_period_display() {
        assert_eq!(BudgetPeriod::Daily.to_string(), "Daily");
        assert_eq!(BudgetPeriod::Monthly.to_string(), "Monthly");
        assert_eq!(BudgetPeriod::Annually.to_string(), "Annually");
    }

    #[test]
    fn test_budget() {
        let budget = Budget::new("Q1 Budget", 10000.0, BudgetPeriod::Quarterly);

        assert_eq!(budget.name, "Q1 Budget");
        assert_eq!(budget.amount, 10000.0);
        assert_eq!(budget.period, BudgetPeriod::Quarterly);
        assert_eq!(budget.status, BudgetStatus::Active);
        assert_eq!(budget.current_spend, 0.0);
    }

    #[test]
    fn test_budget_with_currency() {
        let budget = Budget::new("Dev Budget", 5000.0, BudgetPeriod::Monthly).with_currency("EUR");

        assert_eq!(budget.currency, "EUR");
    }

    #[test]
    fn test_budget_with_cost_center() {
        let budget = Budget::new("Team Budget", 3000.0, BudgetPeriod::Monthly)
            .with_cost_center("cc-engineering");

        assert_eq!(budget.cost_center_id, Some("cc-engineering".to_string()));
    }

    #[test]
    fn test_budget_with_warning_threshold() {
        let budget =
            Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly).with_warning_threshold(0.75);

        assert_eq!(budget.warning_threshold, 0.75);
    }

    #[test]
    fn test_budget_warning_threshold_clamping() {
        let budget1 = Budget::new("B1", 1000.0, BudgetPeriod::Monthly).with_warning_threshold(1.5);
        assert_eq!(budget1.warning_threshold, 1.0);

        let budget2 = Budget::new("B2", 1000.0, BudgetPeriod::Monthly).with_warning_threshold(-0.5);
        assert_eq!(budget2.warning_threshold, 0.0);
    }

    #[test]
    fn test_budget_add_spend() {
        let mut budget = Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly);

        budget.add_spend(300.0);
        assert_eq!(budget.current_spend, 300.0);
        assert_eq!(budget.status, BudgetStatus::Active);

        budget.add_spend(600.0);
        assert_eq!(budget.current_spend, 900.0);
        assert_eq!(budget.status, BudgetStatus::Warning);
    }

    #[test]
    fn test_budget_remaining() {
        let mut budget = Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly);

        assert_eq!(budget.remaining(), 1000.0);

        budget.add_spend(400.0);
        assert_eq!(budget.remaining(), 600.0);

        budget.add_spend(700.0);
        assert_eq!(budget.remaining(), 0.0); // Clamped to 0
    }

    #[test]
    fn test_budget_utilization_percent() {
        let mut budget = Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly);

        assert_eq!(budget.utilization_percent(), 0.0);

        budget.add_spend(500.0);
        assert_eq!(budget.utilization_percent(), 50.0);

        budget.add_spend(250.0);
        assert_eq!(budget.utilization_percent(), 75.0);
    }

    #[test]
    fn test_budget_status_transitions() {
        let mut budget =
            Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly).with_warning_threshold(0.8);

        assert_eq!(budget.status, BudgetStatus::Active);

        budget.add_spend(850.0);
        assert_eq!(budget.status, BudgetStatus::Warning);

        budget.add_spend(200.0);
        assert_eq!(budget.status, BudgetStatus::Exceeded);
    }

    #[test]
    fn test_budget_is_exceeded() {
        let mut budget = Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly);

        assert!(!budget.is_exceeded());

        budget.add_spend(1100.0);
        assert!(budget.is_exceeded());
    }

    #[test]
    fn test_budget_needs_alert() {
        let mut budget = Budget::new("Test Budget", 1000.0, BudgetPeriod::Monthly);

        assert!(!budget.needs_alert());

        budget.add_spend(850.0);
        assert!(budget.needs_alert());

        budget.add_spend(200.0);
        assert!(budget.needs_alert());
    }

    #[test]
    fn test_budget_alert() {
        let alert = BudgetAlert::new("budget-123", AlertType::Warning, "Budget at 85%", 85.0);

        assert_eq!(alert.budget_id, "budget-123");
        assert_eq!(alert.alert_type, AlertType::Warning);
        assert_eq!(alert.utilization_percent, 85.0);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn test_alert_acknowledge() {
        let mut alert = BudgetAlert::new("budget-123", AlertType::Warning, "Test", 85.0);

        assert!(!alert.acknowledged);

        alert.acknowledge();
        assert!(alert.acknowledged);
    }

    #[test]
    fn test_budget_manager() {
        let mut manager = BudgetManager::new();

        let budget = Budget::new("Q1 Budget", 10000.0, BudgetPeriod::Quarterly);
        let id = manager.add_budget(budget);

        assert_eq!(manager.budget_count(), 1);
        assert!(manager.get_budget(&id).is_some());
    }

    #[test]
    fn test_manager_add_alert() {
        let mut manager = BudgetManager::new();

        let alert = BudgetAlert::new("budget-1", AlertType::Warning, "Test", 85.0);
        manager.add_alert(alert);

        assert_eq!(manager.alert_count(), 1);
    }

    #[test]
    fn test_manager_unacknowledged_alerts() {
        let mut manager = BudgetManager::new();

        let mut alert1 = BudgetAlert::new("budget-1", AlertType::Warning, "Test 1", 85.0);
        alert1.acknowledge();

        let alert2 = BudgetAlert::new("budget-2", AlertType::Exceeded, "Test 2", 105.0);

        manager.add_alert(alert1);
        manager.add_alert(alert2);

        let unacked = manager.unacknowledged_alerts();
        assert_eq!(unacked.len(), 1);
    }

    #[test]
    fn test_manager_exceeded_budgets() {
        let mut manager = BudgetManager::new();

        let mut budget1 = Budget::new("B1", 1000.0, BudgetPeriod::Monthly);
        budget1.add_spend(1100.0);

        let budget2 = Budget::new("B2", 2000.0, BudgetPeriod::Monthly);

        manager.add_budget(budget1);
        manager.add_budget(budget2);

        let exceeded = manager.exceeded_budgets();
        assert_eq!(exceeded.len(), 1);
    }

    #[test]
    fn test_manager_budgets_needing_alerts() {
        let mut manager = BudgetManager::new();

        let mut budget1 = Budget::new("B1", 1000.0, BudgetPeriod::Monthly);
        budget1.add_spend(900.0);

        let mut budget2 = Budget::new("B2", 2000.0, BudgetPeriod::Monthly);
        budget2.add_spend(500.0);

        let mut budget3 = Budget::new("B3", 500.0, BudgetPeriod::Monthly);
        budget3.add_spend(550.0);

        manager.add_budget(budget1);
        manager.add_budget(budget2);
        manager.add_budget(budget3);

        let needing_alerts = manager.budgets_needing_alerts();
        assert_eq!(needing_alerts.len(), 2);
    }

    #[test]
    fn test_manager_by_cost_center() {
        let mut manager = BudgetManager::new();

        let budget1 = Budget::new("B1", 1000.0, BudgetPeriod::Monthly).with_cost_center("cc-eng");
        let budget2 = Budget::new("B2", 2000.0, BudgetPeriod::Monthly).with_cost_center("cc-sales");
        let budget3 = Budget::new("B3", 1500.0, BudgetPeriod::Monthly).with_cost_center("cc-eng");

        manager.add_budget(budget1);
        manager.add_budget(budget2);
        manager.add_budget(budget3);

        let eng_budgets = manager.by_cost_center("cc-eng");
        assert_eq!(eng_budgets.len(), 2);
    }

    #[test]
    fn test_manager_total_budget_amount() {
        let mut manager = BudgetManager::new();

        manager.add_budget(Budget::new("B1", 1000.0, BudgetPeriod::Monthly));
        manager.add_budget(Budget::new("B2", 2000.0, BudgetPeriod::Monthly));
        manager.add_budget(Budget::new("B3", 1500.0, BudgetPeriod::Monthly));

        assert_eq!(manager.total_budget_amount(), 4500.0);
    }

    #[test]
    fn test_manager_total_current_spend() {
        let mut manager = BudgetManager::new();

        let mut budget1 = Budget::new("B1", 1000.0, BudgetPeriod::Monthly);
        budget1.add_spend(300.0);

        let mut budget2 = Budget::new("B2", 2000.0, BudgetPeriod::Monthly);
        budget2.add_spend(800.0);

        manager.add_budget(budget1);
        manager.add_budget(budget2);

        assert_eq!(manager.total_current_spend(), 1100.0);
    }

    #[test]
    fn test_budget_period_equality() {
        assert_eq!(BudgetPeriod::Monthly, BudgetPeriod::Monthly);
        assert_ne!(BudgetPeriod::Monthly, BudgetPeriod::Quarterly);
    }

    #[test]
    fn test_alert_type_equality() {
        assert_eq!(AlertType::Warning, AlertType::Warning);
        assert_ne!(AlertType::Warning, AlertType::Exceeded);
    }
}
