// Budget Management - Set and monitor spending budgets

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub name: String,
    pub amount: f64,
    pub period: BudgetPeriod,
    pub scope: BudgetScope,
    pub alerts: Vec<BudgetAlert>,
    pub created_at: DateTime<Utc>,
    pub current_spend: f64,
}

impl Budget {
    pub fn new(name: impl Into<String>, amount: f64, period: BudgetPeriod) -> Self {
        Self {
            name: name.into(),
            amount,
            period,
            scope: BudgetScope::Global,
            alerts: Vec::new(),
            created_at: Utc::now(),
            current_spend: 0.0,
        }
    }

    pub fn with_scope(mut self, scope: BudgetScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn add_alert(mut self, alert: BudgetAlert) -> Self {
        self.alerts.push(alert);
        self
    }

    pub fn update_spend(&mut self, amount: f64) {
        self.current_spend = amount;
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.amount > 0.0 {
            (self.current_spend / self.amount) * 100.0
        } else {
            0.0
        }
    }

    pub fn remaining(&self) -> f64 {
        (self.amount - self.current_spend).max(0.0)
    }

    pub fn is_exceeded(&self) -> bool {
        self.current_spend > self.amount
    }

    pub fn triggered_alerts(&self) -> Vec<&BudgetAlert> {
        let utilization = self.utilization_percent();
        self.alerts
            .iter()
            .filter(|alert| utilization >= alert.threshold_percent)
            .collect()
    }
}

/// Budget period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetPeriod::Daily => write!(f, "Daily"),
            BudgetPeriod::Weekly => write!(f, "Weekly"),
            BudgetPeriod::Monthly => write!(f, "Monthly"),
            BudgetPeriod::Quarterly => write!(f, "Quarterly"),
            BudgetPeriod::Yearly => write!(f, "Yearly"),
        }
    }
}

/// Budget scope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetScope {
    Global,
    Namespace(String),
    Team(String),
    Project(String),
    VM(String),
}

impl std::fmt::Display for BudgetScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetScope::Global => write!(f, "Global"),
            BudgetScope::Namespace(ns) => write!(f, "Namespace: {}", ns),
            BudgetScope::Team(team) => write!(f, "Team: {}", team),
            BudgetScope::Project(proj) => write!(f, "Project: {}", proj),
            BudgetScope::VM(vm) => write!(f, "VM: {}", vm),
        }
    }
}

/// Budget alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub threshold_percent: f64,
    pub notification_type: NotificationType,
    pub recipients: Vec<String>,
}

impl BudgetAlert {
    pub fn new(threshold_percent: f64, notification_type: NotificationType) -> Self {
        Self {
            threshold_percent,
            notification_type,
            recipients: Vec::new(),
        }
    }

    pub fn add_recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipients.push(recipient.into());
        self
    }
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    Email,
    Slack,
    Webhook,
    Log,
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub budget_name: String,
    pub amount: f64,
    pub current_spend: f64,
    pub utilization_percent: f64,
    pub remaining: f64,
    pub status: Status,
    pub triggered_alerts: usize,
    pub days_remaining: i64,
}

impl BudgetStatus {
    pub fn from_budget(budget: &Budget) -> Self {
        let utilization = budget.utilization_percent();
        let status = if budget.is_exceeded() {
            Status::Exceeded
        } else if utilization >= 90.0 {
            Status::Critical
        } else if utilization >= 75.0 {
            Status::Warning
        } else {
            Status::Healthy
        };

        Self {
            budget_name: budget.name.clone(),
            amount: budget.amount,
            current_spend: budget.current_spend,
            utilization_percent: utilization,
            remaining: budget.remaining(),
            status,
            triggered_alerts: budget.triggered_alerts().len(),
            days_remaining: match budget.period {
                BudgetPeriod::Daily => 1,
                BudgetPeriod::Weekly => {
                    let elapsed = Utc::now().signed_duration_since(budget.created_at);
                    (7 - (elapsed.num_days() % 7)).max(0)
                }
                BudgetPeriod::Monthly => {
                    let now = Utc::now();
                    let next_month_start = if now.month() == 12 {
                        chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
                    } else {
                        chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
                    };
                    match next_month_start {
                        Some(end) => (end - now.date_naive()).num_days().max(0),
                        None => 30,
                    }
                }
                BudgetPeriod::Quarterly => {
                    let now = Utc::now();
                    let quarter_end_month = ((now.month() - 1) / 3 + 1) * 3 + 1;
                    let (y, m) = if quarter_end_month > 12 {
                        (now.year() + 1, 1)
                    } else {
                        (now.year(), quarter_end_month)
                    };
                    match chrono::NaiveDate::from_ymd_opt(y, m, 1) {
                        Some(end) => (end - now.date_naive()).num_days().max(0),
                        None => 90,
                    }
                }
                BudgetPeriod::Yearly => {
                    let now = Utc::now();
                    match chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1) {
                        Some(end) => (end - now.date_naive()).num_days().max(0),
                        None => 365,
                    }
                }
            },
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, Status::Healthy)
    }
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Status {
    Healthy,
    Warning,
    Critical,
    Exceeded,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Healthy => write!(f, "Healthy"),
            Status::Warning => write!(f, "Warning"),
            Status::Critical => write!(f, "Critical"),
            Status::Exceeded => write!(f, "Exceeded"),
        }
    }
}

