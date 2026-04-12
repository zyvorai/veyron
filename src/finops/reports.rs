use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Report type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    CostSummary,
    DetailedBreakdown,
    TrendAnalysis,
    ForecastProjection,
    BudgetVariance,
    CostAllocation,
}

/// Report format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    JSON,
    CSV,
    PDF,
    HTML,
}

/// Cost trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrend {
    pub period: String,
    pub total_cost: f64,
    pub change_percent: f64,
    pub breakdown: HashMap<String, f64>,
}

impl CostTrend {
    pub fn new(period: impl Into<String>, total_cost: f64) -> Self {
        Self {
            period: period.into(),
            total_cost,
            change_percent: 0.0,
            breakdown: HashMap::new(),
        }
    }

    pub fn with_change(mut self, change_percent: f64) -> Self {
        self.change_percent = change_percent;
        self
    }

    pub fn add_breakdown(&mut self, category: impl Into<String>, amount: f64) {
        self.breakdown.insert(category.into(), amount);
    }

    pub fn is_increasing(&self) -> bool {
        self.change_percent > 0.0
    }

    pub fn is_significant_change(&self) -> bool {
        self.change_percent.abs() > 10.0
    }
}

/// Cost report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub id: String,
    pub title: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: f64,
    pub currency: String,
    pub cost_by_service: HashMap<String, f64>,
    pub cost_by_region: HashMap<String, f64>,
    pub cost_by_environment: HashMap<String, f64>,
    pub trends: Vec<CostTrend>,
    pub generated_at: DateTime<Utc>,
}

impl CostReport {
    pub fn new(
        title: impl Into<String>,
        report_type: ReportType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        let title_str = title.into();
        let id = format!(
            "report-{}-{}",
            title_str.to_lowercase().replace(' ', "-"),
            Utc::now().timestamp()
        );

        Self {
            id,
            title: title_str,
            report_type,
            format: ReportFormat::JSON,
            period_start: start,
            period_end: end,
            total_cost: 0.0,
            currency: "USD".to_string(),
            cost_by_service: HashMap::new(),
            cost_by_region: HashMap::new(),
            cost_by_environment: HashMap::new(),
            trends: Vec::new(),
            generated_at: Utc::now(),
        }
    }

    pub fn with_format(mut self, format: ReportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_total_cost(mut self, cost: f64) -> Self {
        self.total_cost = cost;
        self
    }

    pub fn add_service_cost(&mut self, service: impl Into<String>, cost: f64) {
        self.cost_by_service.insert(service.into(), cost);
    }

    pub fn add_region_cost(&mut self, region: impl Into<String>, cost: f64) {
        self.cost_by_region.insert(region.into(), cost);
    }

    pub fn add_environment_cost(&mut self, environment: impl Into<String>, cost: f64) {
        self.cost_by_environment.insert(environment.into(), cost);
    }

    pub fn add_trend(&mut self, trend: CostTrend) {
        self.trends.push(trend);
    }

    pub fn service_count(&self) -> usize {
        self.cost_by_service.len()
    }

    pub fn top_service(&self) -> Option<(&str, f64)> {
        self.cost_by_service
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.as_str(), *v))
    }

    pub fn duration_days(&self) -> i64 {
        (self.period_end - self.period_start).num_days()
    }
}

/// Budget variance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetVarianceReport {
    pub id: String,
    pub budget_id: String,
    pub budget_name: String,
    pub budgeted_amount: f64,
    pub actual_spend: f64,
    pub variance: f64,
    pub variance_percent: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
}

impl BudgetVarianceReport {
    pub fn new(
        budget_id: impl Into<String>,
        budget_name: impl Into<String>,
        budgeted: f64,
        actual: f64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        let budget_id_str = budget_id.into();
        let id = format!("variance-{}-{}", budget_id_str, Utc::now().timestamp());

        let variance = actual - budgeted;
        let variance_percent = if budgeted != 0.0 {
            (variance / budgeted) * 100.0
        } else {
            0.0
        };

        Self {
            id,
            budget_id: budget_id_str,
            budget_name: budget_name.into(),
            budgeted_amount: budgeted,
            actual_spend: actual,
            variance,
            variance_percent,
            period_start: start,
            period_end: end,
            generated_at: Utc::now(),
        }
    }

    pub fn is_over_budget(&self) -> bool {
        self.variance > 0.0
    }

    pub fn is_under_budget(&self) -> bool {
        self.variance < 0.0
    }

    pub fn is_significant_variance(&self) -> bool {
        self.variance_percent.abs() > 10.0
    }
}

/// Report manager
pub struct ReportManager {
    reports: HashMap<String, CostReport>,
    variance_reports: HashMap<String, BudgetVarianceReport>,
}

impl ReportManager {
    pub fn new() -> Self {
        Self {
            reports: HashMap::new(),
            variance_reports: HashMap::new(),
        }
    }

    pub fn add_report(&mut self, report: CostReport) -> String {
        let id = report.id.clone();
        self.reports.insert(id.clone(), report);
        id
    }

    pub fn get_report(&self, id: &str) -> Option<&CostReport> {
        self.reports.get(id)
    }

