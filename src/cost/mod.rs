// Cost Management & Optimization - Track and optimize VM resource costs

use chrono::{DateTime, TimeDelta as Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod budgets;
pub mod optimization;
pub mod reports;
pub mod tracking;

/// Average hours in a month (365.25 days / 12 months * 24 hours)
const HOURS_PER_MONTH: f64 = 730.0;

/// Cost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    pub currency: String,
    pub rates: ResourceRates,
    pub billing_period: BillingPeriod,
}

impl CostConfig {
    pub fn new(currency: impl Into<String>) -> Self {
        Self {
            currency: currency.into(),
            rates: ResourceRates::default(),
            billing_period: BillingPeriod::Monthly,
        }
    }

    pub fn with_rates(mut self, rates: ResourceRates) -> Self {
        self.rates = rates;
        self
    }

    pub fn with_billing_period(mut self, period: BillingPeriod) -> Self {
        self.billing_period = period;
        self
    }
}

impl Default for CostConfig {
    fn default() -> Self {
        Self::new("USD")
    }
}

/// Resource pricing rates.
///
/// These are configurable cost estimates, NOT actual cloud provider rates.
/// The default rates use AWS-like pricing as a baseline for estimation.
/// Use `aws_like()`, `gcp_like()`, or `azure_like()` presets, or set custom
/// rates to match your environment. In production, these should be configured
/// to reflect your actual infrastructure costs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRates {
    pub cpu_per_core_hour: f64,     // Cost per CPU core per hour
    pub memory_per_gb_hour: f64,    // Cost per GB memory per hour
    pub storage_per_gb_month: f64,  // Cost per GB storage per month
    pub network_per_gb: f64,        // Cost per GB network transfer
    pub snapshot_per_gb_month: f64, // Cost per GB snapshot per month
}

impl ResourceRates {
    pub fn aws_like() -> Self {
        Self {
            cpu_per_core_hour: 0.0416,   // ~$30/month per core
            memory_per_gb_hour: 0.0052,  // ~$3.75/month per GB
            storage_per_gb_month: 0.10,  // $0.10 per GB/month
            network_per_gb: 0.09,        // $0.09 per GB transfer
            snapshot_per_gb_month: 0.05, // $0.05 per GB/month
        }
    }

    pub fn gcp_like() -> Self {
        Self {
            cpu_per_core_hour: 0.0330,    // ~$24/month per core
            memory_per_gb_hour: 0.0044,   // ~$3.20/month per GB
            storage_per_gb_month: 0.04,   // $0.04 per GB/month
            network_per_gb: 0.12,         // $0.12 per GB transfer
            snapshot_per_gb_month: 0.026, // $0.026 per GB/month
        }
    }

    pub fn azure_like() -> Self {
        Self {
            cpu_per_core_hour: 0.0380,   // ~$27.50/month per core
            memory_per_gb_hour: 0.0048,  // ~$3.50/month per GB
            storage_per_gb_month: 0.045, // $0.045 per GB/month
            network_per_gb: 0.087,       // $0.087 per GB transfer
            snapshot_per_gb_month: 0.03, // $0.03 per GB/month
        }
    }
}

impl Default for ResourceRates {
    fn default() -> Self {
        Self::aws_like()
    }
}

/// Billing period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BillingPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// VM cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMCost {
    pub vm_name: String,
    pub namespace: String,
    pub labels: HashMap<String, String>,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub storage_cost: f64,
    pub network_cost: f64,
    pub snapshot_cost: f64,
    pub total_cost: f64,
    pub runtime_hours: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl VMCost {
    pub fn new(vm_name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            vm_name: vm_name.into(),
            namespace: namespace.into(),
            labels: HashMap::new(),
            cpu_cost: 0.0,
            memory_cost: 0.0,
            storage_cost: 0.0,
            network_cost: 0.0,
            snapshot_cost: 0.0,
            total_cost: 0.0,
            runtime_hours: 0.0,
            period_start: now - Duration::days(30),
            period_end: now,
        }
    }

    pub fn calculate_total(&mut self) {
        self.total_cost = self.cpu_cost
            + self.memory_cost
            + self.storage_cost
            + self.network_cost
            + self.snapshot_cost;
    }

    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
    }

    pub fn cost_per_hour(&self) -> f64 {
        if self.runtime_hours > 0.0 {
            self.total_cost / self.runtime_hours
        } else {
            0.0
        }
    }

    pub fn period_days(&self) -> i64 {
        self.period_end
            .signed_duration_since(self.period_start)
            .num_days()
    }
}

