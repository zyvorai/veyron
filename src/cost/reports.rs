// Cost Reports - Generate cost reports and analytics

use super::{CostSummary, VMCost};
use chrono::{DateTime, TimeDelta as Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cost report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub report_id: String,
    pub report_type: ReportType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: CostSummary,
    pub vm_costs: Vec<VMCost>,
    pub breakdowns: Vec<CostBreakdown>,
    pub generated_at: DateTime<Utc>,
}

impl CostReport {
    pub fn new(
        report_type: ReportType,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Self {
        let report_id = format!("report-{}", Utc::now().format("%Y%m%d-%H%M%S"));

        Self {
            report_id,
            report_type,
            period_start,
            period_end,
            summary: CostSummary::default(),
            vm_costs: Vec::new(),
            breakdowns: Vec::new(),
            generated_at: Utc::now(),
        }
    }

    pub fn add_vm_cost(&mut self, vm_cost: VMCost) {
        self.summary.add_vm_cost(&vm_cost);
        self.vm_costs.push(vm_cost);
    }

    pub fn add_breakdown(&mut self, breakdown: CostBreakdown) {
        self.breakdowns.push(breakdown);
    }

    pub fn top_vms(&self, n: usize) -> Vec<&VMCost> {
        let mut vms = self.vm_costs.iter().collect::<Vec<_>>();
        vms.sort_by(|a, b| {
            b.total_cost
                .partial_cmp(&a.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        vms.truncate(n);
        vms
    }

    pub fn costs_by_namespace(&self) -> HashMap<String, f64> {
        let mut costs = HashMap::new();

        for vm_cost in &self.vm_costs {
            *costs.entry(vm_cost.namespace.clone()).or_insert(0.0) += vm_cost.total_cost;
        }

        costs
    }

    pub fn average_cost_per_vm(&self) -> f64 {
        self.summary.average_cost_per_vm()
    }
}

/// Report type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Custom,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportType::Daily => write!(f, "Daily"),
            ReportType::Weekly => write!(f, "Weekly"),
            ReportType::Monthly => write!(f, "Monthly"),
            ReportType::Quarterly => write!(f, "Quarterly"),
            ReportType::Yearly => write!(f, "Yearly"),
            ReportType::Custom => write!(f, "Custom"),
        }
    }
}

/// Cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub dimension: String, // "namespace", "team", "project", "region", etc.
    pub values: HashMap<String, f64>,
}

impl CostBreakdown {
    pub fn new(dimension: impl Into<String>) -> Self {
        Self {
            dimension: dimension.into(),
            values: HashMap::new(),
        }
    }

    pub fn add_value(&mut self, key: impl Into<String>, value: f64) {
        *self.values.entry(key.into()).or_insert(0.0) += value;
    }

    pub fn get_sorted_values(&self) -> Vec<(String, f64)> {
        let mut values: Vec<_> = self.values.iter().map(|(k, v)| (k.clone(), *v)).collect();
        values.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        values
    }
}

/// Cost comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostComparison {
    pub current_period: String,
    pub previous_period: String,
    pub current_cost: f64,
    pub previous_cost: f64,
    pub change_amount: f64,
    pub change_percent: f64,
}

impl CostComparison {
    pub fn new(
        current_period: impl Into<String>,
        previous_period: impl Into<String>,
        current_cost: f64,
        previous_cost: f64,
    ) -> Self {
        let change_amount = current_cost - previous_cost;
        let change_percent = if previous_cost > 0.0 {
            (change_amount / previous_cost) * 100.0
        } else {
            0.0
        };

        Self {
            current_period: current_period.into(),
            previous_period: previous_period.into(),
            current_cost,
            previous_cost,
            change_amount,
            change_percent,
        }
    }

    pub fn is_increasing(&self) -> bool {
        self.change_amount > 0.0
    }

    pub fn is_decreasing(&self) -> bool {
        self.change_amount < 0.0
    }
}

/// Cost analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalytics {
    pub total_spend: f64,
    pub average_daily_spend: f64,
    pub highest_day_spend: f64,
    pub lowest_day_spend: f64,
    pub cost_trend: f64, // Percent change from previous period
    pub top_cost_drivers: Vec<CostDriver>,
}

impl CostAnalytics {
    pub fn new() -> Self {
        Self {
            total_spend: 0.0,
            average_daily_spend: 0.0,
            highest_day_spend: 0.0,
            lowest_day_spend: 0.0,
            cost_trend: 0.0,
            top_cost_drivers: Vec::new(),
        }
    }