    pub fn report_count(&self) -> usize {
        self.reports.len()
    }

    pub fn add_variance_report(&mut self, report: BudgetVarianceReport) -> String {
        let id = report.id.clone();
        self.variance_reports.insert(id.clone(), report);
        id
    }

    pub fn variance_report_count(&self) -> usize {
        self.variance_reports.len()
    }

    pub fn by_report_type(&self, report_type: &ReportType) -> Vec<&CostReport> {
        self.reports
            .values()
            .filter(|r| &r.report_type == report_type)
            .collect()
    }

    pub fn by_format(&self, format: &ReportFormat) -> Vec<&CostReport> {
        self.reports
            .values()
            .filter(|r| &r.format == format)
            .collect()
    }

    pub fn over_budget_variances(&self) -> Vec<&BudgetVarianceReport> {
        self.variance_reports
            .values()
            .filter(|v| v.is_over_budget())
            .collect()
    }

    pub fn significant_variances(&self) -> Vec<&BudgetVarianceReport> {
        self.variance_reports
            .values()
            .filter(|v| v.is_significant_variance())
            .collect()
    }

    pub fn recent_reports(&self, limit: usize) -> Vec<&CostReport> {
        let mut reports: Vec<_> = self.reports.values().collect();
        reports.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
        reports.truncate(limit);
        reports
    }
}

impl Default for ReportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_trend() {
        let trend = CostTrend::new("2024-01", 5000.0);