/// Cost calculator
pub struct CostCalculator {
    config: CostConfig,
}

impl CostCalculator {
    pub fn new(config: CostConfig) -> Self {
        Self { config }
    }

    /// Access the underlying resource rates (useful for display/reverse calculations).
    pub fn rates(&self) -> &ResourceRates {
        &self.config.rates
    }

    /// Calculate VM costs for a period
    pub fn calculate_vm_cost(
        &self,
        vm_name: &str,
        namespace: &str,
        cpu_cores: u32,
        memory_gb: u32,
        storage_gb: u32,
        runtime_hours: f64,
    ) -> VMCost {
        let mut vm_cost = VMCost::new(vm_name, namespace);

        vm_cost.cpu_cost = cpu_cores as f64 * self.config.rates.cpu_per_core_hour * runtime_hours;
        vm_cost.memory_cost =
            memory_gb as f64 * self.config.rates.memory_per_gb_hour * runtime_hours;

        // Storage is charged monthly, convert to hours
        let storage_hours = runtime_hours / HOURS_PER_MONTH;
        vm_cost.storage_cost =
            storage_gb as f64 * self.config.rates.storage_per_gb_month * storage_hours;

        vm_cost.runtime_hours = runtime_hours;
        vm_cost.calculate_total();

        vm_cost
    }

    /// Calculate network transfer costs
    pub fn calculate_network_cost(&self, gb_transferred: f64) -> f64 {
        gb_transferred * self.config.rates.network_per_gb
    }

    /// Calculate snapshot storage costs
    pub fn calculate_snapshot_cost(&self, snapshot_gb: u32, months: f64) -> f64 {
        snapshot_gb as f64 * self.config.rates.snapshot_per_gb_month * months
    }

    /// Estimate monthly cost for VM
    pub fn estimate_monthly_cost(&self, cpu_cores: u32, memory_gb: u32, storage_gb: u32) -> f64 {
        let monthly_hours = HOURS_PER_MONTH;

        let cpu_cost = cpu_cores as f64 * self.config.rates.cpu_per_core_hour * monthly_hours;
        let memory_cost = memory_gb as f64 * self.config.rates.memory_per_gb_hour * monthly_hours;
        let storage_cost = storage_gb as f64 * self.config.rates.storage_per_gb_month;

        cpu_cost + memory_cost + storage_cost
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new(CostConfig::default())
    }
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost: f64,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub storage_cost: f64,
    pub network_cost: f64,
    pub snapshot_cost: f64,
    pub vm_count: usize,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl CostSummary {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            total_cost: 0.0,
            cpu_cost: 0.0,
            memory_cost: 0.0,
            storage_cost: 0.0,
            network_cost: 0.0,
            snapshot_cost: 0.0,
            vm_count: 0,
            period_start: now - Duration::days(30),
            period_end: now,
        }
    }

    pub fn add_vm_cost(&mut self, vm_cost: &VMCost) {
        self.total_cost += vm_cost.total_cost;
        self.cpu_cost += vm_cost.cpu_cost;
        self.memory_cost += vm_cost.memory_cost;
        self.storage_cost += vm_cost.storage_cost;
        self.network_cost += vm_cost.network_cost;
        self.snapshot_cost += vm_cost.snapshot_cost;
        self.vm_count += 1;
    }

    pub fn average_cost_per_vm(&self) -> f64 {
        if self.vm_count > 0 {
            self.total_cost / self.vm_count as f64
        } else {
            0.0
        }
    }
}