    pub fn from_costs(daily_costs: &[f64], previous_period_total: f64) -> Self {
        let total_spend: f64 = daily_costs.iter().sum();
        let average_daily_spend = if !daily_costs.is_empty() {
            total_spend / daily_costs.len() as f64
        } else {
            0.0
        };

        let highest_day_spend = daily_costs
            .iter()
            .fold(0.0, |max, &cost| if cost > max { cost } else { max });
        let lowest_day_spend = if daily_costs.is_empty() {
            0.0
        } else {
            daily_costs
                .iter()
                .fold(f64::MAX, |min, &cost| if cost < min { cost } else { min })
        };

        let cost_trend = if previous_period_total > 0.0 {
            ((total_spend - previous_period_total) / previous_period_total) * 100.0
        } else {
            0.0
        };

        Self {
            total_spend,
            average_daily_spend,
            highest_day_spend,
            lowest_day_spend,
            cost_trend,
            top_cost_drivers: Vec::new(),
        }
    }

    pub fn add_cost_driver(&mut self, driver: CostDriver) {
        self.top_cost_drivers.push(driver);
    }
}

impl Default for CostAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDriver {
    pub name: String,
    pub cost: f64,
    pub percentage: f64,
}

impl CostDriver {
    pub fn new(name: impl Into<String>, cost: f64, total_cost: f64) -> Self {
        let percentage = if total_cost > 0.0 {
            (cost / total_cost) * 100.0
        } else {
            0.0
        };

        Self {
            name: name.into(),
            cost,
            percentage,
        }
    }
}

/// Report generator
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate monthly cost report
    pub fn monthly_report(year: i32, month: u32) -> CostReport {
        let clamped_month = month.clamp(1, 12);
        // SAFETY: clamped_month is 1-12 and day is 1, so only an extreme year value
        // (outside ~262000 BCE to 262000 CE) could fail. Fall back to Unix epoch.
        let fallback = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let start_date =
            chrono::NaiveDate::from_ymd_opt(year, clamped_month, 1).unwrap_or(fallback);
        // SAFETY: (0,0,0) is always a valid time
        let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

        let end_date = if clamped_month == 12 {
            chrono::NaiveDate::from_ymd_opt(year.saturating_add(1), 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, clamped_month + 1, 1)
        }
        .unwrap_or(start_date + chrono::TimeDelta::days(30));
        // SAFETY: (0,0,0) is always a valid time
        let end = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

        CostReport::new(ReportType::Monthly, start, end)
    }

    /// Generate weekly report
    pub fn weekly_report(week_start: DateTime<Utc>) -> CostReport {
        let week_end = week_start + Duration::days(7);
        CostReport::new(ReportType::Weekly, week_start, week_end)
    }

    /// Generate custom period report
    pub fn custom_report(start: DateTime<Utc>, end: DateTime<Utc>) -> CostReport {
        CostReport::new(ReportType::Custom, start, end)
    }

    /// Create comparison report
    pub fn comparison_report(
        current_report: &CostReport,
        previous_report: &CostReport,
    ) -> CostComparison {
        CostComparison::new(
            current_report.report_type.to_string(),
            previous_report.report_type.to_string(),
            current_report.summary.total_cost,
            previous_report.summary.total_cost,
        )
    }
}

/// Export format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    JSON,
    CSV,
    PDF,
    Excel,
}

/// Report exporter
/// Escape a field for CSV output: quote if it contains commas, quotes, or newlines.
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub struct ReportExporter;