/// Budget manager
pub struct BudgetManager {
    budgets: Vec<Budget>,
}

impl BudgetManager {
    pub fn new() -> Self {
        Self {
            budgets: Vec::new(),
        }
    }

    pub fn add_budget(&mut self, budget: Budget) {
        self.budgets.push(budget);
    }

    pub fn get_budget(&self, name: &str) -> Option<&Budget> {
        self.budgets.iter().find(|b| b.name == name)
    }

    pub fn get_budget_mut(&mut self, name: &str) -> Option<&mut Budget> {
        self.budgets.iter_mut().find(|b| b.name == name)
    }

    pub fn update_budget_spend(&mut self, name: &str, amount: f64) -> bool {
        if let Some(budget) = self.get_budget_mut(name) {
            budget.update_spend(amount);
            true
        } else {
            false
        }
    }

    pub fn all_budgets(&self) -> &[Budget] {
        &self.budgets
    }

    pub fn exceeded_budgets(&self) -> Vec<&Budget> {
        self.budgets.iter().filter(|b| b.is_exceeded()).collect()
    }

    pub fn budgets_with_alerts(&self) -> Vec<&Budget> {
        self.budgets
            .iter()
            .filter(|b| !b.triggered_alerts().is_empty())
            .collect()
    }

    pub fn budget_count(&self) -> usize {
        self.budgets.len()
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostForecast {
    pub period: BudgetPeriod,
    pub current_spend: f64,
    pub projected_spend: f64,
    pub forecast_method: ForecastMethod,
    pub confidence: f64, // 0-100
}

impl CostForecast {
    pub fn new(period: BudgetPeriod, current_spend: f64) -> Self {
        Self {
            period,
            current_spend,
            projected_spend: 0.0,
            forecast_method: ForecastMethod::Linear,
            confidence: 85.0,
        }
    }

    /// Linear projection based on current spend rate
    pub fn project_linear(&mut self, days_elapsed: f64, total_days: f64) {
        if days_elapsed > 0.0 {
            let daily_rate = self.current_spend / days_elapsed;
            self.projected_spend = daily_rate * total_days;
            self.forecast_method = ForecastMethod::Linear;
        }
    }

    /// Weighted projection (recent trends weighted higher)
    pub fn project_weighted(&mut self, recent_spend: f64, total_days: f64, recent_days: f64) {
        if recent_days > 0.0 {
            let recent_daily_rate = recent_spend / recent_days;
            self.projected_spend = recent_daily_rate * total_days;
            self.forecast_method = ForecastMethod::Weighted;
            self.confidence = 75.0; // Slightly lower confidence
        }
    }

    pub fn is_over_budget(&self, budget: f64) -> bool {
        self.projected_spend > budget
    }
}

/// Forecast method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForecastMethod {
    Linear,
    Weighted,
    MovingAverage,
}

impl std::fmt::Display for ForecastMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForecastMethod::Linear => write!(f, "Linear"),
            ForecastMethod::Weighted => write!(f, "Weighted"),
            ForecastMethod::MovingAverage => write!(f, "Moving Average"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget() {
        let budget = Budget::new("monthly-budget", 1000.0, BudgetPeriod::Monthly)
            .with_scope(BudgetScope::Team("engineering".to_string()))
            .add_alert(BudgetAlert::new(80.0, NotificationType::Email));

        assert_eq!(budget.name, "monthly-budget");
        assert_eq!(budget.amount, 1000.0);
        assert_eq!(budget.period, BudgetPeriod::Monthly);
        assert_eq!(budget.alerts.len(), 1);
    }

    #[test]
    fn test_budget_utilization() {
        let mut budget = Budget::new("test", 1000.0, BudgetPeriod::Monthly);
        budget.update_spend(750.0);

        assert_eq!(budget.utilization_percent(), 75.0);
        assert_eq!(budget.remaining(), 250.0);
        assert!(!budget.is_exceeded());
    }

    #[test]
    fn test_budget_exceeded() {
        let mut budget = Budget::new("test", 1000.0, BudgetPeriod::Monthly);
        budget.update_spend(1200.0);

        assert!(budget.is_exceeded());
        assert_eq!(budget.remaining(), 0.0);
    }

    #[test]
    fn test_budget_alerts() {
        let budget = Budget::new("test", 1000.0, BudgetPeriod::Monthly)
            .add_alert(BudgetAlert::new(50.0, NotificationType::Log))
            .add_alert(BudgetAlert::new(80.0, NotificationType::Email))
            .add_alert(BudgetAlert::new(95.0, NotificationType::Slack));

        let mut budget = budget;
        budget.update_spend(850.0); // 85% utilization

        let triggered = budget.triggered_alerts();
        assert_eq!(triggered.len(), 2); // 50% and 80% alerts
    }

    #[test]
    fn test_budget_alert_recipients() {
        let alert = BudgetAlert::new(80.0, NotificationType::Email)
            .add_recipient("admin@example.com")
            .add_recipient("finance@example.com");

        assert_eq!(alert.recipients.len(), 2);
    }

    #[test]
    fn test_budget_status() {
        let mut budget = Budget::new("test", 1000.0, BudgetPeriod::Monthly);
        budget.update_spend(750.0);

        let status = BudgetStatus::from_budget(&budget);

        assert_eq!(status.status, Status::Warning);
        assert_eq!(status.utilization_percent, 75.0);
    }

    #[test]
    fn test_budget_manager() {
        let mut manager = BudgetManager::new();

        manager.add_budget(Budget::new("monthly", 1000.0, BudgetPeriod::Monthly));
        manager.add_budget(Budget::new("quarterly", 3000.0, BudgetPeriod::Quarterly));

        assert_eq!(manager.budget_count(), 2);
        assert!(manager.get_budget("monthly").is_some());
    }

    #[test]
    fn test_update_budget_spend() {
        let mut manager = BudgetManager::new();
        manager.add_budget(Budget::new("test", 1000.0, BudgetPeriod::Monthly));

        assert!(manager.update_budget_spend("test", 500.0));

        let budget = manager.get_budget("test").unwrap();
        assert_eq!(budget.current_spend, 500.0);
    }

    #[test]
    fn test_exceeded_budgets() {
        let mut manager = BudgetManager::new();

        let mut budget1 = Budget::new("budget1", 1000.0, BudgetPeriod::Monthly);
        budget1.update_spend(1200.0);

        let mut budget2 = Budget::new("budget2", 500.0, BudgetPeriod::Monthly);
        budget2.update_spend(300.0);

        manager.add_budget(budget1);
        manager.add_budget(budget2);

        let exceeded = manager.exceeded_budgets();
        assert_eq!(exceeded.len(), 1);
        assert_eq!(exceeded[0].name, "budget1");
    }

    #[test]
    fn test_cost_forecast_linear() {
        let mut forecast = CostForecast::new(BudgetPeriod::Monthly, 500.0);
        forecast.project_linear(15.0, 30.0); // 15 days elapsed, 30 total

        assert!((forecast.projected_spend - 1000.0).abs() < 0.01);
        assert_eq!(forecast.forecast_method, ForecastMethod::Linear);
    }

    #[test]
    fn test_cost_forecast_weighted() {
        let mut forecast = CostForecast::new(BudgetPeriod::Monthly, 500.0);
        forecast.project_weighted(200.0, 30.0, 7.0); // Recent 7 days spend $200

        let expected = (200.0 / 7.0) * 30.0;
        assert!((forecast.projected_spend - expected).abs() < 0.01);
        assert_eq!(forecast.forecast_method, ForecastMethod::Weighted);
    }

    #[test]
    fn test_forecast_over_budget() {
        let mut forecast = CostForecast::new(BudgetPeriod::Monthly, 500.0);
        forecast.projected_spend = 1200.0;

        assert!(forecast.is_over_budget(1000.0));
        assert!(!forecast.is_over_budget(1500.0));
    }

    #[test]
    fn test_budget_period_display() {
        assert_eq!(BudgetPeriod::Monthly.to_string(), "Monthly");
        assert_eq!(BudgetPeriod::Yearly.to_string(), "Yearly");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Healthy.to_string(), "Healthy");
        assert_eq!(Status::Exceeded.to_string(), "Exceeded");
    }
}