impl Default for CostSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_config() {
        let config = CostConfig::new("USD")
            .with_rates(ResourceRates::aws_like())
            .with_billing_period(BillingPeriod::Monthly);

        assert_eq!(config.currency, "USD");
        assert_eq!(config.billing_period, BillingPeriod::Monthly);
    }

    #[test]
    fn test_resource_rates() {
        let aws_rates = ResourceRates::aws_like();
        assert_eq!(aws_rates.cpu_per_core_hour, 0.0416);

        let gcp_rates = ResourceRates::gcp_like();
        assert_eq!(gcp_rates.cpu_per_core_hour, 0.0330);

        let azure_rates = ResourceRates::azure_like();
        assert_eq!(azure_rates.cpu_per_core_hour, 0.0380);
    }

    #[test]
    fn test_vm_cost() {
        let mut vm_cost = VMCost::new("test-vm", "default");
        vm_cost.cpu_cost = 10.0;
        vm_cost.memory_cost = 5.0;
        vm_cost.storage_cost = 2.0;
        vm_cost.calculate_total();

        assert_eq!(vm_cost.total_cost, 17.0);
    }

    #[test]
    fn test_cost_calculator() {
        let calculator = CostCalculator::default();

        // Calculate cost for 2 cores, 4GB RAM, 20GB storage, 730 hours (1 month)
        let vm_cost = calculator.calculate_vm_cost("test-vm", "default", 2, 4, 20, 730.0);

        assert!(vm_cost.total_cost > 0.0);
        assert_eq!(vm_cost.runtime_hours, 730.0);
    }

    #[test]
    fn test_monthly_estimate() {
        let calculator = CostCalculator::default();

        // 2 cores, 4GB RAM, 20GB storage
        let monthly_cost = calculator.estimate_monthly_cost(2, 4, 20);

        assert!(monthly_cost > 0.0);
        // Should be around $60-70 with default AWS-like rates
        assert!(monthly_cost > 50.0 && monthly_cost < 100.0);
    }

    #[test]
    fn test_network_cost() {
        let calculator = CostCalculator::default();
        let network_cost = calculator.calculate_network_cost(100.0); // 100 GB

        assert_eq!(network_cost, 9.0); // 100 * 0.09
    }

    #[test]
    fn test_snapshot_cost() {
        let calculator = CostCalculator::default();
        let snapshot_cost = calculator.calculate_snapshot_cost(50, 1.0); // 50GB for 1 month

        assert_eq!(snapshot_cost, 2.5); // 50 * 0.05
    }

    #[test]
    fn test_cost_summary() {
        let mut summary = CostSummary::new();

        let mut vm1 = VMCost::new("vm1", "default");
        vm1.total_cost = 50.0;
        vm1.cpu_cost = 30.0;

        let mut vm2 = VMCost::new("vm2", "default");
        vm2.total_cost = 30.0;
        vm2.cpu_cost = 20.0;

        summary.add_vm_cost(&vm1);
        summary.add_vm_cost(&vm2);

        assert_eq!(summary.total_cost, 80.0);
        assert_eq!(summary.cpu_cost, 50.0);
        assert_eq!(summary.vm_count, 2);
        assert_eq!(summary.average_cost_per_vm(), 40.0);
    }

    #[test]
    fn test_cost_per_hour() {
        let mut vm_cost = VMCost::new("test-vm", "default");
        vm_cost.total_cost = 100.0;
        vm_cost.runtime_hours = 50.0;

        assert_eq!(vm_cost.cost_per_hour(), 2.0);
    }

    #[test]
    fn test_vm_labels() {
        let mut vm_cost = VMCost::new("test-vm", "default");
        vm_cost.add_label("team", "engineering");
        vm_cost.add_label("env", "production");

        assert_eq!(vm_cost.labels.get("team"), Some(&"engineering".to_string()));
        assert_eq!(vm_cost.labels.get("env"), Some(&"production".to_string()));
    }
}