impl ReportExporter {
    /// Export report to JSON
    pub fn to_json(report: &CostReport) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(report)
    }

    /// Export to CSV format with proper field escaping
    pub fn to_csv(report: &CostReport) -> String {
        let mut csv = String::from(
            "VM Name,Namespace,CPU Cost,Memory Cost,Storage Cost,Network Cost,Total Cost\n",
        );

        for vm_cost in &report.vm_costs {
            csv.push_str(&format!(
                "{},{},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
                csv_escape(&vm_cost.vm_name),
                csv_escape(&vm_cost.namespace),
                vm_cost.cpu_cost,
                vm_cost.memory_cost,
                vm_cost.storage_cost,
                vm_cost.network_cost,
                vm_cost.total_cost
            ));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_cost_report() {
        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();
        let mut report = CostReport::new(ReportType::Monthly, start, end);

        let mut vm_cost = VMCost::new("test-vm", "default");
        vm_cost.total_cost = 100.0;

        report.add_vm_cost(vm_cost);

        assert_eq!(report.vm_costs.len(), 1);
        assert_eq!(report.summary.total_cost, 100.0);
    }

    #[test]
    fn test_top_vms() {
        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();
        let mut report = CostReport::new(ReportType::Monthly, start, end);

        let mut vm1 = VMCost::new("vm1", "default");
        vm1.total_cost = 100.0;

        let mut vm2 = VMCost::new("vm2", "default");
        vm2.total_cost = 150.0;

        let mut vm3 = VMCost::new("vm3", "default");
        vm3.total_cost = 75.0;

        report.add_vm_cost(vm1);
        report.add_vm_cost(vm2);
        report.add_vm_cost(vm3);

        let top = report.top_vms(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].vm_name, "vm2");
        assert_eq!(top[1].vm_name, "vm1");
    }

    #[test]
    fn test_costs_by_namespace() {
        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();
        let mut report = CostReport::new(ReportType::Monthly, start, end);

        let mut vm1 = VMCost::new("vm1", "default");
        vm1.total_cost = 100.0;

        let mut vm2 = VMCost::new("vm2", "default");
        vm2.total_cost = 50.0;

        let mut vm3 = VMCost::new("vm3", "prod");
        vm3.total_cost = 75.0;

        report.add_vm_cost(vm1);
        report.add_vm_cost(vm2);
        report.add_vm_cost(vm3);

        let costs = report.costs_by_namespace();
        assert_eq!(costs.get("default"), Some(&150.0));
        assert_eq!(costs.get("prod"), Some(&75.0));
    }

    #[test]
    fn test_cost_breakdown() {
        let mut breakdown = CostBreakdown::new("team");

        breakdown.add_value("engineering", 100.0);
        breakdown.add_value("engineering", 50.0);
        breakdown.add_value("sales", 30.0);

        assert_eq!(breakdown.values.get("engineering"), Some(&150.0));
        assert_eq!(breakdown.values.get("sales"), Some(&30.0));

        let sorted = breakdown.get_sorted_values();
        assert_eq!(sorted[0].0, "engineering");
        assert_eq!(sorted[1].0, "sales");
    }

    #[test]
    fn test_cost_comparison() {
        let comparison = CostComparison::new("Current Month", "Previous Month", 1200.0, 1000.0);

        assert_eq!(comparison.change_amount, 200.0);
        assert_eq!(comparison.change_percent, 20.0);
        assert!(comparison.is_increasing());
        assert!(!comparison.is_decreasing());
    }

    #[test]
    fn test_cost_analytics() {
        let daily_costs = vec![100.0, 150.0, 120.0, 180.0, 110.0];
        let analytics = CostAnalytics::from_costs(&daily_costs, 600.0);

        assert_eq!(analytics.total_spend, 660.0);
        assert_eq!(analytics.average_daily_spend, 132.0);
        assert_eq!(analytics.highest_day_spend, 180.0);
        assert_eq!(analytics.lowest_day_spend, 100.0);
        assert_eq!(analytics.cost_trend, 10.0); // 10% increase from 600
    }

    #[test]
    fn test_cost_driver() {
        let driver = CostDriver::new("VM Compute", 500.0, 1000.0);

        assert_eq!(driver.cost, 500.0);
        assert_eq!(driver.percentage, 50.0);
    }

    #[test]
    fn test_report_generator() {
        let report = ReportGenerator::monthly_report(2024, 1);

        assert_eq!(report.report_type, ReportType::Monthly);
        assert_eq!(report.period_start.month(), 1);
    }

    #[test]
    fn test_weekly_report() {
        let start = Utc::now();
        let report = ReportGenerator::weekly_report(start);

        assert_eq!(report.report_type, ReportType::Weekly);

        let duration = report.period_end.signed_duration_since(report.period_start);
        assert_eq!(duration.num_days(), 7);
    }

    #[test]
    fn test_comparison_report() {
        let start = Utc::now() - Duration::days(60);
        let mid = Utc::now() - Duration::days(30);
        let end = Utc::now();

        let mut prev_report = CostReport::new(ReportType::Monthly, start, mid);
        prev_report.summary.total_cost = 1000.0;

        let mut current_report = CostReport::new(ReportType::Monthly, mid, end);
        current_report.summary.total_cost = 1200.0;

        let comparison = ReportGenerator::comparison_report(&current_report, &prev_report);

        assert!(comparison.is_increasing());
        assert_eq!(comparison.change_percent, 20.0);
    }

    #[test]
    fn test_json_export() {
        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();
        let report = CostReport::new(ReportType::Monthly, start, end);

        let json = ReportExporter::to_json(&report);
        assert!(json.is_ok());
    }

    #[test]
    fn test_csv_export() {
        let start = Utc::now() - Duration::days(30);
        let end = Utc::now();
        let mut report = CostReport::new(ReportType::Monthly, start, end);

        let mut vm_cost = VMCost::new("test-vm", "default");
        vm_cost.total_cost = 100.0;
        vm_cost.cpu_cost = 60.0;

        report.add_vm_cost(vm_cost);

        let csv = ReportExporter::to_csv(&report);
        assert!(csv.contains("VM Name,Namespace"));
        assert!(csv.contains("test-vm,default"));
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Monthly.to_string(), "Monthly");
        assert_eq!(ReportType::Yearly.to_string(), "Yearly");
    }
}