        assert_eq!(trend.period, "2024-01");
        assert_eq!(trend.total_cost, 5000.0);
        assert_eq!(trend.change_percent, 0.0);
    }

    #[test]
    fn test_trend_with_change() {
        let trend = CostTrend::new("2024-02", 6000.0).with_change(20.0);

        assert_eq!(trend.change_percent, 20.0);
    }

    #[test]
    fn test_trend_add_breakdown() {
        let mut trend = CostTrend::new("2024-01", 5000.0);

        trend.add_breakdown("compute", 3000.0);
        trend.add_breakdown("storage", 2000.0);

        assert_eq!(trend.breakdown.len(), 2);
        assert_eq!(trend.breakdown.get("compute"), Some(&3000.0));
    }

    #[test]
    fn test_trend_is_increasing() {
        let trend1 = CostTrend::new("2024-01", 5000.0).with_change(15.0);
        assert!(trend1.is_increasing());

        let trend2 = CostTrend::new("2024-02", 4500.0).with_change(-10.0);
        assert!(!trend2.is_increasing());
    }

    #[test]
    fn test_trend_is_significant_change() {
        let trend1 = CostTrend::new("2024-01", 5000.0).with_change(15.0);
        assert!(trend1.is_significant_change());

        let trend2 = CostTrend::new("2024-02", 5000.0).with_change(5.0);
        assert!(!trend2.is_significant_change());
    }

    #[test]
    fn test_cost_report() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report = CostReport::new("Monthly Report", ReportType::CostSummary, start, end);

        assert_eq!(report.title, "Monthly Report");
        assert_eq!(report.report_type, ReportType::CostSummary);
        assert_eq!(report.format, ReportFormat::JSON);
    }

    #[test]
    fn test_report_with_format() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report = CostReport::new("Report", ReportType::DetailedBreakdown, start, end)
            .with_format(ReportFormat::PDF);

        assert_eq!(report.format, ReportFormat::PDF);
    }

    #[test]
    fn test_report_with_total_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report =
            CostReport::new("Report", ReportType::CostSummary, start, end).with_total_cost(15000.0);

        assert_eq!(report.total_cost, 15000.0);
    }

    #[test]
    fn test_report_add_service_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::CostSummary, start, end);

        report.add_service_cost("compute", 5000.0);
        report.add_service_cost("storage", 2000.0);

        assert_eq!(report.cost_by_service.len(), 2);
        assert_eq!(report.cost_by_service.get("compute"), Some(&5000.0));
    }

    #[test]
    fn test_report_add_region_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::CostSummary, start, end);

        report.add_region_cost("us-east-1", 3000.0);
        report.add_region_cost("eu-west-1", 2000.0);

        assert_eq!(report.cost_by_region.len(), 2);
    }

    #[test]
    fn test_report_add_environment_cost() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::CostSummary, start, end);

        report.add_environment_cost("production", 8000.0);
        report.add_environment_cost("staging", 2000.0);

        assert_eq!(report.cost_by_environment.len(), 2);
    }

    #[test]
    fn test_report_add_trend() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::TrendAnalysis, start, end);

        report.add_trend(CostTrend::new("2024-01", 5000.0));
        report.add_trend(CostTrend::new("2024-02", 6000.0));

        assert_eq!(report.trends.len(), 2);
    }

    #[test]
    fn test_report_service_count() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::CostSummary, start, end);

        report.add_service_cost("compute", 5000.0);
        report.add_service_cost("storage", 2000.0);
        report.add_service_cost("network", 1000.0);

        assert_eq!(report.service_count(), 3);
    }

    #[test]
    fn test_report_top_service() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let mut report = CostReport::new("Report", ReportType::CostSummary, start, end);

        report.add_service_cost("compute", 5000.0);
        report.add_service_cost("storage", 8000.0);
        report.add_service_cost("network", 1000.0);

        let top = report.top_service();
        assert_eq!(top, Some(("storage", 8000.0)));
    }

    #[test]
    fn test_report_duration_days() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report = CostReport::new("Report", ReportType::CostSummary, start, end);

        assert_eq!(report.duration_days(), 30);
    }

    #[test]
    fn test_budget_variance_report() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report =
            BudgetVarianceReport::new("budget-123", "Q1 Budget", 10000.0, 11500.0, start, end);

        assert_eq!(report.budget_id, "budget-123");
        assert_eq!(report.budgeted_amount, 10000.0);
        assert_eq!(report.actual_spend, 11500.0);
        assert_eq!(report.variance, 1500.0);
        assert_eq!(report.variance_percent, 15.0);
    }

    #[test]
    fn test_variance_is_over_budget() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report1 = BudgetVarianceReport::new("b1", "Budget 1", 1000.0, 1200.0, start, end);
        assert!(report1.is_over_budget());

        let report2 = BudgetVarianceReport::new("b2", "Budget 2", 1000.0, 800.0, start, end);
        assert!(!report2.is_over_budget());
    }

    #[test]
    fn test_variance_is_under_budget() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report1 = BudgetVarianceReport::new("b1", "Budget 1", 1000.0, 800.0, start, end);
        assert!(report1.is_under_budget());

        let report2 = BudgetVarianceReport::new("b2", "Budget 2", 1000.0, 1200.0, start, end);
        assert!(!report2.is_under_budget());
    }

    #[test]
    fn test_variance_is_significant() {
        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report1 = BudgetVarianceReport::new("b1", "Budget 1", 1000.0, 1150.0, start, end);
        assert!(report1.is_significant_variance()); // 15%

        let report2 = BudgetVarianceReport::new("b2", "Budget 2", 1000.0, 1050.0, start, end);
        assert!(!report2.is_significant_variance()); // 5%
    }

    #[test]
    fn test_report_manager() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report = CostReport::new("Test Report", ReportType::CostSummary, start, end);
        let id = manager.add_report(report);

        assert_eq!(manager.report_count(), 1);
        assert!(manager.get_report(&id).is_some());
    }

    #[test]
    fn test_manager_add_variance_report() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        let report = BudgetVarianceReport::new("budget-1", "Budget", 1000.0, 1200.0, start, end);
        manager.add_variance_report(report);

        assert_eq!(manager.variance_report_count(), 1);
    }

    #[test]
    fn test_manager_by_report_type() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        manager.add_report(CostReport::new("R1", ReportType::CostSummary, start, end));
        manager.add_report(CostReport::new("R2", ReportType::TrendAnalysis, start, end));
        manager.add_report(CostReport::new("R3", ReportType::CostSummary, start, end));

        let summaries = manager.by_report_type(&ReportType::CostSummary);
        assert_eq!(summaries.len(), 2);
    }

    #[test]
    fn test_manager_by_format() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        manager.add_report(
            CostReport::new("R1", ReportType::CostSummary, start, end)
                .with_format(ReportFormat::PDF),
        );
        manager.add_report(CostReport::new("R2", ReportType::CostSummary, start, end));

        let pdfs = manager.by_format(&ReportFormat::PDF);
        assert_eq!(pdfs.len(), 1);
    }

    #[test]
    fn test_manager_over_budget_variances() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        manager.add_variance_report(BudgetVarianceReport::new(
            "b1", "B1", 1000.0, 1200.0, start, end,
        ));
        manager.add_variance_report(BudgetVarianceReport::new(
            "b2", "B2", 1000.0, 800.0, start, end,
        ));

        let over = manager.over_budget_variances();
        assert_eq!(over.len(), 1);
    }

    #[test]
    fn test_manager_significant_variances() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        manager.add_variance_report(BudgetVarianceReport::new(
            "b1", "B1", 1000.0, 1150.0, start, end,
        ));
        manager.add_variance_report(BudgetVarianceReport::new(
            "b2", "B2", 1000.0, 1050.0, start, end,
        ));

        let significant = manager.significant_variances();
        assert_eq!(significant.len(), 1);
    }

    #[test]
    fn test_manager_recent_reports() {
        let mut manager = ReportManager::new();

        let start = Utc::now();
        let end = start + chrono::TimeDelta::days(30);

        manager.add_report(CostReport::new("R1", ReportType::CostSummary, start, end));
        manager.add_report(CostReport::new("R2", ReportType::CostSummary, start, end));
        manager.add_report(CostReport::new("R3", ReportType::CostSummary, start, end));

        let recent = manager.recent_reports(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_report_type_equality() {
        assert_eq!(ReportType::CostSummary, ReportType::CostSummary);
        assert_ne!(ReportType::CostSummary, ReportType::TrendAnalysis);
    }

    #[test]
    fn test_report_format_equality() {
        assert_eq!(ReportFormat::JSON, ReportFormat::JSON);
        assert_ne!(ReportFormat::JSON, ReportFormat::PDF);
    }
}
